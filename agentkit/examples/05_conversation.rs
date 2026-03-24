//! Conversation 与 Agent/Runtime 结合使用示例
//!
//! 展示如何在 Agent 和 Runtime 中使用对话历史管理，实现多轮对话上下文。
//!
//! # 设计理念
//!
//! - **ConversationManager** = 管理对话历史（添加、截取、压缩消息）
//! - **Agent/Runtime** = 使用对话历史进行推理和工具调用
//! - **模型选择** = 在 Agent/Runtime 层面指定，Provider 仅提供连接能力
//!
//! # 运行方式
//!
//! ```bash
//! # 确保设置了环境变量
//! export OPENAI_API_KEY=sk-your-key
//! export OPENAI_BASE_URL=http://your-ollama-server:11434/v1
//!
//! # 运行示例
//! cargo run --example 05_conversation -p agentkit
//! ```

use agentkit::conversation::ConversationManager;
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::agent::DefaultAgent;
use agentkit_core::provider::types::Role;
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

    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║    AgentKit Conversation 与 Agent/Runtime 结合使用示例    ║");
    info!("╚═══════════════════════════════════════════════════════════╝\n");

    // 示例 1: ConversationManager 基础使用
    info!("=== 示例 1: ConversationManager 基础使用 ===");
    test_conversation_basics().await?;

    // 示例 2: 在 Runtime 中使用对话历史
    info!("\n=== 示例 2: 在 Runtime 中使用对话历史 ===");
    test_runtime_with_conversation().await?;

    // 示例 3: 在 Agent 中使用对话历史
    info!("\n=== 示例 3: 在 Agent 中使用对话历史 ===");
    test_agent_with_conversation().await?;

    // 示例 4: 多轮对话中的上下文保持
    info!("\n=== 示例 4: 多轮对话中的上下文保持 ===");
    test_multi_turn_conversation().await?;

    info!("\n=== 所有示例完成 ===");

    Ok(())
}

/// 示例 1: ConversationManager 基础使用
/// 展示对话历史管理的基本功能
async fn test_conversation_basics() -> anyhow::Result<()> {
    info!("1. 创建对话管理器...");
    let mut conv = ConversationManager::new()
        .with_system_prompt("你是一个有帮助的 AI 助手。")
        .with_max_messages(20); // 保留最近 20 条消息
    
    info!("   ✓ 已创建 ConversationManager (最大消息数：20)");

    // 添加对话消息
    info!("\n2. 添加对话消息...");
    conv.add_user_message("你好，请介绍一下自己".to_string());
    info!("   用户：你好，请介绍一下自己");

    conv.add_assistant_message("你好！我是一个 AI 助手，可以帮助你回答问题、完成任务。".to_string());
    info!("   助手：你好！我是一个 AI 助手，可以帮助你回答问题、完成任务。");

    conv.add_user_message("你会做什么？".to_string());
    info!("   用户：你会做什么？");

    conv.add_assistant_message(
        "我可以帮助你：\n1. 回答各种问题\n2. 编写和解释代码\n3. 翻译文本\n4. 总结文档\n等等。".to_string()
    );
    info!("   助手：我可以帮助你：1. 回答问题 2. 编写代码 3. 翻译文本 4. 总结文档");

    // 查看对话历史
    info!("\n3. 查看对话历史...");
    let messages = conv.get_messages();
    info!("   总消息数：{}", messages.len());

    for (i, msg) in messages.iter().enumerate() {
        let role_str = match msg.role {
            Role::System => "系统",
            Role::User => "用户",
            Role::Assistant => "助手",
            Role::Tool => "工具",
        };
        let preview: String = msg.content.chars().take(40).collect();
        info!("   {}. [{}] {}", i + 1, role_str, preview);
    }

    // 获取用于 API 调用的消息（包含系统提示）
    info!("\n4. 获取 API 消息格式...");
    let api_messages = conv.get_messages();
    info!("   API 消息数：{}", api_messages.len());
    
    for (i, msg) in api_messages.iter().enumerate() {
        let role_str = match msg.role {
            Role::System => "系统",
            Role::User => "用户",
            Role::Assistant => "助手",
            Role::Tool => "工具",
        };
        info!("   {}. [{}]", i + 1, role_str);
    }

    Ok(())
}

/// 示例 2: 在 Runtime 中使用对话历史
/// 展示如何在 Runtime 中集成对话历史管理
async fn test_runtime_with_conversation() -> anyhow::Result<()> {
    info!("1. 创建 Provider 和 Runtime...");
    
    // 创建 Provider（仅仅提供 AI 连接能力）
    let provider = OpenAiProvider::from_env()?;
    info!("   ✓ 已创建 OpenAiProvider");

    // 创建 Runtime（必须指定模型）
    let model = "qwen3.5:9b";
    let runtime = DefaultRuntime::new(
        Arc::new(provider),
        ToolRegistry::new(),
        model,
    );
    info!("   ✓ 已创建 DefaultRuntime (模型：{})", model);

    // 第一轮对话
    info!("\n2. 第一轮对话...");
    let input = AgentInput::new("AgentKit 是什么？");
    info!("   用户：AgentKit 是什么？");
    
    match runtime.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("   助手：{}\n", content);
            }
        }
        Err(e) => {
            info!("   ❌ 错误：{}\n", e);
        }
    }

    // 第二轮对话（带上下文）
    info!("3. 第二轮对话（带上下文）...");
    let input = AgentInput::new("它支持哪些 Provider？");
    info!("   用户：它支持哪些 Provider？");
    
    match runtime.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("   助手：{}\n", content);
            }
        }
        Err(e) => {
            info!("   ❌ 错误：{}\n", e);
        }
    }

    info!("   注意：Runtime 本身不维护对话历史，每次请求是独立的。");
    info!("   如需多轮对话，请使用 ConversationManager 管理历史。");

    Ok(())
}

