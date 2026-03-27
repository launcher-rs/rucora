//! Provider 使用示例
//!
//! 展示如何使用不同的 LLM Provider
//!
//! # 运行方式
//!
//! ```bash
//! export OPENAI_API_KEY=sk-xxx
//! cargo run --example 02_provider -p agentkit
//! ```

use agentkit::provider::{AnthropicProvider, GeminiProvider, OpenAiProvider, OpenRouterProvider};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Provider 使用示例                     ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 示例 1: OpenAI Provider
    if let Ok(provider) = OpenAiProvider::from_env() {
        info!("=== OpenAI Provider ===");
        test_provider(&provider, "gpt-4o-mini").await?;
    } else {
        info!("⚠ 未设置 OPENAI_API_KEY，跳过 OpenAI 测试\n");
    }

    // 示例 2: Anthropic Provider
    if let Ok(provider) = AnthropicProvider::from_env() {
        info!("=== Anthropic Provider ===");
        test_provider(&provider, "claude-3-5-sonnet-20241022").await?;
    } else {
        info!("⚠ 未设置 ANTHROPIC_API_KEY，跳过 Anthropic 测试\n");
    }

    // 示例 3: Gemini Provider
    if let Ok(provider) = GeminiProvider::from_env() {
        info!("=== Gemini Provider ===");
        test_provider(&provider, "gemini-1.5-pro").await?;
    } else {
        info!("⚠ 未设置 GOOGLE_API_KEY，跳过 Gemini 测试\n");
    }

    // 示例 4: OpenRouter Provider
    if let Ok(provider) = OpenRouterProvider::from_env() {
        info!("=== OpenRouter Provider ===");
        test_provider(&provider, "anthropic/claude-3-5-sonnet").await?;
    } else {
        info!("⚠ 未设置 OPENROUTER_API_KEY，跳过 OpenRouter 测试\n");
    }

    info!("\n=== 所有 Provider 测试完成 ===");

    Ok(())
}

/// 测试 Provider
async fn test_provider(provider: &dyn LlmProvider, model: &str) -> anyhow::Result<()> {
    let request =
        ChatRequest::new(vec![ChatMessage::user("用一句话介绍 Rust 编程语言")]).with_model(model);

    match provider.chat(request).await {
        Ok(response) => {
            info!("✓ 回复：{}\n", response.message.content);
        }
        Err(e) => {
            info!("❌ 错误：{}\n", e);
        }
    }

    Ok(())
}
