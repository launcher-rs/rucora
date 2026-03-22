//! Agent 使用示例
//!
//! 本示例展示 Agent 和 Runtime 的完整使用方式，包括：
//! - Agent 独立运行（简单任务）
//! - Agent 嵌入 Runtime 运行（复杂任务）
//! - 自定义 Agent 实现
//! - 多 Provider 切换

use agentkit::prelude::*;
use agentkit::provider::{
    AnthropicProvider, DeepSeekProvider, GeminiProvider, MoonshotProvider, OpenRouterProvider,
};
use agentkit::core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use agentkit::core::provider::LlmProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

/// 示例 1: 使用 DefaultAgent 独立运行（简单任务）
///
/// DefaultAgent 是最简单的 Agent 实现，适合直接对话场景
async fn demo_default_agent_standalone() -> anyhow::Result<()> {
    info!("\n=== 示例 1: DefaultAgent 独立运行 ===");

    // 创建 Provider（这里使用 OpenRouter，支持多种模型）
    let provider = OpenRouterProvider::from_env()?
        .with_default_model("anthropic/claude-3-5-sonnet");

    // 创建 DefaultAgent
    let agent = agentkit::agent::DefaultAgent::builder()
        .provider(provider)
        .system_prompt("你是一个友好的助手，用简短的语句回复")
        .build();

    // 独立运行 Agent（简单对话）
    let input = AgentInput::new("你好，请介绍一下你自己");
    
    match agent.run(input).await {
        Ok(output) => {
            info!("✓ Agent 回复：{}", output.value);
        }
        Err(e) => {
            warn!("⚠ Agent 运行失败：{}", e);
        }
    }

    Ok(())
}

/// 示例 2: 使用 Runtime 运行 Agent（复杂任务，支持工具调用）
///
/// Runtime 负责执行 Agent 的决策，支持工具调用、多轮对话等复杂场景
async fn demo_agent_with_runtime() -> anyhow::Result<()> {
    info!("\n=== 示例 2: Agent + Runtime（支持工具）===");

    // 创建 Provider
    let provider = OpenRouterProvider::from_env()?
        .with_default_model("anthropic/claude-3-5-sonnet");

    // 创建工具注册表
    let tools = ToolRegistry::new()
        .register(agentkit::tools::EchoTool)
        .register(agentkit::tools::FileReadTool::new());

    // 创建 Runtime
    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt("你是一个有用的助手，可以使用工具帮助用户")
        .with_max_steps(5);

    // 使用 Runtime 运行（支持工具调用）
    let input = AgentInput::new("请回显这句话：Hello, Agent!");
    
    let mut stream = runtime.run_stream(input);
    while let Some(event) = stream.next().await {
        match event? {
            ChannelEvent::TokenDelta(delta) => {
                print!("{}", delta.delta);
            }
            ChannelEvent::ToolCall(call) => {
                info!("\n🔧 工具调用：{} {:?}", call.name, call.input);
            }
            ChannelEvent::ToolResult(result) => {
                info!("\n✓ 工具结果：{}", result.output);
            }
            _ => {}
        }
    }

    Ok(())
}

/// 示例 3: 自定义 Agent 实现
///
/// 实现自己的 Agent 逻辑，定义思考和决策流程
struct WeatherAgent {
    system_prompt: String,
}

#[async_trait]
impl Agent for WeatherAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        let input = context.input.text();

        // 简单规则：包含"天气"就调用工具，否则直接对话
        if input.contains("天气") {
            AgentDecision::ToolCall {
                name: "weather_query".to_string(),
                input: serde_json::json!({
                    "location": "北京",
                    "date": "today"
                }),
            }
        } else {
            AgentDecision::Chat {
                request: context.default_chat_request(),
            }
        }
    }

    fn name(&self) -> &str {
        "weather_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("天气查询助手".to_string())
    }
}

async fn demo_custom_agent() -> anyhow::Result<()> {
    info!("\n=== 示例 3: 自定义 Agent ===");

    // 创建 Provider
    let provider = OpenRouterProvider::from_env()?
        .with_default_model("anthropic/claude-3-5-sonnet");

    // 创建工具（这里用一个 Echo 工具模拟天气查询）
    let tools = ToolRegistry::new()
        .register(agentkit::tools::EchoTool);

    // 创建自定义 Agent
    let agent = WeatherAgent {
        system_prompt: "你是一个天气查询助手".to_string(),
    };

    // 使用 Runtime 运行自定义 Agent
    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt(&agent.system_prompt)
        .with_max_steps(5);

    // 测试 1: 普通对话
    info!("\n--- 测试 1: 普通对话 ---");
    let input = AgentInput::new("你好");
    let _ = runtime.run_with_agent(&agent, input).await;

    // 测试 2: 触发工具调用
    info!("\n--- 测试 2: 天气查询（触发工具）---");
    let input = AgentInput::new("北京今天天气怎么样？");
    let _ = runtime.run_with_agent(&agent, input).await;

    Ok(())
}

