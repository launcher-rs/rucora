//! 基础聊天示例
//!
//! 展示最简单的对话场景
//!
//! # 设计理念
//!
//! - **Provider** = 提供 AI 能力（连接 API），不需要指定 model
//! - **Runtime** = 决策和执行单元，**必须指定 model**
//!
//! # 运行方式
//!
//! ```bash
//! # 方式 1：使用环境变量指定默认模型（推荐）
//! export OPENAI_API_KEY=sk-xxx
//! export OPENAI_DEFAULT_MODEL=gpt-4o
//! cargo run --example 01_basic_chat
//!
//! # 方式 2：Runtime 中显式指定模型
//! export OPENAI_API_KEY=sk-xxx
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
///
/// Provider 仅提供 AI 连接能力，不负责选择模型
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

    // 创建 Provider（仅仅提供 AI 能力）
    let provider = create_provider_from_env()?;

    // 方式 1：从环境变量读取模型（推荐）
    // 优先级：OPENAI_DEFAULT_MODEL > ANTHROPIC_DEFAULT_MODEL > ... > gpt-4o-mini
    let model = std::env::var("OPENAI_DEFAULT_MODEL")
        .or_else(|_| std::env::var("ANTHROPIC_DEFAULT_MODEL"))
        .or_else(|_| std::env::var("GEMINI_DEFAULT_MODEL"))
        .unwrap_or_else(|_| "gpt-4o-mini".to_string());

    info!("✓ 使用模型：{}", model);

    // 方式 2：显式指定模型（更清晰）
    // let model = "gpt-4o";  // 或者任何你想要的模型

    // 创建运行时（必须指定 model）
    // Runtime 作为决策单元，负责选择合适的模型
    let runtime = DefaultRuntime::new(provider, ToolRegistry::new(), model)
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
