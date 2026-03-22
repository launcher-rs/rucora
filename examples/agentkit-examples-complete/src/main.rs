//! AgentKit 全部使用功能示例
//!
//! 本示例展示 AgentKit 的所有核心功能：
//! - Provider 配置和使用
//! - Tool 注册和调用
//! - Skill 加载和执行
//! - Memory 存储和检索
//! - Runtime 配置和执行
//!
//! # 运行方式
//!
//! ```bash
//! # 设置环境变量（可选，用于测试真实 API）
//! export OPENAI_API_KEY=your-api-key
//!
//! # 运行示例
//! cargo run -p agentkit-examples-complete
//! ```

use agentkit::memory::InMemoryMemory;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{
    EchoTool, GitTool, HttpRequestTool, MemoryRecallTool, MemoryStoreTool, ShellTool,
};
use agentkit::core::agent::types::AgentInput;
use agentkit::core::channel::types::ChannelEvent;
use agentkit::core::memory::{Memory, MemoryItem, MemoryQuery};
use agentkit::core::provider::types::{ChatMessage, Role};
use agentkit::core::provider::LlmProvider;
use agentkit::core::runtime::Runtime;
use agentkit::core::tool::Tool;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use futures_util::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("=== AgentKit 全部使用功能示例 ===");

    // 1. Provider 示例
    demo_provider().await?;

    // 2. Tool 示例
    demo_tools().await?;

    // 3. Memory 示例
    demo_memory().await?;

    // 4. Runtime 示例
    demo_runtime().await?;

    // 5. 完整 Agent 示例
    demo_complete_agent().await?;

    info!("\n=== 示例运行完成 ===");
    info!("提示：设置 OPENAI_API_KEY 环境变量可测试真实 API");

    Ok(())
}

/// 1. Provider 示例
async fn demo_provider() -> anyhow::Result<()> {
    info!("\n=== 1. Provider 示例 ===");

    // 尝试从环境变量创建 OpenAI Provider
    match OpenAiProvider::from_env() {
        Ok(provider) => {
            info!("✓ OpenAI Provider 创建成功");

            // 测试聊天
            let request = agentkit::core::provider::types::ChatRequest {
                messages: vec![ChatMessage {
                    role: Role::User,
                    content: "用一句话介绍 Rust".to_string(),
                    name: None,
                }],
                model: Some("gpt-4o-mini".to_string()),
                tools: None,
                temperature: Some(0.7),
                max_tokens: None,
                response_format: None,
                metadata: None,
            };

            match provider.chat(request).await {
                Ok(response) => {
                    info!("✓ 聊天成功：{}", response.message.content);
                }
                Err(e) => {
                    info!("⚠ 聊天失败（可能是 API Key 无效）: {}", e);
                }
            }
        }
        Err(e) => {
            info!("⚠ OpenAI Provider 创建失败（请设置 OPENAI_API_KEY）: {}", e);
            info!("  使用 Mock Provider 演示...");

            // 使用 Mock Provider 演示
            demo_mock_provider().await?;
        }
    }

    Ok(())
}

