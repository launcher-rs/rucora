//! Agent 基本使用示例
//!
//! 本示例展示 DefaultAgent 的基本使用方式，包括：
//! - DefaultAgent 的创建和使用
//! - 自定义 Agent 实现
//! - Agent 与 Runtime 配合使用

use agentkit::agent::{DefaultAgent, DefaultAgentBuilder};
use agentkit::core::agent::{Agent, AgentContext, AgentDecision, AgentInput};
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use async_trait::async_trait;
use futures_util::StreamExt;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         AgentKit Agent 基本使用示例                    ║");
    info!("╚════════════════════════════════════════════════════════╝");

    // 示例 1: 使用 DefaultAgent 进行简单对话
    demo_default_agent().await?;

    // 示例 2: 自定义 Agent 实现
    demo_custom_agent().await?;

    // 示例 3: Agent 与 Runtime 配合使用（支持工具调用）
    demo_agent_with_runtime().await?;

    info!("\n=== 所有示例完成 ===");

    Ok(())
}

/// 示例 1: 使用 DefaultAgent 进行简单对话
async fn demo_default_agent() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 示例 1: DefaultAgent 简单对话 ===");

    // 尝试创建 OpenAI Provider
    match OpenAiProvider::from_env() {
        Ok(provider) => {
            // 创建 DefaultAgent
            let agent = DefaultAgentBuilder::new()
                .provider(provider)
                .system_prompt("你是一个友好的 AI 助手，用简洁的语言回答")
                .default_model("gpt-4o-mini")
                .build();

            info!("✓ DefaultAgent 创建成功");
            info!("  - Agent 名称：{}", agent.name());
            info!("  - Agent 描述：{:?}", agent.description());

            // 运行 Agent
            let input = AgentInput::new("用一句话介绍 Rust 编程语言");

            match agent.run(input).await {
                Ok(output) => {
                    info!("✓ Agent 回复：{}", output.value);
                }
                Err(e) => {
                    info!("⚠ Agent 运行失败：{}", e);
                }
            }
        }
        Err(e) => {
            info!("⚠ 跳过此示例（请设置 OPENAI_API_KEY 环境变量）: {}", e);
        }
    }

    Ok(())
}

/// 示例 2: 自定义 Agent 实现
async fn demo_custom_agent() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 示例 2: 自定义 Agent 实现 ===");

    // 创建一个简单的翻译 Agent
    struct TranslatorAgent {
        target_language: String,
    }

    #[async_trait]
    impl Agent for TranslatorAgent {
        async fn think(&self, context: &AgentContext) -> AgentDecision {
            let input_text = context.input.text();

            // 构建翻译请求
            let system_prompt = format!(
                "你是一个专业的翻译助手，将用户输入翻译成{}。只输出翻译结果，不要解释。",
                self.target_language
            );

            let request = agentkit::core::provider::types::ChatRequest {
                messages: vec![
                    agentkit::core::provider::types::ChatMessage::system(system_prompt),
                    agentkit::core::provider::types::ChatMessage::user(input_text),
                ],
                model: None,
                tools: None,
                temperature: Some(0.3),
                max_tokens: None,
                response_format: None,
                metadata: None,
            };

            AgentDecision::Chat { request }
        }

        fn name(&self) -> &str {
            "translator_agent"
        }

        fn description(&self) -> Option<&str> {
            Some("翻译助手，翻译成指定语言")
        }
    }

    // 尝试使用真实 Provider
    match OpenAiProvider::from_env() {
        Ok(provider) => {
            let agent = TranslatorAgent {
                target_language: "中文".to_string(),
            };

            info!("✓ TranslatorAgent 创建成功");
            info!("  - Agent 名称：{}", agent.name());
            info!("  - Agent 描述：{:?}", agent.description());

            // 使用 Runtime 运行自定义 Agent
            let runtime =
                DefaultRuntime::new(Arc::new(provider), ToolRegistry::new()).with_max_steps(3);

            let input = AgentInput::new("Hello, how are you today?");

            info!("\n--- 翻译测试 ---");
            info!("原文：{}", input.text);

            match runtime.run_with_agent(&agent, input).await {
                Ok(output) => {
                    info!("✓ 翻译结果：{}", output.value);
                }
                Err(e) => {
                    info!("⚠ 翻译失败：{}", e);
                }
            }
        }
        Err(e) => {
            info!("⚠ 跳过此示例（请设置 OPENAI_API_KEY 环境变量）: {}", e);
        }
    }

    Ok(())
}

