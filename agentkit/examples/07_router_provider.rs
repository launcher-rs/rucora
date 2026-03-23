//! Router Provider 使用示例
//!
//! Router Provider 可以在多个 Provider 之间路由请求
//!
//! # 运行方式
//!
//! ```bash
//! export OPENAI_API_KEY=sk-xxx
//! export ANTHROPIC_API_KEY=sk-ant-xxx
//! cargo run --example 07_router_provider -p agentkit
//! ```

use agentkit::prelude::*;
use agentkit::provider::{AnthropicProvider, OpenAiProvider, RouterProvider};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest};
use std::sync::Arc;
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
    info!("║     AgentKit Router Provider 使用示例                  ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 检查是否有至少一个 Provider
    let has_openai = std::env::var("OPENAI_API_KEY").is_ok();
    let has_anthropic = std::env::var("ANTHROPIC_API_KEY").is_ok();

    if !has_openai && !has_anthropic {
        info!("❌ 未找到 API Key");
        info!("\n请设置以下环境变量之一：");
        info!("  export OPENAI_API_KEY=sk-xxx");
        info!("  export ANTHROPIC_API_KEY=sk-ant-xxx\n");
        return Ok(());
    }

    // 1. 创建子 Provider
    info!("=== 1. 创建子 Provider ===\n");

    let mut providers: Vec<(&str, Arc<dyn LlmProvider + Send + Sync>)> = Vec::new();

    if has_openai {
        match OpenAiProvider::from_env() {
            Ok(p) => {
                info!("✓ OpenAI Provider 初始化成功");
                providers.push(("openai", Arc::new(p)));
            }
            Err(e) => {
                info!("⚠ OpenAI Provider 初始化失败：{}", e);
            }
        }
    }

    if has_anthropic {
        match AnthropicProvider::from_env() {
            Ok(p) => {
                info!("✓ Anthropic Provider 初始化成功");
                providers.push(("anthropic", Arc::new(p)));
            }
            Err(e) => {
                info!("⚠ Anthropic Provider 初始化失败：{}", e);
            }
        }
    }

    if providers.is_empty() {
        info!("\n❌ 没有可用的 Provider");
        return Ok(());
    }

    info!("\n共加载 {} 个 Provider\n", providers.len());

    // 2. 创建 Router Provider
    info!("=== 2. 创建 Router Provider ===\n");

    let mut router = RouterProvider::new("openai"); // 默认使用 openai

    if has_openai {
        if let Ok(p) = OpenAiProvider::from_env() {
            router = router.register("openai", p);
        }
    }

    if has_anthropic {
        if let Ok(p) = AnthropicProvider::from_env() {
            router = router.register("anthropic", p);
        }
    }

    info!("✓ Router Provider 创建成功");
    info!("  默认 Provider: openai");
    info!("  已注册 Provider: openai, anthropic");
    info!("");

    // 3. 测试路由功能
    info!("=== 3. 测试路由功能 ===\n");

    // 测试 1: 自动路由（默认策略）
    info!("测试 1: 自动路由（默认策略）");
    test_router(&mut router, "auto", "用一句话介绍 Rust").await?;

    // 测试 2: 指定 Provider
    if has_openai {
        info!("\n测试 2: 指定使用 OpenAI");
        test_router(&mut router, "openai", "用一句话介绍 Python").await?;
    }

    // 测试 3: 指定另一个 Provider
    if has_anthropic {
        info!("\n测试 3: 指定使用 Anthropic");
        test_router(&mut router, "anthropic", "用一句话介绍 Java").await?;
    }

    info!("\n=== Router Provider 示例完成 ===");

    Ok(())
}

/// 测试 Router Provider
async fn test_router(
    router: &mut RouterProvider,
    strategy: &str,
    prompt: &str,
) -> anyhow::Result<()> {
    info!("策略：{}", strategy);
    info!("问题：{}", prompt);

    let request = ChatRequest::from_user_text(prompt).with_model("gpt-4o-mini"); // 默认模型，实际使用取决于路由策略

    match router.chat(request).await {
        Ok(response) => {
            info!("✓ 回复：{}", response.message.content);
        }
        Err(e) => {
            info!("❌ 错误：{}", e);
        }
    }

    Ok(())
}
