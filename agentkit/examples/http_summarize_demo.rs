use agentkit::tools::HttpRequestTool;
use agentkit_core::{
    agent::types::AgentInput,
    provider::types::{ChatMessage, Role},
    runtime::Runtime,
};
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 注意：ToolCallingAgent 依赖 provider 在 ChatResponse.tool_calls 字段中返回结构化 tool calls。
    // 当前 OllamaProvider 仅返回 message.content，不解析/填充 tool_calls，
    // 模型可能会把 tool call 以 JSON 文本形式输出，导致流程直接结束。
    // 因此这里使用 OpenAiProvider（OpenAI-compatible API），指向 Ollama 的 /v1 接口。
    let provider = agentkit::provider::OpenAiProvider::new("http://127.0.0.1:11434/v1", "ollama")
        .with_default_model("qwen2.5:14b");

    let tools = ToolRegistry::new().register(agentkit::tools::HttpRequestTool::new());

    let agent = DefaultRuntime::new(Arc::new(provider), tools)
        .with_system_prompt(
            "你是一个严谨的总结助手。你可以调用外部工具获取网页内容，然后给出简洁但准确的总结。\n\
输出请使用中文。",
        )
        .with_max_steps(6);

    let url = "https://feeds.bbci.co.uk/news/world/rss.xml";

    let prompt = format!(
        "总结一下该网页内容：{}\n\
输出格式：\n\
1) 5-8 条要点（中文）\n\
2) 最后 1-2 句结论\n\
注意：如果你还没有拿到网页内容，请先调用工具获取网页内容，再总结。",
        url
    );

    let out = agent
        .run(agentkit_core::agent::types::AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: prompt,
                name: None,
            }],
            metadata: None,
        })
        .await;

    match out {
        Ok(out) => {
            println!("{}", out.message.content);
        }
        Err(e) => {
            eprintln!("运行失败：{}", e);
        }
    }
}