/// Mock Provider 演示
async fn demo_mock_provider() -> anyhow::Result<()> {
    use agentkit::core::error::ProviderError;
    use agentkit::core::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk};
    use async_trait::async_trait;
    use futures_util::stream::{self, BoxStream};
    use futures_util::StreamExt;

    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse {
                message: ChatMessage {
                    role: Role::Assistant,
                    content: "Rust 是一门系统编程语言，专注于安全和性能。".to_string(),
                    name: None,
                },
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            })
        }

        fn stream_chat(
            &self,
            _request: ChatRequest,
        ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError>
        {
            let text = "Rust 是一门系统编程语言。";
            let chars: Vec<char> = text.chars().collect();

            let stream = stream::unfold(0, move |index| {
                let chars = chars.clone();
                async move {
                    if index < chars.len() {
                        let chunk = ChatStreamChunk {
                            delta: Some(chars[index].to_string()),
                            tool_calls: vec![],
                            usage: None,
                            finish_reason: None,
                        };
                        Some((Ok(chunk), index + 1))
                    } else {
                        None
                    }
                }
            });

            Ok(Box::pin(stream))
        }
    }

    let provider = MockProvider;
    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "介绍 Rust".to_string(),
            name: None,
        }],
        model: None,
        tools: None,
        temperature: None,
        max_tokens: None,
        response_format: None,
        metadata: None,
    };

    let response = provider.chat(request).await?;
    info!("✓ Mock Provider 回复：{}", response.message.content);

    // 测试流式输出
    info!("\n--- 流式输出 ---");
    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "流式输出".to_string(),
            name: None,
        }],
        model: None,
        tools: None,
        temperature: None,
        max_tokens: None,
        response_format: None,
        metadata: None,
    };

    let mut stream = provider.stream_chat(request)?;
    let mut content = String::new();
    while let Some(chunk) = stream.next().await {
        if let Ok(chunk) = chunk {
            if let Some(delta) = &chunk.delta {
                content.push_str(delta);
                print!("{}", delta);
            }
        }
    }
    println!();
    info!("✓ 流式输出完成：{}", content);

    Ok(())
}

/// 2. Tool 示例
async fn demo_tools() -> anyhow::Result<()> {
    info!("\n=== 2. Tool 示例 ===");

    // Echo 工具
    let echo_tool = EchoTool;
    info!("Echo 工具：{}", echo_tool.name());
    let result = echo_tool.call(json!({"text": "Hello, AgentKit!"})).await?;
    info!("✓ Echo 结果：{}", result);

    // Git 工具
    let git_tool = GitTool::new();
    info!("\nGit 工具：{}", git_tool.name());
    info!("✓ Git 工具分类：{:?}", git_tool.categories());

    // HTTP 工具
    let http_tool = HttpRequestTool::new();
    info!("\nHTTP 工具：{}", http_tool.name());
    info!("✓ HTTP 工具描述：{:?}", http_tool.description());

    // Shell 工具
    let shell_tool = ShellTool::new();
    info!("\nShell 工具：{}", shell_tool.name());

    // 记忆工具
    let memory_store = MemoryStoreTool::new();
    let memory_recall = MemoryRecallTool::new();
    info!(
        "\n记忆工具：{} / {}",
        memory_store.name(),
        memory_recall.name()
    );

    info!("\n✓ 所有工具演示完成");

    Ok(())
}

/// 3. Memory 示例
async fn demo_memory() -> anyhow::Result<()> {
    info!("\n=== 3. Memory 示例 ===");

    let memory = InMemoryMemory::new();

    // 添加记忆
    memory
        .add(MemoryItem {
            id: "user:name".to_string(),
            content: "张三".to_string(),
            metadata: Some(json!({"category": "core"})),
        })
        .await?;
    info!("✓ 添加记忆：user:name = 张三");

    memory
        .add(MemoryItem {
            id: "user:lang".to_string(),
            content: "Rust".to_string(),
            metadata: Some(json!({"category": "core"})),
        })
        .await?;
    info!("✓ 添加记忆：user:lang = Rust");

    memory
        .add(MemoryItem {
            id: "daily:topic".to_string(),
            content: "AgentKit 示例".to_string(),
            metadata: Some(json!({"category": "daily"})),
        })
        .await?;
    info!("✓ 添加记忆：daily:topic = AgentKit 示例");

    // 检索记忆
    let results = memory
        .query(MemoryQuery {
            text: "user".to_string(),
            limit: 10,
        })
        .await?;
    info!("✓ 检索记忆：{} 条结果", results.len());

    for item in &results {
        info!("  - {}: {}", item.id, item.content);
    }

    Ok(())
}

