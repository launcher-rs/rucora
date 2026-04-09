//! AgentKit 统一的对话管理器示例
//!
//! 展示如何使用 ConversationManager 管理对话和自动压缩。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 22_unified_conversation
//! ```
//!
//! ## 功能演示
//!
//! 1. **创建对话管理器** - 配置压缩参数
//! 2. **消息管理** - 添加、获取、清空消息
//! 3. **Token 监控** - 自动估算和追踪 token 使用
//! 4. **压缩检查** - 自动检测是否需要压缩
//! 5. **执行压缩** - 调用 LLM 生成对话摘要
//! 6. **Agent 集成** - 在 Agent 中使用对话管理器

use agentkit::compact::{CompactConfig, CompactStrategy};
use agentkit::conversation::ConversationManager;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 统一的对话管理器示例       ║");
    info!("╚════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 1: 创建带压缩功能的对话管理器
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 1: 创建带压缩功能的对话管理器");
    info!("═══════════════════════════════════════\n");

    info!("1.1 创建压缩配置...\n");
    
    let config = CompactConfig::new()
        .with_auto_compact(true)
        .with_strategy(CompactStrategy::Auto)
        .with_buffer_tokens(50_000);

    info!("   压缩配置:");
    info!("   - 自动压缩：{}", config.auto_compact_enabled);
    info!("   - 压缩策略：{:?}", config.strategy);
    info!("   - 缓冲区：{} tokens", config.auto_compact_buffer_tokens);
    info!("   - 警告缓冲区：{} tokens", config.warning_buffer_tokens);
    info!("");

    info!("1.2 创建对话管理器...\n");
    
    let mut manager = ConversationManager::new()
        .with_system_prompt("你是一个友好的助手，擅长编程和技术咨询。")
        .with_max_messages(100)
        .with_compact_config(config);

    info!("✓ 对话管理器创建成功");
    info!("  - 系统提示词：已设置");
    info!("  - 最大消息数：100");
    info!("  - 自动压缩：启用");
    info!("  - 压缩缓冲区：50,000 tokens\n");

    // ═══════════════════════════════════════════════════════════
    // 示例 2: 消息管理
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 2: 消息管理");
    info!("═══════════════════════════════════════\n");

    info!("2.1 添加消息...\n");
    
    // 模拟对话
    manager.add_user_message("你好，我想学习 Rust 编程");
    manager.add_assistant_message("你好！很高兴帮助你学习 Rust。Rust 是一门系统编程语言...");
    
    manager.add_user_message("Rust 的所有权系统是什么？");
    manager.add_assistant_message("所有权是 Rust 的核心特性之一。它包括三个主要概念...");
    
    manager.add_user_message("能举个例子吗？");
    manager.add_assistant_message("当然！比如你有一个 String 变量，当你把它赋值给另一个变量时...");

    info!("   已添加 6 条消息");
    info!("   当前 token 数：{}", manager.token_count());
    info!("   消息数量：{}\n", manager.len());

    info!("2.2 获取最近消息...\n");
    
    let recent = manager.get_recent_messages(4);
    info!("   最近 4 条消息:");
    for (i, msg) in recent.iter().enumerate() {
        let role = match msg.role {
            agentkit_core::provider::types::Role::User => "用户",
            agentkit_core::provider::types::Role::Assistant => "助手",
            agentkit_core::provider::types::Role::System => "系统",
            agentkit_core::provider::types::Role::Tool => "工具",
        };
        let preview = msg.content.chars().take(30).collect::<String>();
        info!("   {}. [{}] {}", i + 1, role, preview);
    }
    info!("");

    // ═══════════════════════════════════════════════════════════
    // 示例 3: Token 监控
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 3: Token 监控");
    info!("═══════════════════════════════════════\n");

    info!("3.1 模拟长对话...\n");
    
    // 模拟添加更多消息
    for i in 1..=30 {
        manager.add_user_message(format!("这是第 {} 轮对话的用户消息", i));
        manager.add_assistant_message(format!("这是第 {} 轮对话的助手回复，包含一些详细内容来增加 token 数量。", i));
        
        if i % 10 == 0 {
            info!("   已添加 {} 轮对话，当前 token 数：{}", i * 2, manager.token_count());
        }
    }
    info!("");

    info!("3.2 Token 使用统计:\n");
    
    let total_tokens = manager.token_count();
    let total_messages = manager.len();
    let avg_tokens_per_message = if total_messages > 0 {
        total_tokens / total_messages as u32
    } else {
        0
    };
    
    info!("   - 总消息数：{}", total_messages);
    info!("   - 总 token 数：{}", total_tokens);
    info!("   - 平均每条消息：{} tokens\n", avg_tokens_per_message);

    // ═══════════════════════════════════════════════════════════
    // 示例 4: 压缩检查
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 4: 压缩检查");
    info!("═══════════════════════════════════════\n");

    info!("4.1 检查不同模型的压缩需求...\n");
    
    let models = vec![
        ("gpt-4o", 128_000),
        ("gpt-4-turbo", 128_000),
        ("claude-3-sonnet", 200_000),
        ("local-model", 32_000),
    ];
    
    for (model, context_window) in &models {
        let should_compact = manager.should_compact(model);
        let usage_percent = (total_tokens as f64 / *context_window as f64) * 100.0;
        
        info!("   模型：{} (上下文：{}K)", model, context_window / 1000);
        info!("   - 使用率：{:.2}%", usage_percent);
        info!("   - 是否需要压缩：{}\n", if should_compact { "是" } else { "否" });
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 5: 执行压缩（需要 API）
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 5: 执行压缩");
    info!("═══════════════════════════════════════\n");

    // 检查 API 配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置，跳过实际压缩演示");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434\n");
        
        info!("压缩流程说明:\n");
        info!("   1. 分组消息（按 API 轮次）");
        info!("   2. 选择要压缩的组（保留最近 3 轮）");
        info!("   3. 调用 LLM 生成摘要");
        info!("   4. 创建边界消息");
        info!("   5. 替换已压缩的消息");
        info!("   6. 重新计算 token 计数\n");
    } else {
        use agentkit::provider::OpenAiProvider;

        let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        info!("5.1 创建 Provider...\n");
        
        let provider = OpenAiProvider::from_env()?;
        info!("   ✓ Provider 创建成功\n");

        info!("5.2 检查是否需要压缩...\n");
        
        if manager.should_compact(&model_name) {
            info!("   ✓ 检测到需要压缩\n");
            
            info!("5.3 执行压缩...\n");
            
            match manager.compact(&provider, &model_name).await {
                Ok(summary) => {
                    info!("   ✓ 压缩成功");
                    info!("   摘要长度：{} 字符", summary.len());
                    info!("   压缩后 token 数：{}", manager.token_count());
                    info!("   摘要预览：{}...\n", summary.chars().take(100).collect::<String>());
                }
                Err(e) => {
                    info!("   ✗ 压缩失败：{}\n", e);
                }
            }
        } else {
            info!("   ℹ 当前 token 使用量较低，不需要压缩\n");
            info!("   提示：可以继续添加消息以达到压缩阈值\n");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 6: 与 Agent 集成
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 6: 与 Agent 集成");
    info!("═══════════════════════════════════════\n");

    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置，跳过 Agent 集成演示\n");
    } else {
        use agentkit::agent::ToolAgent;
        use agentkit::prelude::Agent;
        use agentkit::provider::OpenAiProvider;

        let model_name = std::env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        info!("6.1 创建带对话管理的 Agent...\n");

        let provider = OpenAiProvider::from_env()?;

        let agent = ToolAgent::builder()
            .provider(provider)
            .model(&model_name)
            .with_conversation(true)  // 启用对话历史管理
            .system_prompt(
                "你是一个友好的助手。请简洁回答，但要包含必要的信息。"
            )
            .build();

        info!("   ✓ Agent 创建成功\n");

        info!("6.2 测试对话...\n");
        
        info!("   用户：你好\n");
        
        match agent.run("你好".into()).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("   助手：{}\n", text);
                }
            }
            Err(e) => {
                info!("   错误：{}\n", e);
            }
        }

        info!("   提示：Agent 的对话管理器现在支持自动压缩功能\n");
    }

    // ═══════════════════════════════════════════════════════════
    // 示例 7: 配置建议
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例 7: 配置建议");
    info!("═══════════════════════════════════════\n");

    info!("7.1 不同模型的推荐配置:\n");
    
    let configs = vec![
        ("GPT-4o / GPT-4-Turbo", 128_000, 20_000),
        ("Claude 3 / Claude 3.5", 200_000, 30_000),
        ("本地模型 (4K-8K)", 8_192, 1_000),
        ("本地模型 (16K-32K)", 32_000, 2_000),
    ];
    
    for (model, context, buffer) in configs {
        info!("   {}: ", model);
        info!("   - 上下文窗口：{} tokens", context);
        info!("   - 推荐 buffer：{} tokens", buffer);
        info!("   - 压缩触发点：{} tokens\n", context - buffer);
    }

    info!("7.2 压缩策略选择:\n");
    
    info!("   - Auto（自动）: 接近限制时自动触发");
    info!("     适用场景：长对话、自动化的场景\n");
    
    info!("   - Reactive（响应式）: API 拒绝时触发");
    info!("     适用场景：节省 token、可控的场景\n");
    
    info!("   - Manual（手动）: 用户主动触发");
    info!("     适用场景：需要精确控制的场景\n");

    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════\n");

    info!("📝 统一的对话管理器总结：\n");

    info!("1. 核心功能:");
    info!("   ✅ 消息管理（添加、获取、清空）");
    info!("   ✅ Token 计数（自动估算）");
    info!("   ✅ 自动压缩（接近限制时触发）");
    info!("   ✅ 压缩执行（调用 LLM 生成摘要）");
    info!("   ✅ 消息分组（按 API 轮次）\n");

    info!("2. 使用方式:");
    info!("   - 简单场景：with_conversation(true)");
    info!("   - 复杂场景：手动创建 ConversationManager");
    info!("   - 统一 API，易于使用\n");

    info!("3. 性能优化:");
    info!("   - 增量 token 计数");
    info!("   - 快速估算（避免完整 tokenization）");
    info!("   - 按需压缩");
    info!("   - 保留最近 3 轮对话\n");

    info!("4. 最佳实践:");
    info!("   - 根据模型选择合适的 buffer_tokens");
    info!("   - 启用自动压缩减少手动干预");
    info!("   - 定期监控 token 使用情况");
    info!("   - 在长对话场景中特别有用\n");

    Ok(())
}


