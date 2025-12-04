//! OnlineRecognizer (流式识别器) 封装
//! 
//! 直接使用 sherpa_rs_sys FFI 绑定，模仿 TMSpeech 的实现方式。
//! 与离线识别器不同，OnlineRecognizer 支持实时流式识别，
//! 并内置 endpoint 检测，无需额外的 VAD。

use sherpa_rs::sherpa_rs_sys;
use std::ffi::CString;
use std::ptr;

/// OnlineRecognizer 配置
#[derive(Debug, Clone)]
pub struct OnlineRecognizerConfig {
    /// encoder 模型路径
    pub encoder: String,
    /// decoder 模型路径
    pub decoder: String,
    /// joiner 模型路径
    pub joiner: String,
    /// tokens 文件路径
    pub tokens: String,
    /// 采样率
    pub sample_rate: i32,
    /// 特征维度
    pub feature_dim: i32,
    /// 线程数
    pub num_threads: i32,
    /// 是否启用 endpoint 检测
    pub enable_endpoint: bool,
    /// Rule1: 尾部静音最小时长 (秒) - 用于检测句子结束
    pub rule1_min_trailing_silence: f32,
    /// Rule2: 尾部静音最小时长 (秒) - 用于中间停顿
    pub rule2_min_trailing_silence: f32,
    /// Rule3: 最小语句长度 (秒)
    pub rule3_min_utterance_length: f32,
    /// 解码方法
    pub decoding_method: String,
    /// 是否开启调试模式
    pub debug: bool,
}

impl Default for OnlineRecognizerConfig {
    fn default() -> Self {
        Self {
            encoder: String::new(),
            decoder: String::new(),
            joiner: String::new(),
            tokens: String::new(),
            sample_rate: 16000,
            feature_dim: 80,
            num_threads: 2,
            enable_endpoint: true,
            rule1_min_trailing_silence: 2.4,
            rule2_min_trailing_silence: 1.2,
            rule3_min_utterance_length: 20.0,
            decoding_method: "greedy_search".to_string(),
            debug: false,
        }
    }
}

/// OnlineRecognizer 封装
/// 
/// 流式语音识别器，支持实时识别和 endpoint 检测
pub struct OnlineRecognizer {
    recognizer: *const sherpa_rs_sys::SherpaOnnxOnlineRecognizer,
    stream: *const sherpa_rs_sys::SherpaOnnxOnlineStream,
    sample_rate: i32,
}

// 手动实现 Send 和 Sync，因为 sherpa-onnx 内部是线程安全的
unsafe impl Send for OnlineRecognizer {}
unsafe impl Sync for OnlineRecognizer {}

