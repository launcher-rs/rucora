//! 配置管理模块
//!
//! 负责加载和保存用户配置，支持：
//! 1. 环境变量优先（如 OPENAI_API_KEY 等）
//! 2. 配置文件 (~/.agentkit/config.toml)
//! 3. 交互式配置向导

use console::style;
use serde::{Deserialize, Serialize};
use std::env;
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
    Nvidia,
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
            ProviderType::Nvidia,
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
            ProviderType::Nvidia => "NVIDIA (DGX Cloud)",
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
            ProviderType::Ollama => "qwen3.5:9b",
            ProviderType::Nvidia => "nvidia/nemotron-4-340b-instruct",
        }
    }

    /// 获取默认 base_url（如果适用）
    pub fn default_base_url(&self) -> Option<&'static str> {
        match self {
            ProviderType::OpenAI => Some("https://api.openai.com/v1"),
            ProviderType::Anthropic => Some("https://api.anthropic.com/v1"),
            ProviderType::Gemini => Some("https://generativelanguage.googleapis.com/v1beta/openai"),
            ProviderType::AzureOpenAI => None, // Azure 需要自定义端点
            ProviderType::OpenRouter => Some("https://openrouter.ai/api/v1"),
            ProviderType::DeepSeek => Some("https://api.deepseek.com/v1"),
            ProviderType::Moonshot => Some("https://api.moonshot.cn/v1"),
            ProviderType::Ollama => Some("http://localhost:11434/v1"),
            ProviderType::Nvidia => Some("https://integrate.api.nvidia.com/v1"),
        }
    }

    /// 获取环境变量前缀
    #[allow(dead_code)]
    pub fn env_prefix(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "OPENAI",
            ProviderType::Anthropic => "ANTHROPIC",
            ProviderType::Gemini => "GOOGLE",
            ProviderType::AzureOpenAI => "AZURE_OPENAI",
            ProviderType::OpenRouter => "OPENROUTER",
            ProviderType::DeepSeek => "DEEPSEEK",
            ProviderType::Moonshot => "MOONSHOT",
            ProviderType::Ollama => "OLLAMA",
            ProviderType::Nvidia => "NVIDIA",
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
    /// Tavily 搜索 API Keys（可选，多个轮询使用）
    #[serde(default)]
    pub tavily_keys: Option<Vec<String>>,
    /// 旧字段向后兼容（读取后迁移到 tavily_keys）
    #[serde(default)]
    pub serpapi_keys: Option<Vec<String>>,
}

impl AppConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Option<Self> {
        // 尝试从常见的环境变量读取
        let provider = env::var("PROVIDER").ok().or_else(|| {
            // 根据 Base URL 推断 Provider（优先级高于 API Key）
            let base_url = env::var("BASE_URL")
                .or_else(|_| env::var("OPENAI_BASE_URL"))
                .or_else(|_| env::var("OLLAMA_BASE_URL"))
                .or_else(|_| env::var("NVIDIA_BASE_URL"))
                .unwrap_or_default();

            if base_url.contains("11434") || base_url.contains("ollama") {
                Some("Ollama (本地)".to_string())
            } else if base_url.contains("nvidia") || base_url.contains("nemotron") {
                Some("NVIDIA (DGX Cloud)".to_string())
            } else if base_url.contains("anthropic") {
                Some("Anthropic".to_string())
            } else if base_url.contains("google") || base_url.contains("gemini") {
                Some("Google Gemini".to_string())
            } else if base_url.contains("openrouter") {
                Some("OpenRouter".to_string())
            } else if base_url.contains("deepseek") {
                Some("DeepSeek".to_string())
            } else if base_url.contains("moonshot") {
                Some("Moonshot (Kimi)".to_string())
            } else if base_url.contains("azure") {
                Some("Azure OpenAI".to_string())
            // 根据 API Key 环境变量推断 Provider
            } else if env::var("OPENAI_API_KEY").is_ok() {
                Some("OpenAI".to_string())
            } else if env::var("ANTHROPIC_API_KEY").is_ok() {
                Some("Anthropic".to_string())
            } else if env::var("GOOGLE_API_KEY").is_ok() {
                Some("Google Gemini".to_string())
            } else if env::var("AZURE_OPENAI_API_KEY").is_ok() {
                Some("Azure OpenAI".to_string())
            } else if env::var("OPENROUTER_API_KEY").is_ok() {
                Some("OpenRouter".to_string())
            } else if env::var("DEEPSEEK_API_KEY").is_ok() {
                Some("DeepSeek".to_string())
            } else if env::var("MOONSHOT_API_KEY").is_ok() {
                Some("Moonshot (Kimi)".to_string())
            } else if env::var("NVIDIA_API_KEY").is_ok() {
                Some("NVIDIA (DGX Cloud)".to_string())
            } else {
                None
            }
        });

