use agentkit_core::{
    agent::Agent,
    provider::types::{ChatMessage, Role},
};
use agentkit_runtime::SimpleAgent;

#[tokio::main]
async fn main() {
    // Ollama 独立示例：
    // - 默认连接 http://localhost:11434
    // - 可选环境变量：OLLAMA_BASE_URL
    // - 需要本地已经 `ollama serve` 并存在对应模型

    let provider = agentkit::provider::OllamaProvider::from_env().with_default_model("qwen3.5:27b");
    let agent = SimpleAgent::new(provider);

    let out = agent
        .run(agentkit_core::agent::types::AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "你好".to_string(),
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
