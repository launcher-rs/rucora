//! 配置管理模块
//!
//! 负责加载和保存用户配置

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 支持的 Provider 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    AzureOpenAI,
    OpenRouter,
    DeepSeek,
    Moonshot,
    Ollama,
}

impl ProviderType {
    pub fn all() -> Vec<ProviderType> {
        vec![
            ProviderType::OpenAI,
            ProviderType::Anthropic,
            ProviderType::Gemini,
            ProviderType::AzureOpenAI,
            ProviderType::OpenRouter,
            ProviderType::DeepSeek,
            ProviderType::Moonshot,
            ProviderType::Ollama,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Gemini => "Google Gemini",
            ProviderType::AzureOpenAI => "Azure OpenAI",
            ProviderType::OpenRouter => "OpenRouter",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::Moonshot => "Moonshot (Kimi)",
            ProviderType::Ollama => "Ollama (本地)",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "gpt-4o-mini",
            ProviderType::Anthropic => "claude-3-5-sonnet-20241022",
            ProviderType::Gemini => "gemini-1.5-pro",
            ProviderType::AzureOpenAI => "gpt-4o",
            ProviderType::OpenRouter => "anthropic/claude-3-5-sonnet",
            ProviderType::DeepSeek => "deepseek-chat",
            ProviderType::Moonshot => "moonshot-v1-8k",
            ProviderType::Ollama => "llama2",
        }
    }
}

/// 用户配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub serpapi_keys: Option<String>,
}

impl AppConfig {
    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|mut path| {
            path.push(".agentkit");
            path.push("config.toml");
            path
        })
    }

    /// 加载配置
    pub fn load() -> Option<Self> {
        let path = Self::config_path()?;
        let content = fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    /// 保存配置
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path().ok_or_else(|| anyhow::anyhow!("无法获取配置路径"))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 检查配置是否完整
    pub fn is_complete(&self) -> bool {
        self.provider.is_some() && self.api_key.is_some() && self.model.is_some()
    }
}
