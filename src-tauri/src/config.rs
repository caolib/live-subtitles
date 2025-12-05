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
    /// 模型文件夹路径
    pub model_dir: String,
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

/// 扫描模型文件夹的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedModelFiles {
    /// 检测到的 encoder 文件
    pub encoder: Option<String>,
    /// 检测到的 decoder 文件
    pub decoder: Option<String>,
    /// 检测到的 joiner 文件
    pub joiner: Option<String>,
    /// 检测到的 tokens 文件
    pub tokens: Option<String>,
    /// 模型文件夹名称（作为模型名称）
    pub model_name: String,
}

impl ScannedModelFiles {
    /// 扫描指定文件夹，自动识别模型文件
    pub fn scan_directory(dir: &PathBuf) -> Option<Self> {
        if !dir.is_dir() {
            return None;
        }

        let model_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut result = ScannedModelFiles {
            encoder: None,
            decoder: None,
            joiner: None,
            tokens: None,
            model_name,
        };

        // 遍历目录中的文件
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                // 模糊匹配文件类型
                if file_name.contains("encoder") && file_name.ends_with(".onnx") {
                    result.encoder = Some(path.to_string_lossy().to_string());
                } else if file_name.contains("decoder") && file_name.ends_with(".onnx") {
                    result.decoder = Some(path.to_string_lossy().to_string());
                } else if file_name.contains("joiner") && file_name.ends_with(".onnx") {
                    result.joiner = Some(path.to_string_lossy().to_string());
                } else if file_name.contains("tokens") && file_name.ends_with(".txt") {
                    result.tokens = Some(path.to_string_lossy().to_string());
                }
            }
        }

        Some(result)
    }

    /// 检查是否是完整的 Transducer 模型
    pub fn is_complete_transducer(&self) -> bool {
        self.encoder.is_some()
            && self.decoder.is_some()
            && self.joiner.is_some()
            && self.tokens.is_some()
    }
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
            current_model_id: "default".to_string(),
            models: vec![AsrModelConfig {
                id: "default".to_string(),
                name: "默认模型".to_string(),
                model_dir: String::new(),
                model_type: AsrModelType::Transducer {
                    encoder: "encoder.onnx".to_string(),
                    decoder: "decoder.onnx".to_string(),
                    joiner: "joiner.onnx".to_string(),
                },
                tokens: "tokens.txt".to_string(),
                languages: vec!["zh".to_string(), "en".to_string()],
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
    #[allow(dead_code)]
    pub fn switch_model(&mut self, model_id: &str) -> bool {
        if self.models.iter().any(|m| m.id == model_id) {
            self.current_model_id = model_id.to_string();
            true
        } else {
            false
        }
    }

    /// 添加新模型
    #[allow(dead_code)]
    pub fn add_model(&mut self, model: AsrModelConfig) {
        // 如果已存在同 ID 的模型，则更新
        if let Some(existing) = self.models.iter_mut().find(|m| m.id == model.id) {
            *existing = model;
        } else {
            self.models.push(model);
        }
    }

    /// 获取模型的绝对路径
    #[allow(dead_code)]
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
        assert_eq!(config.current_model_id, "default");
        assert!(config.current_model().is_some());
    }

    #[test]
    fn test_switch_model() {
        let mut config = AppConfig::default();
        assert!(!config.switch_model("non-existent"));
        assert!(config.switch_model("default"));
    }
}
