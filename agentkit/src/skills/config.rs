//! Skill 配置模块
//!
//! 支持多种配置文件格式：YAML, TOML, JSON
//! 参考 zeroclaw 项目的 Skill 设计

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Skill 配置（支持 SKILL.yaml, SKILL.toml, SKILL.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub skill: SkillMeta,
    #[serde(default)]
    pub tools: Vec<SkillToolConfig>,
    #[serde(default)]
    pub prompts: Vec<String>,
}

/// Skill 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// Skill 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolConfig {
    pub name: String,
    pub description: String,
    /// "shell", "http", "script"
    pub kind: String,
    /// The command/URL/script to execute
    pub command: String,
    #[serde(default)]
    pub args: HashMap<String, String>,
}

impl SkillConfig {
    /// 从文件加载配置（自动检测格式）
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Ok(serde_yaml::from_str(&content)?),
            "toml" => Ok(toml::from_str(&content)?),
            "json" => Ok(serde_json::from_str(&content)?),
            _ => Err(format!("Unsupported config format: {}", ext).into()),
        }
    }
    
    /// 尝试从目录加载配置（按优先级尝试不同格式）
    pub fn from_dir(dir: &Path) -> Option<Self> {
        // 优先级：TOML > YAML > JSON
        let formats = ["SKILL.toml", "SKILL.yaml", "SKILL.yml", "SKILL.json"];
        
        for format in &formats {
            let path = dir.join(format);
            if path.exists() {
                if let Ok(config) = Self::from_file(&path) {
                    return Some(config);
                }
            }
        }
        None
    }
}
