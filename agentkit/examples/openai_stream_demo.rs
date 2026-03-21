use agentkit_core::provider::{
    types::{ChatMessage, ChatRequest, Role},
    LlmProvider,
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    // OpenAI 流式示例：
    // 运行前需要设置环境变量：OPENAI_API_KEY

    let provider = agentkit::provider::OpenAiProvider::from_env()
        .expect("缺少 OPENAI_API_KEY 或 OpenAI provider 初始化失败")
        .with_default_model("gpt-4o-mini");

    let req = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "用一句话介绍 Rust".to_string(),
            name: None,
        }],
        model: None,
        tools: None,
        temperature: None,
        max_tokens: None,
        response_format: None,
        metadata: None,
    };

    let mut stream = provider
        .stream_chat(req)
        .expect("stream_chat not supported");

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                if let Some(delta) = chunk.delta {
                    print!("{}", delta);
                }
            }
            Err(e) => {
                eprintln!("\n流式输出失败：{}", e);
                break;
            }
        }
    }

    println!();
}
