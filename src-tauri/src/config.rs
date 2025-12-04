//! 模型配置模块
//! 支持多种 ASR 模型的灵活切换

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// ASR 模型类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum AsrModelType {
    /// Transducer 模型 (encoder/decoder/joiner)
    Transducer {
        encoder: String,
        decoder: String,
        joiner: String,
    },
    /// Paraformer 模型 (单一模型文件)
    Paraformer { model: String },
    /// Whisper 模型
    Whisper { encoder: String, decoder: String },
    /// SenseVoice 模型
    SenseVoice { model: String },
}

/// ASR 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrModelConfig {
    /// 模型唯一标识
    pub id: String,
    /// 模型显示名称
    pub name: String,
    /// 模型类型和路径
    pub model_type: AsrModelType,
    /// tokens 文件路径
    pub tokens: String,
    /// 支持的语言
    pub languages: Vec<String>,
    /// 采样率 (默认 16000)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    /// 线程数
    #[serde(default = "default_num_threads")]
    pub num_threads: i32,
}

fn default_sample_rate() -> u32 {
    16000
}

fn default_num_threads() -> i32 {
    2
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 当前使用的模型 ID
    pub current_model_id: String,
    /// 可用的 ASR 模型列表
    pub models: Vec<AsrModelConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            current_model_id: "tmspeech-zipformer".to_string(),
            models: vec![AsrModelConfig {
                id: "tmspeech-zipformer".to_string(),
                name: "TMSpeech Zipformer (中文)".to_string(),
                model_type: AsrModelType::Transducer {
                    encoder: "encoder.onnx".to_string(),
                    decoder: "decoder.onnx".to_string(),
                    joiner: "joiner.onnx".to_string(),
                },
                tokens: "tokens.txt".to_string(),
                languages: vec!["zh".to_string()],
                sample_rate: 16000,
                num_threads: 2,
            }],
        }
    }
}

impl AppConfig {
    /// 获取当前选中的模型配置
    pub fn current_model(&self) -> Option<&AsrModelConfig> {
        self.models.iter().find(|m| m.id == self.current_model_id)
    }

    /// 切换模型
    pub fn switch_model(&mut self, model_id: &str) -> bool {
        if self.models.iter().any(|m| m.id == model_id) {
            self.current_model_id = model_id.to_string();
            true
        } else {
            false
        }
    }

    /// 添加新模型
    pub fn add_model(&mut self, model: AsrModelConfig) {
        // 如果已存在同 ID 的模型，则更新
        if let Some(existing) = self.models.iter_mut().find(|m| m.id == model.id) {
            *existing = model;
        } else {
            self.models.push(model);
        }
    }

    /// 获取模型的绝对路径
    pub fn resolve_model_path(&self, relative_path: &str, base_dir: &PathBuf) -> PathBuf {
        let path = PathBuf::from(relative_path);
        if path.is_absolute() {
            path
        } else {
            base_dir.join(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.current_model_id, "tmspeech-zipformer");
        assert!(config.current_model().is_some());
    }

    #[test]
    fn test_switch_model() {
        let mut config = AppConfig::default();
        assert!(!config.switch_model("non-existent"));
        assert!(config.switch_model("tmspeech-zipformer"));
    }
}
