use agentkit_core::provider::{
    LlmProvider,
    types::{ChatMessage, ChatRequest, Role},
};

#[tokio::main]
async fn main() {
    // OpenAI 直接调用 chat 示例：
    // 运行前需要设置环境变量：OPENAI_API_KEY

    let provider = agentkit::provider::OpenAiProvider::from_env()
        .expect("缺少 OPENAI_API_KEY 或 OpenAI provider 初始化失败")
        .with_default_model("gpt-4o-mini");

    let resp = provider
        .chat(ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "用一句话介绍 Rust".to_string(),
                name: None,
            }],
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            metadata: None,
        })
        .await;

    match resp {
        Ok(resp) => println!("{}", resp.message.content),
        Err(e) => eprintln!("调用失败：{}", e),
    }
}
