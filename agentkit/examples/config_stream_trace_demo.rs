use agentkit::config::AgentkitConfig;
use agentkit_core::agent::types::AgentInput;
use agentkit_core::provider::types::{ChatMessage, Role};
use agentkit_runtime::trace::write_trace_jsonl;
use agentkit_runtime::{ChannelEvent, DefaultRuntime, ToolRegistry};
use futures_util::StreamExt;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 示例：
    // - 从配置（env + file + profile）加载 provider
    // - 用 StreamingToolCallingAgent 运行一次流式对话
    // - 将 ChannelEvent 序列化写入 trace JSONL 文件
    //
    // 运行前准备：
    // - 可选：设置 AGENTKIT_CONFIG/AGENTKIT_PROFILE
    // - OpenAI：需要 OPENAI_API_KEY 或 AGENTKIT_OPENAI_API_KEY
    // - Ollama：确保本地已启动（默认 http://localhost:11434）

    let profile = AgentkitConfig::load().await.expect("load config failed");
    let provider = AgentkitConfig::build_provider(&profile).expect("build provider failed");

    let tools = ToolRegistry::new();
    let agent = DefaultRuntime::new(Arc::new(provider), tools).with_max_steps(2);

    let input = AgentInput {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "用一句话介绍 Rust".to_string(),
            name: None,
        }],
        metadata: None,
    };

    let mut events: Vec<ChannelEvent> = Vec::new();
    let mut s = agent.run_stream(input);

    while let Some(item) = s.next().await {
        match item {
            Ok(ev) => {
                // 简单打印：
                // - token 事件会非常多，这里只做最小展示
                println!("{:?}", ev);
                events.push(ev);
            }
            Err(e) => {
                eprintln!("agent error: {}", e);
                break;
            }
        }
    }

    let path =
        std::env::var("AGENTKIT_TRACE_PATH").unwrap_or_else(|_| "agentkit-trace.jsonl".to_string());
    write_trace_jsonl(&path, &events)
        .await
        .expect("write trace failed");

    eprintln!("trace saved: {} (events={})", path, events.len());
}
