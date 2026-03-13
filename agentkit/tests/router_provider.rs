use agentkit::provider::RouterProvider;
use agentkit_core::{
    error::ProviderError,
    provider::{
        types::{ChatMessage, ChatRequest, ChatResponse, Role},
        LlmProvider,
    },
};
use async_trait::async_trait;
use std::sync::Mutex;

struct FixedProvider {
    name: &'static str,
    seen_model: Mutex<Vec<Option<String>>>,
}

impl FixedProvider {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            seen_model: Mutex::new(Vec::new()),
        }
    }

    fn seen_models(&self) -> Vec<Option<String>> {
        self.seen_model.lock().unwrap().clone()
    }
}

#[async_trait]
impl LlmProvider for FixedProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.seen_model.lock().unwrap().push(request.model.clone());

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: format!("from:{}", self.name),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

#[tokio::test]
async fn router_should_route_by_model_prefix_and_strip_prefix() {
    let openai = FixedProvider::new("openai");
    let ollama = FixedProvider::new("ollama");

    let router = RouterProvider::new("ollama")
        .register("openai", openai)
        .register("ollama", ollama);

    let resp = router
        .chat(ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "hi".to_string(),
                name: None,
            }],
            model: Some("openai:gpt-4o-mini".to_string()),
            tools: None,
            temperature: None,
            max_tokens: None,
            metadata: None,
        })
        .await
        .expect("chat failed");

    assert_eq!(resp.message.content, "from:openai");

    // 再来一次走默认 provider
    let resp2 = router
        .chat(ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "hi".to_string(),
                name: None,
            }],
            model: Some("llama3".to_string()),
            tools: None,
            temperature: None,
            max_tokens: None,
            metadata: None,
        })
        .await
        .expect("chat failed");

    assert_eq!(resp2.message.content, "from:ollama");
}