/// 示例 3: 在 Agent 中使用对话历史
/// 展示如何在 Agent 中集成对话历史管理
async fn test_agent_with_conversation() -> anyhow::Result<()> {
    info!("1. 创建 Provider 和 Agent...");
    
    // 创建 Provider
    let provider = OpenAiProvider::from_env()?;
    info!("   ✓ 已创建 OpenAiProvider");

    // 创建 Agent（必须指定模型）
    let model = "qwen3.5:9b";
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model(model)
        .system_prompt("你是一个贴心的个人助手。")
        .max_steps(10)
        .build();
    info!("   ✓ 已创建 DefaultAgent (模型：{})", model);

    // 第一轮对话
    info!("\n2. 第一轮对话...");
    let input = AgentInput::new("你好，我叫小明");
    info!("   用户：你好，我叫小明");
    
    match agent.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("   助手：{}\n", content);
            }
        }
        Err(e) => {
            info!("   ❌ 错误：{}\n", e);
        }
    }

    // 第二轮对话
    info!("3. 第二轮对话...");
    let input = AgentInput::new("我今年 25 岁");
    info!("   用户：我今年 25 岁");
    
    match agent.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("   助手：{}\n", content);
            }
        }
        Err(e) => {
            info!("   ❌ 错误：{}\n", e);
        }
    }

    // 第三轮对话（测试记忆）
    info!("4. 第三轮对话（测试记忆）...");
    let input = AgentInput::new("我叫什么名字？");
    info!("   用户：我叫什么名字？");
    
    match agent.run(input).await {
        Ok(output) => {
            if let Some(content) = output.text() {
                info!("   助手：{}\n", content);
                info!("   注意：Agent 本身不维护对话历史，需要配合 ConversationManager 使用。");
            }
        }
        Err(e) => {
            info!("   ❌ 错误：{}\n", e);
        }
    }

    Ok(())
}

/// 示例 4: 多轮对话中的上下文保持
/// 展示如何使用 ConversationManager 配合 Agent 实现真正的多轮对话
async fn test_multi_turn_conversation() -> anyhow::Result<()> {
    info!("1. 创建 Provider、Agent 和 ConversationManager...");
    
    // 创建 Provider
    let provider = OpenAiProvider::from_env()?;
    info!("   ✓ 已创建 OpenAiProvider");

    // 创建对话管理器
    let mut conv = ConversationManager::new()
        .with_system_prompt("你是一个贴心的个人助手，记住用户的个人信息。")
        .with_max_messages(20);
    info!("   ✓ 已创建 ConversationManager");

    // 创建 Agent
    let model = "qwen3.5:9b";
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model(model)
        .max_steps(10)
        .build();
    info!("   ✓ 已创建 DefaultAgent (模型：{})", model);

    // 模拟多轮对话
    info!("\n2. 开始多轮对话（使用 ConversationManager 保持上下文）...\n");

    let conversations = vec![
        ("你好，我叫张三", "打招呼并介绍自己"),
        ("我是一名软件工程师", "回应用户的职业"),
        ("我平时喜欢用 Rust 编程", "讨论 Rust 语言"),
        ("你还记得我叫什么吗？", "测试记忆：应该回答张三"),
        ("我是做什么工作的？", "测试记忆：应该回答软件工程师"),
    ];

    for (i, (user_input, _description)) in conversations.iter().enumerate() {
        info!("【第 {} 轮】", i + 1);
        info!("   用户：{}", user_input);

        // 添加用户消息到对话历史
        conv.add_user_message(user_input.to_string());

        // 获取对话历史并创建请求
        let messages = conv.get_messages();
        
        // 构建包含历史消息的输入
        let input_text = user_input.to_string();
        let input = if messages.len() > 1 {
            // 有历史消息，构建包含上下文的请求
            let mut full_input = String::new();
            for msg in messages.iter().filter(|m| m.role != Role::System) {
                let role = match msg.role {
                    Role::User => "用户",
                    Role::Assistant => "助手",
                    _ => "",
                };
                if !role.is_empty() {
                    full_input.push_str(&format!("{}: {}\n", role, msg.content));
                }
            }
            full_input.push_str(&format!("助手："));
            AgentInput::new(full_input)
        } else {
            AgentInput::new(input_text)
        };

        // 运行 Agent
        let output: Result<agentkit::prelude::AgentOutput, _> = agent.run(input).await;
        match output {
            Ok(output) => {
                if let Some(content) = output.text() {
                    info!("   助手：{}", content);
                    
                    // 添加助手回复到对话历史
                    conv.add_assistant_message(content.to_string());
                }
            }
            Err(e) => {
                info!("   ❌ 错误：{}", e);
            }
        }
        info!("");
    }

    // 查看最终对话历史
    info!("3. 最终对话历史...");
    let messages = conv.get_messages();
    info!("   总消息数：{}", messages.len());

    for (i, msg) in messages.iter().enumerate() {
        let role_str = match msg.role {
            Role::System => "系统",
            Role::User => "用户",
            Role::Assistant => "助手",
            Role::Tool => "工具",
        };
        let preview: String = msg.content.chars().take(30).collect();
        info!("   {}. [{}] {}", i + 1, role_str, preview);
    }

    info!("\n✓ 通过 ConversationManager，Agent 可以保持多轮对话的上下文！");

    Ok(())
}
