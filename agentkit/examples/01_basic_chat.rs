//! 基础聊天示例
//!
//! 展示最简单的对话场景
//!
//! # 运行方式
//!
//! ```bash
//! # 设置环境变量
//! export OPENAI_API_KEY=sk-xxx
//!
//! # 运行示例
//! cargo run --example 01_basic_chat
//! ```

use agentkit::prelude::*;
use agentkit::provider::{AnthropicProvider, GeminiProvider, OpenAiProvider, OpenRouterProvider};
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::provider::LlmProvider;
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// 从环境变量创建 Provider
fn create_provider_from_env() -> anyhow::Result<Arc<dyn LlmProvider + Send + Sync>> {
    // 尝试 OpenAI
    if let Ok(provider) = OpenAiProvider::from_env() {
        tracing::info!("✓ 使用 OpenAI Provider");
        return Ok(Arc::new(provider));
    }

    // 尝试 Anthropic
    if let Ok(provider) = AnthropicProvider::from_env() {
        tracing::info!("✓ 使用 Anthropic Provider");
        return Ok(Arc::new(provider));
    }

    // 尝试 Gemini
    if let Ok(provider) = GeminiProvider::from_env() {
        tracing::info!("✓ 使用 Gemini Provider");
        return Ok(Arc::new(provider));
    }

    // 尝试 OpenRouter
    if let Ok(provider) = OpenRouterProvider::from_env() {
        tracing::info!("✓ 使用 OpenRouter Provider");
        return Ok(Arc::new(provider));
    }

    anyhow::bail!(
        "未找到有效的 API Key。请设置以下环境变量之一：\n  - OPENAI_API_KEY\n  - ANTHROPIC_API_KEY\n  - GOOGLE_API_KEY\n  - OPENROUTER_API_KEY"
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         AgentKit 基础聊天示例                          ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 创建 Provider
    let provider = create_provider_from_env()?;

    // 创建运行时
    let runtime = DefaultRuntime::new(provider, ToolRegistry::new())
        .with_system_prompt("你是一个友好的助手，用简短的语句回复");

    // 对话循环
    info!("开始对话（输入 'quit' 退出）\n");

    loop {
        print!("你：");
        let mut input_text = String::new();
        std::io::stdin().read_line(&mut input_text)?;
        let input_text = input_text.trim();

        if input_text.is_empty()
            || input_text.to_lowercase() == "quit"
            || input_text.to_lowercase() == "exit"
        {
            info!("再见！");
            break;
        }

        // 创建输入
        let input = AgentInput::new(input_text.to_string());

        // 运行对话
        match runtime.run(input).await {
            Ok(output) => {
                if let Some(content) = output.text() {
                    info!("助手：{}\n", content);
                }
            }
            Err(e) => {
                info!("❌ 错误：{}\n", e);
            }
        }
    }

    Ok(())
}
