//! ASR 语音识别模块
//! 支持多种模型的统一接口

use crate::config::{AsrModelConfig, AsrModelType, VadConfig};
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};
use sherpa_rs::zipformer::{ZipFormer, ZipFormerConfig};
use std::path::PathBuf;

/// ASR 识别结果
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// 识别的文本
    pub text: String,
    /// 开始时间 (秒)
    pub start_time: f32,
    /// 持续时间 (秒)
    pub duration: f32,
    /// 是否是最终结果 (非中间结果)
    pub is_final: bool,
}

/// 统一的 ASR 识别器 trait
pub trait AsrRecognizer: Send {
    /// 识别音频片段
    fn recognize(&mut self, samples: &[f32], sample_rate: u32) -> Option<String>;
}

/// Transducer 识别器包装
pub struct TransducerWrapper {
    recognizer: TransducerRecognizer,
}

impl TransducerWrapper {
    pub fn new(config: &AsrModelConfig, base_dir: &PathBuf) -> Result<Self, String> {
        if let AsrModelType::Transducer {
            encoder,
            decoder,
            joiner,
        } = &config.model_type
        {
            let transducer_config = TransducerConfig {
                encoder: base_dir.join(encoder).to_string_lossy().to_string(),
                decoder: base_dir.join(decoder).to_string_lossy().to_string(),
                joiner: base_dir.join(joiner).to_string_lossy().to_string(),
                tokens: base_dir.join(&config.tokens).to_string_lossy().to_string(),
                num_threads: config.num_threads,
                sample_rate: config.sample_rate as i32,
                feature_dim: 80,
                ..Default::default()
            };

            let recognizer = TransducerRecognizer::new(transducer_config)
                .map_err(|e| format!("Failed to create Transducer recognizer: {}", e))?;

            Ok(Self { recognizer })
        } else {
            Err("Invalid model type for TransducerWrapper".to_string())
        }
    }
}