/// 示例 4: 多 Provider 切换
///
/// 展示如何在不同 Provider 之间切换
async fn demo_multi_provider() -> anyhow::Result<()> {
    info!("\n=== 示例 4: 多 Provider 切换 ===");

    // 准备多个 Provider
    let providers: Vec<(&str, Arc<dyn LlmProvider>)> = vec![
        // OpenRouter（需要 API Key）
        (
            "OpenRouter",
            Arc::new(
                OpenRouterProvider::from_env()
                    .unwrap_or_else(|_| OpenRouterProvider::with_api_key("sk-or-xxx")),
            ),
        ),
        // Anthropic（需要 API Key）
        (
            "Anthropic",
            Arc::new(
                AnthropicProvider::from_env()
                    .unwrap_or_else(|_| AnthropicProvider::with_api_key("sk-ant-xxx")),
            ),
        ),
        // Google Gemini（需要 API Key）
        (
            "Gemini",
            Arc::new(
                GeminiProvider::from_env()
                    .unwrap_or_else(|_| GeminiProvider::with_api_key("xxx")),
            ),
        ),
        // DeepSeek（需要 API Key）
        (
            "DeepSeek",
            Arc::new(
                DeepSeekProvider::from_env()
                    .unwrap_or_else(|_| DeepSeekProvider::with_api_key("sk-xxx")),
            ),
        ),
        // Moonshot（需要 API Key）
        (
            "Moonshot",
            Arc::new(
                MoonshotProvider::from_env()
                    .unwrap_or_else(|_| MoonshotProvider::with_api_key("sk-xxx")),
            ),
        ),
    ];

    let question = "用一句话介绍 Rust 编程语言";

    for (name, provider) in providers {
        info!("\n--- 使用 {} ---", name);

        let agent = agentkit::agent::DefaultAgent::builder()
            .provider(provider.clone())
            .system_prompt("你是一个简洁的助手，只用一句话回答")
            .build();

        match agent.run(AgentInput::new(question)).await {
            Ok(output) => {
                if let Some(content) = output.value.get("content").and_then(|v| v.as_str()) {
                    info!("✓ {} 回复：{}", name, content);
                }
            }
            Err(e) => {
                warn!("⚠ {} 失败：{}", name, e);
            }
        }
    }

    Ok(())
}

/// 示例 5: ReAct 模式的 Agent
///
/// ReAct = Reasoning + Acting，交替进行思考和行动
struct ReActAgent {
    max_thoughts: usize,
}

#[async_trait]
impl Agent for ReActAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // ReAct 模式：分析当前情况，决定下一步
        if context.step >= self.max_thoughts {
            return AgentDecision::Return(serde_json::json!({
                "status": "completed",
                "message": "已达到最大思考次数"
            }));
        }

        // 检查是否有工具结果
        if !context.tool_results.is_empty() {
            // 已经有工具结果，让 LLM 生成最终回复
            AgentDecision::Chat {
                request: context.default_chat_request(),
            }
        } else if context.input.text().contains("计算") {
            // 需要计算，调用工具
            AgentDecision::ToolCall {
                name: "calculator".to_string(),
                input: serde_json::json!({
                    "expression": "2 + 2"
                }),
            }
        } else {
            // 直接对话
            AgentDecision::Chat {
                request: context.default_chat_request(),
            }
        }
    }

    fn name(&self) -> &str {
        "react_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("ReAct 模式的智能助手".to_string())
    }
}

async fn demo_react_agent() -> anyhow::Result<()> {
    info!("\n=== 示例 5: ReAct Agent ===");

    let provider = OpenRouterProvider::from_env()?
        .with_default_model("anthropic/claude-3-5-sonnet");

    let tools = ToolRegistry::new()
        .register(agentkit::tools::EchoTool);

    let agent = ReActAgent { max_thoughts: 3 };

    let runtime = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt("你是一个 ReAct 模式的助手，会分析情况并决定是否需要使用工具")
        .with_max_steps(10);

    let input = AgentInput::new("请帮我计算 2+2，然后告诉我结果");
    
    match runtime.run_with_agent(&agent, input).await {
        Ok(output) => {
            info!("✓ ReAct Agent 回复：{}", output.value);
            info!("  - 消息历史：{} 条", output.messages.len());
            info!("  - 工具调用：{} 次", output.tool_calls.len());
        }
        Err(e) => {
            warn!("⚠ ReAct Agent 失败：{}", e);
        }
    }

    Ok(())
}

/// 主函数：运行所有示例
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("agentkit=info".parse().unwrap())
                .add_directive("agentkit_examples=info".parse().unwrap()),
        )
        .init();

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║       AgentKit Agent 使用示例集合                      ║");
    info!("╚════════════════════════════════════════════════════════╝");

    // 运行示例（取消注释以启用）
    
    // demo_default_agent_standalone().await?;
    // demo_agent_with_runtime().await?;
    // demo_custom_agent().await?;
    // demo_multi_provider().await?;
    // demo_react_agent().await?;

    info!("\n=== 所有示例完成 ===");
    info!("提示：取消注释相应的示例函数以运行");

    Ok(())
}
