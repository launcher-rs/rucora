use agentkit_core::{
    agent::Agent,
    provider::types::{ChatMessage, Role},
};

#[tokio::main]
async fn main() {
    // OpenAI 独立示例：
    // 运行前需要设置环境变量：OPENAI_API_KEY

    let provider = agentkit::provider::OpenAiProvider::from_env()
        .expect("缺少 OPENAI_API_KEY 或 OpenAI provider 初始化失败")
        .with_default_model("gpt-4o-mini");

    let agent = agentkit::SimpleAgent::new(provider);

    let out = agent
        .run(agentkit_core::agent::types::AgentInput {
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
