//! agentkit-providers - LLM Providers for AgentKit

pub mod anthropic;
pub mod azure_openai;
pub mod deepseek;
pub mod gemini;
pub mod helpers;
pub mod http_config;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod resilient;

pub use anthropic::AnthropicProvider;
pub use azure_openai::AzureOpenAiProvider;
pub use deepseek::DeepSeekProvider;
pub use gemini::GeminiProvider;
pub use moonshot::MoonshotProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use resilient::{CancelHandle, ResilientProvider, RetryConfig};

/// 预览函数
pub(crate) fn preview(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let truncated: String = s.char_indices().take(max).map(|(_, c)| c).collect();
        format!("{}...<truncated:{}>", truncated, s.len())
    }
}
