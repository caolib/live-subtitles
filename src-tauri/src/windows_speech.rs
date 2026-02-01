//! Windows 内置语音识别模块
//!
//! 使用 Windows.Media.SpeechRecognition API 实现语音转文本
//! 这是 Windows 11 的 Win+H 语音输入背后使用的同一套 API
//!
//! 优点：
//! - 免费使用
//! - 无需下载模型
//! - 识别质量高（使用微软云服务）
//! - 支持多语言（包括中文）
//!
//! 注意：
//! - 需要用户在系统设置中启用在线语音识别
//! - 需要网络连接（使用预定义 Dictation 语法时）

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use windows::core::{Result as WinResult, HSTRING};
use windows::Foundation::TypedEventHandler;
use windows::Globalization::Language;
use windows::Media::SpeechRecognition::{
    SpeechContinuousRecognitionResultGeneratedEventArgs,
    SpeechContinuousRecognitionSession, SpeechRecognitionScenario,
    SpeechRecognitionTopicConstraint, SpeechRecognizer, SpeechRecognizerState,
    SpeechRecognizerStateChangedEventArgs, SpeechRecognitionHypothesisGeneratedEventArgs,
};

/// Windows 语音识别事件
#[derive(Debug, Clone)]
pub enum WindowsSpeechEvent {
    /// 中间识别结果（假设）
    Hypothesis(String),
    /// 最终识别结果
    Result(String),
    /// 识别错误
    #[allow(dead_code)]
    Error(String),
    /// 状态变化
    StateChanged(String),
}

/// Windows 语音识别器配置
#[derive(Debug, Clone)]
pub struct WindowsSpeechConfig {
    /// 识别语言 (如 "zh-CN", "en-US")
    pub language: String,
    /// 识别场景
    pub scenario: WindowsSpeechScenario,
}

/// 识别场景
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum WindowsSpeechScenario {
    /// 听写（自由形式的语音输入）- 需要网络
    Dictation,
    /// 网页搜索 - 需要网络
    WebSearch,
    /// 表单填写
    FormFilling,
}

impl Default for WindowsSpeechConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            scenario: WindowsSpeechScenario::Dictation,
        }
    }
}

/// Windows 语音识别器封装
pub struct WindowsSpeechRecognizer {
    recognizer: SpeechRecognizer,
    #[allow(dead_code)]
    event_rx: Receiver<WindowsSpeechEvent>,
    is_running: Arc<Mutex<bool>>,
    _event_tx: Sender<WindowsSpeechEvent>, // 保持 sender 存活
}

impl WindowsSpeechRecognizer {
    /// 创建新的 Windows 语音识别器
    pub fn new(config: WindowsSpeechConfig) -> WinResult<Self> {
        // 创建语言对象
        let language = Language::CreateLanguage(&HSTRING::from(&config.language))?;

        // 创建语音识别器 (使用 Create 而不是 CreateWithLanguage)
        let recognizer = SpeechRecognizer::Create(&language)?;

        // 添加识别约束（使用 Dictation 场景获得最佳自由文本识别）
        let scenario = match config.scenario {
            WindowsSpeechScenario::Dictation => SpeechRecognitionScenario::Dictation,
            WindowsSpeechScenario::WebSearch => SpeechRecognitionScenario::WebSearch,
            WindowsSpeechScenario::FormFilling => SpeechRecognitionScenario::FormFilling,
        };

        let constraint =
            SpeechRecognitionTopicConstraint::Create(scenario, &HSTRING::from("Default"))?;
        recognizer.Constraints()?.Append(&constraint)?;

        // 创建事件通道
        let (event_tx, event_rx) = mpsc::channel();
        let is_running = Arc::new(Mutex::new(false));

        Ok(Self {
            recognizer,
            event_rx,
            is_running,
            _event_tx: event_tx,
        })
    }

    /// 编译语法约束
    pub fn compile_constraints(&self) -> WinResult<bool> {
        // 使用阻塞方式编译
        let compile_result = self.recognizer.CompileConstraintsAsync()?.get()?;
        let status = compile_result.Status()?;
        Ok(status == windows::Media::SpeechRecognition::SpeechRecognitionResultStatus::Success)
    }