impl AsrRecognizer for TransducerWrapper {
    fn recognize(&mut self, samples: &[f32], sample_rate: u32) -> Option<String> {
        let text = self.recognizer.transcribe(sample_rate, samples);
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

/// ZipFormer 识别器包装 (离线)
pub struct ZipFormerWrapper {
    recognizer: ZipFormer,
}

impl ZipFormerWrapper {
    pub fn new(config: &AsrModelConfig, base_dir: &PathBuf) -> Result<Self, String> {
        if let AsrModelType::Transducer {
            encoder,
            decoder,
            joiner,
        } = &config.model_type
        {
            let zipformer_config = ZipFormerConfig {
                encoder: base_dir.join(encoder).to_string_lossy().to_string(),
                decoder: base_dir.join(decoder).to_string_lossy().to_string(),
                joiner: base_dir.join(joiner).to_string_lossy().to_string(),
                tokens: base_dir.join(&config.tokens).to_string_lossy().to_string(),
                num_threads: Some(config.num_threads),
                ..Default::default()
            };

            let recognizer = ZipFormer::new(zipformer_config)
                .map_err(|e| format!("Failed to create ZipFormer recognizer: {}", e))?;

            Ok(Self { recognizer })
        } else {
            Err("Invalid model type for ZipFormerWrapper".to_string())
        }
    }
}

impl AsrRecognizer for ZipFormerWrapper {
    fn recognize(&mut self, samples: &[f32], sample_rate: u32) -> Option<String> {
        let text = self.recognizer.decode(sample_rate, samples.to_vec());
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

/// 创建 ASR 识别器的工厂函数
pub fn create_recognizer(
    config: &AsrModelConfig,
    base_dir: &PathBuf,
) -> Result<Box<dyn AsrRecognizer>, String> {
    match &config.model_type {
        AsrModelType::Transducer { .. } => {
            // 优先使用 ZipFormer (更稳定)
            Ok(Box::new(ZipFormerWrapper::new(config, base_dir)?))
        }
        AsrModelType::Paraformer { .. } => {
            Err("Paraformer model is not yet implemented".to_string())
        }
        AsrModelType::Whisper { .. } => Err("Whisper model is not yet implemented".to_string()),
        AsrModelType::SenseVoice { .. } => {
            Err("SenseVoice model is not yet implemented".to_string())
        }
    }
}

/// VAD + ASR 识别引擎
pub struct RecognitionEngine {
    vad: SileroVad,
    recognizer: Box<dyn AsrRecognizer>,
    sample_rate: u32,
    /// 当前累积的语音样本
    speech_buffer: Vec<f32>,
    /// 样本计数器 (用于计算时间)
    sample_counter: usize,
    /// 语音开始时的样本位置
    speech_start_sample: usize,
}

impl RecognitionEngine {
    /// 创建识别引擎
    pub fn new(
        vad_config: &VadConfig,
        asr_config: &AsrModelConfig,
        base_dir: &PathBuf,
    ) -> Result<Self, String> {
        // 创建 VAD
        let silero_config = SileroVadConfig {
            model: base_dir
                .join(&vad_config.model)
                .to_string_lossy()
                .to_string(),
            threshold: vad_config.threshold,
            min_silence_duration: vad_config.min_silence_duration,
            min_speech_duration: vad_config.min_speech_duration,
            window_size: vad_config.window_size,
            sample_rate: asr_config.sample_rate,
            ..Default::default()
        };

        let vad = SileroVad::new(silero_config, 60.0)
            .map_err(|e| format!("Failed to create VAD: {}", e))?;

        // 创建 ASR 识别器
        let recognizer = create_recognizer(asr_config, base_dir)?;

        Ok(Self {
            vad,
            recognizer,
            sample_rate: asr_config.sample_rate,
            speech_buffer: Vec::new(),
            sample_counter: 0,
            speech_start_sample: 0,
        })
    }

    /// 处理音频样本
    ///
    /// 返回识别结果 (如果有)
    pub fn process(&mut self, samples: &[f32]) -> Option<RecognitionResult> {
        let window_size = 512; // VAD 窗口大小
        let mut result = None;

        // 计算音频电平用于调试
        let max_amplitude: f32 = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let rms: f32 = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        
        // 每隔一段时间打印音频统计信息
        self.sample_counter += samples.len();
        if self.sample_counter % (self.sample_rate as usize * 2) < samples.len() {
            println!(
                "[ASR DEBUG] Samples: {}, Max amplitude: {:.4}, RMS: {:.6}",
                samples.len(), max_amplitude, rms
            );
        }

        for chunk in samples.chunks(window_size) {
            if chunk.len() < window_size {
                // 填充不足的部分
                let mut padded = chunk.to_vec();
                padded.resize(window_size, 0.0);
                self.vad.accept_waveform(padded);
            } else {
                self.vad.accept_waveform(chunk.to_vec());
            }

            // 检查是否检测到语音（实时检测）
            let is_speech = self.vad.is_speech();
            
            // 每2秒打印一次 VAD 状态
            if self.sample_counter % (self.sample_rate as usize * 2) < window_size {
                println!(
                    "[VAD DEBUG] is_speech: {}, is_empty: {}",
                    is_speech, self.vad.is_empty()
                );
            }

            // 检查 VAD 缓冲区
            while !self.vad.is_empty() {
                let segment = self.vad.front();

                // 计算时间信息
                let start_time = segment.start as f32 / self.sample_rate as f32;
                let duration = segment.samples.len() as f32 / self.sample_rate as f32;

                println!(
                    "[ASR DEBUG] Speech segment: start={:.2}s, duration={:.2}s, samples={}",
                    start_time, duration, segment.samples.len()
                );

                // 识别语音片段
                if let Some(text) = self
                    .recognizer
                    .recognize(&segment.samples, self.sample_rate)
                {
                    println!("[ASR DEBUG] Recognized: {}", text);
                    result = Some(RecognitionResult {
                        text,
                        start_time,
                        duration,
                        is_final: true,
                    });
                } else {
                    println!("[ASR DEBUG] No text recognized from segment");
                }

                self.vad.pop();
            }
        }

        result
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.vad.clear();
        self.speech_buffer.clear();
        self.sample_counter = 0;
        self.speech_start_sample = 0;
    }

    /// 获取采样率
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