        let api_key = env::var("API_KEY")
            .ok()
            .or_else(|| env::var("OPENAI_API_KEY").ok())
            .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
            .or_else(|| env::var("GOOGLE_API_KEY").ok())
            .or_else(|| env::var("AZURE_OPENAI_API_KEY").ok())
            .or_else(|| env::var("OPENROUTER_API_KEY").ok())
            .or_else(|| env::var("DEEPSEEK_API_KEY").ok())
            .or_else(|| env::var("MOONSHOT_API_KEY").ok())
            .or_else(|| env::var("NVIDIA_API_KEY").ok());

        let model = env::var("MODEL")
            .ok()
            .or_else(|| env::var("OPENAI_DEFAULT_MODEL").ok())
            .or_else(|| env::var("ANTHROPIC_DEFAULT_MODEL").ok())
            .or_else(|| env::var("GOOGLE_DEFAULT_MODEL").ok())
            .or_else(|| env::var("AZURE_OPENAI_DEFAULT_MODEL").ok())
            .or_else(|| env::var("OPENROUTER_DEFAULT_MODEL").ok())
            .or_else(|| env::var("DEEPSEEK_DEFAULT_MODEL").ok())
            .or_else(|| env::var("MOONSHOT_DEFAULT_MODEL").ok())
            .or_else(|| env::var("OLLAMA_DEFAULT_MODEL").ok())
            .or_else(|| env::var("NVIDIA_DEFAULT_MODEL").ok());

        let base_url = env::var("BASE_URL")
            .ok()
            .or_else(|| env::var("OPENAI_BASE_URL").ok())
            .or_else(|| env::var("ANTHROPIC_BASE_URL").ok())
            .or_else(|| env::var("GOOGLE_BASE_URL").ok())
            .or_else(|| env::var("AZURE_OPENAI_BASE_URL").ok())
            .or_else(|| env::var("OPENROUTER_BASE_URL").ok())
            .or_else(|| env::var("DEEPSEEK_BASE_URL").ok())
            .or_else(|| env::var("MOONSHOT_BASE_URL").ok())
            .or_else(|| env::var("OLLAMA_BASE_URL").ok())
            .or_else(|| env::var("NVIDIA_BASE_URL").ok());

        let serpapi_keys = env::var("SERPAPI_API_KEYS")
            .ok()
            .or_else(|| env::var("SERPAPI_API_KEY").ok());
        let serpapi_keys = serpapi_keys.map(|key| vec![key]);

        // Tavily Key（优先）
        let tavily_keys = env::var("TAVILY_API_KEYS")
            .ok()
            .or_else(|| env::var("TAVILY_API_KEY").ok())
            .map(|keys| {
                keys.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty());

        println!("api_key: {:?} model: {:?}, base_url: {:?}", api_key, model, base_url);

        // 只有当至少有一个配置项时才返回
        if api_key.is_some() || model.is_some() || base_url.is_some() {
            Some(Self {
                provider,
                api_key,
                model,
                base_url,
                tavily_keys,
                serpapi_keys,
            })
        } else {
            None
        }
    }

    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|mut path| {
            path.push(".agentkit");
            path.push("config.toml");
            path
        })
    }

    /// 加载配置（优先环境变量，其次配置文件）
    pub fn load() -> Option<Self> {
        // 优先从环境变量加载
        if let Some(env_config) = Self::from_env() {
            return Some(env_config);
        }

        // 其次从配置文件加载
        let path = Self::config_path()?;
        println!("读取配置文件 {}....",path.display());
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
        self.api_key.is_some() && self.model.is_some()
    }

    /// 显示当前配置
    pub fn display(&self) {
        println!("\n{}", style("━━━ 当前配置 ━━━").green().bold());

        if let Some(ref provider) = self.provider {
            println!("  Provider: {}", provider);
        }

        if let Some(ref api_key) = self.api_key {
            let masked_key = if api_key.len() > 10 {
                format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
            } else {
                "****".to_string()
            };
            println!("  API Key: {}", masked_key);
        }

        if let Some(ref model) = self.model {
            println!("  模型：{}", model);
        }

        if let Some(ref url) = self.base_url {
            println!("  Base URL: {}", url);
        }

        if self.serpapi_keys.is_some() {
            println!("  SerpAPI: 已配置（旧版，建议迁移至 Tavily）");
        }

        if let Some(ref keys) = self.tavily_keys {
            println!("  Tavily: 已配置 ({} 个 Key)", keys.len());
        } else {
            println!("  Tavily: 未配置（将使用 Browse+DuckDuckGo 降级方案）");
        }

        println!();
    }
}