    /// 开始连续识别
    pub fn start_continuous(&self) -> WinResult<Receiver<WindowsSpeechEvent>> {
        // 创建新的事件通道用于返回
        let (tx, rx) = mpsc::channel::<WindowsSpeechEvent>();

        // 获取连续识别会话
        let session = self.recognizer.ContinuousRecognitionSession()?;

        // 设置结果生成事件处理器
        let tx_result = tx.clone();
        session.ResultGenerated(&TypedEventHandler::<
            SpeechContinuousRecognitionSession,
            SpeechContinuousRecognitionResultGeneratedEventArgs,
        >::new(
            move |_session, args| {
                if let Some(args) = args {
                    if let Ok(result) = args.Result() {
                        if let Ok(text) = result.Text() {
                            let text_str = text.to_string();
                            if !text_str.is_empty() {
                                let _ = tx_result.send(WindowsSpeechEvent::Result(text_str));
                            }
                        }
                    }
                }
                Ok(())
            },
        ))?;

        // 设置假设事件处理器（中间结果）
        let tx_hypothesis = tx.clone();
        self.recognizer.HypothesisGenerated(&TypedEventHandler::<
            SpeechRecognizer,
            SpeechRecognitionHypothesisGeneratedEventArgs,
        >::new(
            move |_recognizer, args| {
                if let Some(args) = args {
                    if let Ok(hypothesis) = args.Hypothesis() {
                        if let Ok(text) = hypothesis.Text() {
                            let text_str = text.to_string();
                            if !text_str.is_empty() {
                                let _ = tx_hypothesis.send(WindowsSpeechEvent::Hypothesis(text_str));
                            }
                        }
                    }
                }
                Ok(())
            },
        ))?;

        // 设置状态变化事件
        let tx_state = tx.clone();
        self.recognizer.StateChanged(&TypedEventHandler::<
            SpeechRecognizer,
            SpeechRecognizerStateChangedEventArgs,
        >::new(
            move |_recognizer, args| {
                if let Some(args) = args {
                    if let Ok(state) = args.State() {
                        let state_str = format!("{:?}", state);
                        let _ = tx_state.send(WindowsSpeechEvent::StateChanged(state_str));
                    }
                }
                Ok(())
            },
        ))?;

        // 开始连续识别
        session
            .StartAsync()?
            .get()?;

        // 标记为运行中
        if let Ok(mut running) = self.is_running.lock() {
            *running = true;
        }

        Ok(rx)
    }

    /// 停止连续识别
    pub fn stop_continuous(&self) -> WinResult<()> {
        // 标记为停止
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }

        // 停止会话
        let session = self.recognizer.ContinuousRecognitionSession()?;
        session.StopAsync()?.get()?;

        Ok(())
    }

    /// 检查是否正在运行
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.is_running
            .lock()
            .map(|r| *r)
            .unwrap_or(false)
    }

    /// 获取当前状态
    #[allow(dead_code)]
    pub fn state(&self) -> WinResult<SpeechRecognizerState> {
        self.recognizer.State()
    }
}

impl Drop for WindowsSpeechRecognizer {
    fn drop(&mut self) {
        // 确保停止识别
        let _ = self.stop_continuous();
    }
}

/// 检查 Windows 语音识别是否可用
pub fn is_windows_speech_available() -> bool {
    // 尝试创建一个默认的识别器来检测是否可用
    SpeechRecognizer::new().is_ok()
}

/// 检查在线语音识别是否启用
/// 用户需要在 设置 -> 隐私 -> 语音 中启用此功能
pub fn check_online_speech_permission() -> bool {
    // 实际检测需要尝试编译带有在线约束的识别器
    // 如果用户未启用在线语音，编译会失败
    if let Ok(recognizer) = SpeechRecognizer::new() {
        if let Ok(constraints) = recognizer.Constraints() {
            if let Ok(constraint) = SpeechRecognitionTopicConstraint::Create(
                SpeechRecognitionScenario::Dictation,
                &HSTRING::from("test"),
            ) {
                if constraints.Append(&constraint).is_ok() {
                    if let Ok(compile_async) = recognizer.CompileConstraintsAsync() {
                        if let Ok(result) = compile_async.get() {
                            if let Ok(status) = result.Status() {
                                return status
                                    == windows::Media::SpeechRecognition::SpeechRecognitionResultStatus::Success;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// 获取支持的语言列表
pub fn get_supported_languages() -> Vec<String> {
    // 使用静态方法获取所有支持的语言
    if let Ok(languages) = SpeechRecognizer::SupportedTopicLanguages() {
        let mut result = Vec::new();
        if let Ok(size) = languages.Size() {
            for i in 0..size {
                if let Ok(lang) = languages.GetAt(i) {
                    if let Ok(tag) = lang.LanguageTag() {
                        result.push(tag.to_string());
                    }
                }
            }
        }
        return result;
    }
    vec!["zh-CN".to_string(), "en-US".to_string()] // 默认返回常用语言
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_availability() {
        println!("Windows Speech available: {}", is_windows_speech_available());
        println!("Online speech enabled: {}", check_online_speech_permission());
        println!("Supported languages: {:?}", get_supported_languages());
    }
}
