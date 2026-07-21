use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Provider 运行时标识。
///
/// 用于结构化路由选择，替代字符串拼接的 "provider/model" 模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeKey {
    OpenAi,
    Anthropic,
    Gemini,
    DeepSeek,
    Moonshot,
    OpenRouter,
    Ollama,
    AzureOpenAi,
}

impl RuntimeKey {
    /// 所有已知的运行时标识。
    pub const ALL: &'static [RuntimeKey] = &[
        RuntimeKey::OpenAi,
        RuntimeKey::Anthropic,
        RuntimeKey::Gemini,
        RuntimeKey::DeepSeek,
        RuntimeKey::Moonshot,
        RuntimeKey::OpenRouter,
        RuntimeKey::Ollama,
        RuntimeKey::AzureOpenAi,
    ];

    /// 返回运行时的人类可读名称。
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeKey::OpenAi => "openai",
            RuntimeKey::Anthropic => "anthropic",
            RuntimeKey::Gemini => "gemini",
            RuntimeKey::DeepSeek => "deepseek",
            RuntimeKey::Moonshot => "moonshot",
            RuntimeKey::OpenRouter => "openrouter",
            RuntimeKey::Ollama => "ollama",
            RuntimeKey::AzureOpenAi => "azure_openai",
        }
    }
}

impl std::fmt::Display for RuntimeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_', ' '], "") {
            s if s == "openai" || s == "openai compatible" => Ok(RuntimeKey::OpenAi),
            s if s == "anthropic" => Ok(RuntimeKey::Anthropic),
            s if s == "gemini" || s == "google" => Ok(RuntimeKey::Gemini),
            s if s == "deepseek" => Ok(RuntimeKey::DeepSeek),
            s if s == "moonshot" || s == "kimi" => Ok(RuntimeKey::Moonshot),
            s if s == "openrouter" => Ok(RuntimeKey::OpenRouter),
            s if s == "ollama" => Ok(RuntimeKey::Ollama),
            s if s == "azure" || s == "azureopenai" => Ok(RuntimeKey::AzureOpenAi),
            _ => Err(format!("未知的 RuntimeKey: {s}")),
        }
    }
}

/// 结构化模型路由选择。
///
/// 替代字符串格式的 "openai/gpt-4o" 路由方式，提供类型安全的模型和 Provider 选择。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSelection {
    /// 模型名称（如 "gpt-4o"、"claude-3-5-sonnet"）。
    pub model: String,
    /// Provider 运行时标识。
    pub runtime_key: RuntimeKey,
    /// 自定义 Base URL（可选，用于兼容端点）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl RouteSelection {
    /// 创建新的路由选择。
    pub fn new(model: impl Into<String>, runtime_key: RuntimeKey) -> Self {
        Self {
            model: model.into(),
            runtime_key,
            base_url: None,
        }
    }

    /// 设置自定义 Base URL。
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// 将 route 转换为常见的 "provider/model" 格式字符串。
    pub fn to_provider_model(&self) -> String {
        format!("{}/{}", self.runtime_key, self.model)
    }
}

impl std::fmt::Display for RouteSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.runtime_key, self.model)
    }
}

impl FromStr for RouteSelection {
    type Err = String;

    /// 从 "provider/model" 格式字符串解析 RouteSelection。
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(format!("路由格式错误，应为 \"provider/model\"，得到: {s}"));
        }
        let runtime_key = RuntimeKey::from_str(parts[0])?;
        Ok(RouteSelection {
            model: parts[1].to_string(),
            runtime_key,
            base_url: None,
        })
    }
}

/// 扩展方法：为 Option<String> 提供 RouteSelection 解析便利。
pub trait RouteSelectionExt {
    fn to_route_selection(&self) -> Option<RouteSelection>;
}

impl RouteSelectionExt for Option<String> {
    fn to_route_selection(&self) -> Option<RouteSelection> {
        self.as_ref().and_then(|s| RouteSelection::from_str(s).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_key_display() {
        assert_eq!(RuntimeKey::OpenAi.to_string(), "openai");
        assert_eq!(RuntimeKey::Anthropic.to_string(), "anthropic");
    }

    #[test]
    fn test_runtime_key_from_str() {
        assert_eq!("openai".parse::<RuntimeKey>().unwrap(), RuntimeKey::OpenAi);
        assert_eq!("Anthropic".parse::<RuntimeKey>().unwrap(), RuntimeKey::Anthropic);
        assert_eq!("google".parse::<RuntimeKey>().unwrap(), RuntimeKey::Gemini);
        assert_eq!("azure_openai".parse::<RuntimeKey>().unwrap(), RuntimeKey::AzureOpenAi);
        assert!("unknown".parse::<RuntimeKey>().is_err());
    }

    #[test]
    fn test_route_selection() {
        let route = RouteSelection::new("gpt-4o", RuntimeKey::OpenAi);
        assert_eq!(route.to_string(), "openai/gpt-4o");

        let parsed: RouteSelection = "openai/gpt-4o".parse().unwrap();
        assert_eq!(parsed.model, "gpt-4o");
        assert_eq!(parsed.runtime_key, RuntimeKey::OpenAi);
    }

    #[test]
    fn test_route_selection_with_base_url() {
        let route = RouteSelection::new("gpt-4o", RuntimeKey::OpenAi)
            .with_base_url("https://api.openai.com/v1");
        assert_eq!(route.base_url.as_deref(), Some("https://api.openai.com/v1"));
    }

    #[test]
    fn test_route_selection_from_option() {
        let opt = Some("ollama/qwen2.5:7b".to_string());
        let route = opt.to_route_selection().unwrap();
        assert_eq!(route.runtime_key, RuntimeKey::Ollama);
        assert_eq!(route.model, "qwen2.5:7b");
    }
}
