use agentkit_core::provider::{
    types::{ChatMessage, ChatRequest, Role},
    LlmProvider,
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    // Ollama 流式示例：
    // - 默认连接 http://localhost:11434
    // - 可选环境变量：OLLAMA_BASE_URL
    // - 需要本地已经 `ollama serve` 并存在对应模型

    let provider = agentkit::provider::OllamaProvider::from_env().with_default_model("qwen2.5:14b");

    let req = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "你好，讲个笑话".to_string(),
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