/// 4. Runtime 示例
async fn demo_runtime() -> anyhow::Result<()> {
    info!("\n=== 4. Runtime 示例 ===");

    // 创建 Mock Provider
    use agentkit::core::error::ProviderError;
    use agentkit::core::provider::types::{ChatRequest, ChatResponse, ChatStreamChunk};
    use async_trait::async_trait;
    use futures_util::stream::{self, BoxStream};

    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse {
                message: ChatMessage {
                    role: Role::Assistant,
                    content: "你好！我是一个模拟的 AI 助手。".to_string(),
                    name: None,
                },
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            })
        }

        fn stream_chat(
            &self,
            _request: ChatRequest,
        ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError>
        {
            let text = "你好！我是助手。";
            let chars: Vec<char> = text.chars().collect();
            let stream = stream::unfold(0, move |index| {
                let chars = chars.clone();
                async move {
                    if index < chars.len() {
                        Some((
                            Ok(ChatStreamChunk {
                                delta: Some(chars[index].to_string()),
                                tool_calls: vec![],
                                usage: None,
                                finish_reason: None,
                            }),
                            index + 1,
                        ))
                    } else {
                        None
                    }
                }
            });
            Ok(Box::pin(stream))
        }
    }

    // 创建工具注册表
    let tools = ToolRegistry::new()
        .register(EchoTool)
        .register(MemoryStoreTool::new())
        .register(MemoryRecallTool::new());

    info!("✓ 工具注册表：{} 个工具", tools.len());

    // 创建运行时
    let runtime = DefaultRuntime::new(Arc::new(MockProvider), tools)
        .with_system_prompt("你是一个有用的助手")
        .with_max_steps(5);

    info!("✓ Runtime 创建成功");

    // 测试非流式执行
    let input = AgentInput::new("你好");

    match runtime.run(input.clone()).await {
        Ok(output) => {
            info!("✓ 非流式回复：{}", output.value);
        }
        Err(e) => {
            info!("⚠ 非流式执行失败：{}", e);
        }
    }

    // 测试流式执行
    info!("\n--- 流式执行 ---");
    let mut stream = runtime.run_stream(input);
    while let Some(event) = stream.next().await {
        match event {
            Ok(ChannelEvent::TokenDelta(delta)) => {
                print!("{}", delta.delta);
            }
            Ok(ChannelEvent::Message(msg)) => {
                info!("\n✓ 完整消息：{}", msg.content);
            }
            Ok(ChannelEvent::ToolCall(call)) => {
                info!("\n🔧 工具调用：{}", call.name);
            }
            Ok(ChannelEvent::ToolResult(result)) => {
                info!("\n✓ 工具结果：{}", result.output);
            }
            Ok(ChannelEvent::Error(err)) => {
                info!("\n❌ 错误：{}", err.message);
            }
            _ => {}
        }
    }

    Ok(())
}

/// 5. 完整 Agent 示例
async fn demo_complete_agent() -> anyhow::Result<()> {
    info!("\n=== 5. 完整 Agent 示例 ===");

    // 尝试使用真实 Provider
    match OpenAiProvider::from_env() {
        Ok(provider) => {
            // 创建工具注册表
            let tools = ToolRegistry::new()
                .register(EchoTool)
                .register(MemoryStoreTool::new())
                .register(MemoryRecallTool::new());

            // 创建运行时
            let runtime = DefaultRuntime::new(Arc::new(provider), tools)
                .with_system_prompt("你是一个有用的助手，可以帮助用户存储和检索信息")
                .with_max_steps(5);

            info!("✓ 完整 Agent 运行时创建成功");

            // 测试对话
            let input = AgentInput::new("你好，请记住我的名字是李四");

            info!("\n--- 对话开始 ---");
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
                        info!("\n✓ 最终回复：{}", msg.content);
                    }
                    _ => {}
                }
            }
        }
        Err(_) => {
            info!("⚠ 跳过完整 Agent 演示（请设置 OPENAI_API_KEY）");
        }
    }

    Ok(())
}