/// 示例 3: Agent 与 Runtime 配合使用（支持工具调用）
async fn demo_agent_with_runtime() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 示例 3: Agent + Runtime（支持工具调用）===");

    // 创建一个会使用工具的 Agent
    struct AssistantAgent;

    #[async_trait]
    impl Agent for AssistantAgent {
        async fn think(&self, context: &AgentContext) -> AgentDecision {
            let input_text = context.input.text();

            // 简单规则：包含"回显"就调用工具，否则直接对话
            if input_text.contains("回显") || input_text.contains("echo") {
                // 提取要回显的内容
                let content = input_text
                    .replace("回显", "")
                    .replace("echo", "")
                    .trim()
                    .to_string();

                AgentDecision::ToolCall {
                    name: "echo".to_string(),
                    input: serde_json::json!({
                        "text": if content.is_empty() { "Hello!" } else { &content }
                    }),
                }
            } else {
                // 直接对话
                let request = context.default_chat_request();
                AgentDecision::Chat { request }
            }
        }

        fn name(&self) -> &str {
            "assistant_agent"
        }

        fn description(&self) -> Option<&str> {
            Some("智能助手，可以使用工具")
        }
    }

    // 尝试使用真实 Provider
    match OpenAiProvider::from_env() {
        Ok(provider) => {
            // 创建工具注册表
            let tools = ToolRegistry::new().register(agentkit::tools::EchoTool);

            info!("✓ 工具注册表：{} 个工具", tools.len());

            // 创建 Runtime
            let runtime = DefaultRuntime::new(Arc::new(provider), tools)
                .with_system_prompt("你是一个有用的助手，可以使用工具帮助用户")
                .with_max_steps(5);

            let agent = AssistantAgent;

            // 测试 1: 普通对话
            info!("\n--- 测试 1: 普通对话 ---");
            let input = AgentInput::new("你好，请介绍一下你自己");

            match runtime.run_with_agent(&agent, input).await {
                Ok(output) => {
                    info!("✓ 回复：{}", output.value);
                    info!("  - 工具调用次数：{}", output.tool_calls.len());
                }
                Err(e) => {
                    info!("⚠ 失败：{}", e);
                }
            }

            // 测试 2: 触发工具调用
            info!("\n--- 测试 2: 触发工具调用 ---");
            let input = AgentInput::new("请回显这句话：Hello, AgentKit!");

            info!("输入：{}", input.text);
            info!("\n[流式输出开始]");

            let mut stream = runtime.run_stream(input);
            while let Some(event) = stream.next().await {
                match event {
                    Ok(ChannelEvent::TokenDelta(delta)) => {
                        print!("{}", delta.delta);
                    }
                    Ok(ChannelEvent::ToolCall(call)) => {
                        info!("\n🔧 工具调用：{} {:?}", call.name, call.input);
                    }
                    Ok(ChannelEvent::ToolResult(result)) => {
                        info!("\n✓ 工具结果：{}", result.output);
                    }
                    Ok(ChannelEvent::Message(msg)) => {
                        info!("\n[最终回复] {}", msg.content);
                    }
                    Ok(ChannelEvent::Error(err)) => {
                        info!("\n❌ 错误：{}", err.message);
                    }
                    _ => {}
                }
            }
            info!("[流式输出结束]");
        }
        Err(e) => {
            info!("⚠ 跳过此示例（请设置 OPENAI_API_KEY 环境变量）: {}", e);
        }
    }

    Ok(())
}
