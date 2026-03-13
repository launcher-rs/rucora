use agentkit_core::provider::{types::{ChatMessage, ChatRequest, Role}, LlmProvider};

#[tokio::main]
async fn main() {
    // Ollama 直接调用 chat 示例：
    // - 默认连接 http://localhost:11434
    // - 可选环境变量：OLLAMA_BASE_URL
    // - 需要本地已经 `ollama serve` 并存在对应模型

    let provider = agentkit::provider::OllamaProvider::from_env().with_default_model("qwen2.5:14b");

    let resp = provider
        .chat(ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "你好".to_string(),
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


    let resp = provider.chat("你好".into()).await;

    match resp {
        Ok(resp) => println!("{}", resp.message.content),
        Err(e) => eprintln!("调用失败：{}", e),
    }
}