impl OnlineRecognizer {
    /// 创建新的 OnlineRecognizer
    pub fn new(config: OnlineRecognizerConfig) -> Result<Self, String> {
        // 准备 C 字符串
        let encoder = CString::new(config.encoder.as_str()).map_err(|e| e.to_string())?;
        let decoder = CString::new(config.decoder.as_str()).map_err(|e| e.to_string())?;
        let joiner = CString::new(config.joiner.as_str()).map_err(|e| e.to_string())?;
        let tokens = CString::new(config.tokens.as_str()).map_err(|e| e.to_string())?;
        let decoding_method = CString::new(config.decoding_method.as_str()).map_err(|e| e.to_string())?;
        let provider = CString::new("cpu").map_err(|e| e.to_string())?;

        unsafe {
            // 构建 Transducer 模型配置
            let transducer_config = sherpa_rs_sys::SherpaOnnxOnlineTransducerModelConfig {
                encoder: encoder.as_ptr(),
                decoder: decoder.as_ptr(),
                joiner: joiner.as_ptr(),
            };

            // 构建模型配置
            let model_config = sherpa_rs_sys::SherpaOnnxOnlineModelConfig {
                transducer: transducer_config,
                tokens: tokens.as_ptr(),
                num_threads: config.num_threads,
                provider: provider.as_ptr(),
                debug: if config.debug { 1 } else { 0 },
                // 其他模型类型设为默认/空
                paraformer: std::mem::zeroed(),
                zipformer2_ctc: std::mem::zeroed(),
                model_type: ptr::null(),
                modeling_unit: ptr::null(),
                bpe_vocab: ptr::null(),
                tokens_buf: ptr::null(),
                tokens_buf_size: 0,
                nemo_ctc: std::mem::zeroed(),
            };

            // 特征配置
            let feat_config = sherpa_rs_sys::SherpaOnnxFeatureConfig {
                sample_rate: config.sample_rate,
                feature_dim: config.feature_dim,
            };

            // 识别器配置
            let recognizer_config = sherpa_rs_sys::SherpaOnnxOnlineRecognizerConfig {
                feat_config,
                model_config,
                decoding_method: decoding_method.as_ptr(),
                max_active_paths: 4,
                enable_endpoint: if config.enable_endpoint { 1 } else { 0 },
                rule1_min_trailing_silence: config.rule1_min_trailing_silence,
                rule2_min_trailing_silence: config.rule2_min_trailing_silence,
                rule3_min_utterance_length: config.rule3_min_utterance_length,
                // 其他配置
                hotwords_file: ptr::null(),
                hotwords_score: 0.0,
                ctc_fst_decoder_config: std::mem::zeroed(),
                hotwords_buf: ptr::null(),
                hotwords_buf_size: 0,
                rule_fsts: ptr::null(),
                rule_fars: ptr::null(),
                blank_penalty: 0.0,
                hr: std::mem::zeroed(),
            };

            // 创建识别器
            let recognizer = sherpa_rs_sys::SherpaOnnxCreateOnlineRecognizer(&recognizer_config);
            if recognizer.is_null() {
                return Err("Failed to create OnlineRecognizer. Please check your model files.".to_string());
            }

            // 创建流
            let stream = sherpa_rs_sys::SherpaOnnxCreateOnlineStream(recognizer);
            if stream.is_null() {
                sherpa_rs_sys::SherpaOnnxDestroyOnlineRecognizer(recognizer);
                return Err("Failed to create OnlineStream".to_string());
            }

            Ok(Self {
                recognizer,
                stream,
                sample_rate: config.sample_rate,
            })
        }
    }

    /// 接受音频波形数据
    pub fn accept_waveform(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }

        unsafe {
            sherpa_rs_sys::SherpaOnnxOnlineStreamAcceptWaveform(
                self.stream,
                self.sample_rate,
                samples.as_ptr(),
                samples.len() as i32,
            );
        }
    }

    /// 检查是否准备好解码
    pub fn is_ready(&self) -> bool {
        unsafe {
            sherpa_rs_sys::SherpaOnnxIsOnlineStreamReady(self.recognizer, self.stream) == 1
        }
    }

    /// 解码当前数据
    pub fn decode(&self) {
        unsafe {
            sherpa_rs_sys::SherpaOnnxDecodeOnlineStream(self.recognizer, self.stream);
        }
    }

    /// 检查是否到达 endpoint (句子结束)
    pub fn is_endpoint(&self) -> bool {
        unsafe {
            sherpa_rs_sys::SherpaOnnxOnlineStreamIsEndpoint(self.recognizer, self.stream) == 1
        }
    }

    /// 重置流状态 (用于开始新的句子)
    pub fn reset(&self) {
        unsafe {
            sherpa_rs_sys::SherpaOnnxOnlineStreamReset(self.recognizer, self.stream);
        }
    }

    /// 获取当前识别结果
    pub fn get_result(&self) -> String {
        unsafe {
            let result = sherpa_rs_sys::SherpaOnnxGetOnlineStreamResult(self.recognizer, self.stream);
            if result.is_null() {
                return String::new();
            }

            let text = if (*result).text.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr((*result).text)
                    .to_string_lossy()
                    .to_string()
            };

            sherpa_rs_sys::SherpaOnnxDestroyOnlineRecognizerResult(result);
            text
        }
    }

    /// 处理音频并返回识别结果
    /// 
    /// 返回 (text, is_endpoint)
    pub fn process(&self, samples: &[f32]) -> (String, bool) {
        // 接受波形
        self.accept_waveform(samples);

        // 尝试解码
        while self.is_ready() {
            self.decode();
        }

        // 获取结果
        let text = self.get_result();
        let is_endpoint = self.is_endpoint();

        (text, is_endpoint)
    }
}

impl Drop for OnlineRecognizer {
    fn drop(&mut self) {
        unsafe {
            if !self.stream.is_null() {
                sherpa_rs_sys::SherpaOnnxDestroyOnlineStream(self.stream);
            }
            if !self.recognizer.is_null() {
                sherpa_rs_sys::SherpaOnnxDestroyOnlineRecognizer(self.recognizer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OnlineRecognizerConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert!(config.enable_endpoint);
    }
}
