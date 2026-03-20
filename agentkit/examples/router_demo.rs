use agentkit_core::{
    agent::types::AgentInput,
    provider::types::{ChatMessage, Role},
    runtime::Runtime,
};
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Router 示例：展示 RouterProvider + ToolCallingAgent 如何一起工作。
    //
    // 如果你只想单独测试 OpenAI / Ollama，请改用：
    // - examples/openai_demo.rs
    // - examples/ollama_demo.rs
    //
    // 运行前准备：
    // - 启动 ollama（可选）
    // - 或配置 OPENAI_API_KEY（可选）
    //
    // 注意：此示例不强制要求真实网络可用；如果 provider 不可用会直接报错。

    let openai = agentkit::provider::OpenAiProvider::from_env();
    let ollama = agentkit::provider::OllamaProvider::from_env().with_default_model("llama3");

    let mut router = agentkit::provider::RouterProvider::new("ollama").register("ollama", ollama);

    if let Ok(openai) = openai {
        router = router.register("openai", openai.with_default_model("gpt-4o-mini"));
    }

    let tools = ToolRegistry::new();
    let agent = DefaultRuntime::new(Arc::new(router), tools)
        .with_system_prompt(
            "你是一个严谨的助手。你可以调用外部工具获取真实信息。\n\
当用户询问今天几号、日期、农历等信息时，请调用日期工具获取真实结果后再回答。\n\
回答请使用中文。",
        )
        .with_max_steps(6);

    let out = agent
        .run(AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "用一句话介绍 Rust".to_string(),
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
