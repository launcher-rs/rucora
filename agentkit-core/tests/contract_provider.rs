use agentkit_core::error::ProviderError;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, ChatResponse, Role};
use agentkit_core::provider::LlmProvider;
use async_trait::async_trait;

struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let last_user = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: format!("echo: {}", last_user),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

#[tokio::test]
async fn provider_contract_chat_should_return_assistant_message() {
    let p = MockProvider;

    let req = ChatRequest {
        messages: vec![ChatMessage {
            role: Role::User,
            content: "hi".to_string(),
            name: None,
        }],
        model: None,
        tools: None,
        temperature: None,
        max_tokens: None,
        response_format: None,
        metadata: None,
    };

    let resp = p.chat(req).await.unwrap();
    assert_eq!(resp.message.role, Role::Assistant);
    assert!(resp.message.content.contains("hi"));
}

#[tokio::test]
async fn provider_contract_stream_chat_default_should_error() {
    // trait 默认实现约定：如果 provider 不支持流式，应返回 Err。
    let p = MockProvider;

    let req = ChatRequest {
        messages: vec![ChatMessage::user("hi")],
        model: None,
        tools: None,
        temperature: None,
        max_tokens: None,
        response_format: None,
        metadata: None,
    };

    match p.stream_chat(req) {
        Ok(_) => panic!("expected stream_chat to return Err for default implementation"),
        Err(err) => match err {
            ProviderError::Message(msg) => assert!(msg.contains("not supported")),
            _ => panic!("expected Message error, got {:?}", err),
        },
    }
}
