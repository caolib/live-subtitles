// Live Subtitles - 实时字幕应用
// 基于 Tauri v2 + sherpa-rs

#[cfg(not(target_os = "windows"))]
mod audio;
#[cfg(target_os = "windows")]
mod audio_wasapi;
mod config;
mod online_asr;

#[cfg(not(target_os = "windows"))]
use audio::AudioCapture;
#[cfg(target_os = "windows")]
use audio_wasapi::AudioCapture;
use config::AppConfig;
use online_asr::{OnlineRecognizer, OnlineRecognizerConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{Emitter, Manager, State};

/// 应用状态
pub struct AppState {
    /// 配置
    config: Mutex<AppConfig>,
    /// 音频捕获 (运行时创建)
    audio_capture: Mutex<Option<AudioCapture>>,
    /// 是否正在识别
    is_running: Mutex<bool>,
    /// 模型目录
    models_dir: PathBuf,
}

impl AppState {
    fn new(models_dir: PathBuf) -> Self {
        Self {
            config: Mutex::new(AppConfig::default()),
            audio_capture: Mutex::new(None),
            is_running: Mutex::new(false),
            models_dir,
        }
    }
}

/// 发送给前端的字幕事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleEvent {
    /// 识别的文本
    pub text: String,
    /// 是否是句子结束 (endpoint)
    pub is_final: bool,
    /// 时间戳 (毫秒)
    pub timestamp: u64,
}

impl SubtitleEvent {
    fn new(text: String, is_final: bool) -> Self {
        Self {
            text,
            is_final,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

/// 获取可用的模型列表
#[tauri::command]
async fn get_available_models(_state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    // TODO: 扫描 models 目录，返回可用的模型列表
    // 目前只返回默认的 zipformer-small-bilingual
    Ok(vec!["zipformer-small-bilingual".to_string()])
}

/// 获取当前配置
#[tauri::command]
async fn get_config(state: State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// 更新配置
#[tauri::command]
async fn update_config(state: State<'_, Arc<AppState>>, config: AppConfig) -> Result<(), String> {
    let mut current_config = state.config.lock().map_err(|e| e.to_string())?;
    *current_config = config;
    Ok(())
}

/// 获取识别状态
#[tauri::command]
async fn is_recognition_running(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let is_running = state.is_running.lock().map_err(|e| e.to_string())?;
    Ok(*is_running)
}

/// 开始识别
#[tauri::command]
async fn start_recognition(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // 检查是否已经在运行
    {
        let is_running = state.is_running.lock().map_err(|e| e.to_string())?;
        if *is_running {
            return Err("Recognition is already running".to_string());
        }
    }

    // 获取配置
    let config = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.clone()
    };

    // 获取当前模型配置
    let asr_config = config
        .current_model()
        .ok_or_else(|| "No ASR model configured".to_string())?
        .clone();

    // 创建音频捕获
    let mut audio_capture = AudioCapture::new(asr_config.sample_rate);
    let audio_rx = audio_capture
        .start()
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // 保存音频捕获实例
    {
        let mut audio = state.audio_capture.lock().map_err(|e| e.to_string())?;
        *audio = Some(audio_capture);
    }

    // 标记为运行中
    {
        let mut is_running = state.is_running.lock().map_err(|e| e.to_string())?;
        *is_running = true;
    }

    // 在后台线程中运行识别
    let models_dir = state.models_dir.clone();
    let state_clone = Arc::clone(&state.inner());

    thread::spawn(move || {
        // 构建 OnlineRecognizer 配置
        let (encoder, decoder, joiner) = match &asr_config.model_type {
            config::AsrModelType::Transducer {
                encoder,
                decoder,
                joiner,
            } => (
                models_dir.join(encoder).to_string_lossy().to_string(),
                models_dir.join(decoder).to_string_lossy().to_string(),
                models_dir.join(joiner).to_string_lossy().to_string(),
            ),
            _ => {
                eprintln!("OnlineRecognizer only supports Transducer models");
                let _ =
                    app_handle.emit("recognition_error", "Only Transducer models are supported");
                return;
            }
        };

        let online_config = OnlineRecognizerConfig {
            encoder,
            decoder,
            joiner,
            tokens: models_dir
                .join(&asr_config.tokens)
                .to_string_lossy()
                .to_string(),
            sample_rate: asr_config.sample_rate as i32,
            feature_dim: 80,
            num_threads: asr_config.num_threads,
            enable_endpoint: true,
            rule1_min_trailing_silence: 2.4,  // 句子结束静音
            rule2_min_trailing_silence: 1.2,  // 中间停顿静音
            rule3_min_utterance_length: 20.0, // 最小语句长度
            decoding_method: "greedy_search".to_string(),
            debug: true,
        };

        // 创建 OnlineRecognizer
        match OnlineRecognizer::new(online_config) {
            Ok(recognizer) => {
                let mut last_text = String::new();

                // 循环处理音频
                while let Ok(samples) = audio_rx.recv() {
                    // 检查是否仍在运行
                    if let Ok(is_running) = state_clone.is_running.lock() {
                        if !*is_running {
                            break;
                        }
                    }

                    // 处理音频
                    let (text, is_endpoint) = recognizer.process(&samples);

                    // 如果有新文本，发送更新
                    if !text.is_empty() && text != last_text {
                        // 中间结果，不是最终的
                        let event = SubtitleEvent::new(text.clone(), false);
                        let _ = app_handle.emit("subtitle", &event);
                        last_text = text.clone();
                    }

                    // 如果到达 endpoint，发送最终结果并重置流
                    if is_endpoint && !last_text.is_empty() {
                        // 发送最终结果
                        let event = SubtitleEvent::new(last_text.clone(), true);
                        let _ = app_handle.emit("subtitle", &event);

                        recognizer.reset();
                        last_text.clear();
                    } else if is_endpoint {
                        // 没有文本但检测到 endpoint，只重置
                        recognizer.reset();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to create OnlineRecognizer: {}", e);
                let _ = app_handle.emit("recognition_error", &e);
            }
        }

        // 清理状态
        if let Ok(mut is_running) = state_clone.is_running.lock() {
            *is_running = false;
        }
    });

    Ok(())
}

/// 停止识别
#[tauri::command]
async fn stop_recognition(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    // 标记为停止
    {
        let mut is_running = state.is_running.lock().map_err(|e| e.to_string())?;
        *is_running = false;
    }

    // 停止音频捕获
    {
        let mut audio = state.audio_capture.lock().map_err(|e| e.to_string())?;
        if let Some(mut capture) = audio.take() {
            capture.stop();
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 使用 src-tauri/models 目录
            let models_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models");

            // 创建应用状态
            let state = Arc::new(AppState::new(models_dir));
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_available_models,
            get_config,
            update_config,
            is_recognition_running,
            start_recognition,
            stop_recognition,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
