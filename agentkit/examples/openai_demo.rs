use agentkit::provider::OpenAiProvider;
use agentkit_core::{
    agent::types::AgentInput,
    provider::types::{ChatMessage, Role},
    runtime::Runtime,
};
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // OpenAI 独立示例：
    // 运行前需要设置环境变量：OPENAI_API_KEY

    // let provider = agentkit::provider::OpenAiProvider::from_env()
    //     .expect("缺少 OPENAI_API_KEY 或 OpenAI provider 初始化失败")
    //     .with_default_model("gpt-4o-mini");

    let provider = agentkit::provider::OpenAiProvider::new("http://127.0.0.1:11434/v1", "ollama")
        .with_default_model("qwen2.5:14b");

    let agent = DefaultRuntime::new(Arc::new(provider), ToolRegistry::new())
        .with_system_prompt("你是一个有帮助的助手。")
        .with_max_steps(8);

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
        Ok(out) => println!("{}", out.message.content),
        Err(e) => eprintln!("运行失败：{}", e),
    }
}
