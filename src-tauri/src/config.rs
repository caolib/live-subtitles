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

/// 模型版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariant {
    /// 版本名称（如 "fp32", "int8"）
    pub variant_name: String,
    /// encoder 文件路径
    pub encoder: String,
    /// decoder 文件路径
    pub decoder: String,
    /// joiner 文件路径
    pub joiner: String,
}

/// 扫描模型文件夹的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedModelFiles {
    /// 模型 ID（目录名）
    pub id: String,
    /// 模型文件夹名称（作为模型名称）
    pub model_name: String,
    /// 模型文件夹完整路径
    pub model_dir: String,
    /// 检测到的 encoder 文件（默认版本）
    pub encoder: Option<String>,
    /// 检测到的 decoder 文件（默认版本）
    pub decoder: Option<String>,
    /// 检测到的 joiner 文件（默认版本）
    pub joiner: Option<String>,
    /// 检测到的 tokens 文件
    pub tokens: Option<String>,
    /// 是否是完整的模型（包含所有必需文件）
    pub is_complete: bool,
    /// 可用的模型版本列表（如果有多个版本）
    pub variants: Vec<ModelVariant>,
    /// 是否有多个版本
    pub has_multiple_variants: bool,
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

        let model_dir = dir.to_string_lossy().to_string();

        let mut result = ScannedModelFiles {
            id: model_name.clone(),
            model_name,
            model_dir,
            encoder: None,
            decoder: None,
            joiner: None,
            tokens: None,
            is_complete: false,
            variants: Vec::new(),
            has_multiple_variants: false,
        };

        // 收集所有文件
        let mut encoders = Vec::new();
        let mut decoders = Vec::new();
        let mut joiners = Vec::new();

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

                // 收集所有模型文件
                if file_name.contains("encoder") && file_name.ends_with(".onnx") {
                    encoders.push(path.to_string_lossy().to_string());
                } else if file_name.contains("decoder") && file_name.ends_with(".onnx") {
                    decoders.push(path.to_string_lossy().to_string());
                } else if file_name.contains("joiner") && file_name.ends_with(".onnx") {
                    joiners.push(path.to_string_lossy().to_string());
                } else if file_name.contains("tokens") && file_name.ends_with(".txt") {
                    result.tokens = Some(path.to_string_lossy().to_string());
                }
            }
        }

        // 优先选择 int8 版本作为默认版本（更快）
        result.encoder = encoders
            .iter()
            .find(|e| e.contains("int8"))
            .or_else(|| encoders.first())
            .cloned();
        result.decoder = decoders
            .iter()
            .find(|d| d.contains("int8"))
            .or_else(|| decoders.first())
            .cloned();
        result.joiner = joiners
            .iter()
            .find(|j| j.contains("int8"))
            .or_else(|| joiners.first())
            .cloned();

        // 检测多版本（如果 encoder/decoder/joiner 都有多个文件）
        if encoders.len() > 1 && decoders.len() > 1 && joiners.len() > 1 {
            result.has_multiple_variants = true;

            // 尝试匹配版本对（int8 和默认版本）
            // 1. 先找 int8 版本
            let int8_encoder = encoders.iter().find(|e| e.contains("int8"));
            let int8_decoder = decoders.iter().find(|d| d.contains("int8"));
            let int8_joiner = joiners.iter().find(|j| j.contains("int8"));

            if let (Some(encoder), Some(decoder), Some(joiner)) =
                (int8_encoder, int8_decoder, int8_joiner)
            {
                result.variants.push(ModelVariant {
                    variant_name: "int8".to_string(),
                    encoder: encoder.clone(),
                    decoder: decoder.clone(),
                    joiner: joiner.clone(),
                });
            }

            // 2. 再找默认版本（不含 int8 的就是 fp32）
            let fp32_encoder = encoders.iter().find(|e| !e.contains("int8"));
            let fp32_decoder = decoders.iter().find(|d| !d.contains("int8"));
            let fp32_joiner = joiners.iter().find(|j| !j.contains("int8"));

            if let (Some(encoder), Some(decoder), Some(joiner)) =
                (fp32_encoder, fp32_decoder, fp32_joiner)
            {
                result.variants.push(ModelVariant {
                    variant_name: "fp32".to_string(),
                    encoder: encoder.clone(),
                    decoder: decoder.clone(),
                    joiner: joiner.clone(),
                });
            }
        }

        // 计算是否完整
        result.is_complete = result.encoder.is_some()
            && result.decoder.is_some()
            && result.joiner.is_some()
            && result.tokens.is_some();

        Some(result)
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
