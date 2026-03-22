use agentkit_core::{
    agent::types::{AgentInput, AgentOutput},
    error::{AgentError, ProviderError},
    provider::types::{ChatMessage, ChatRequest, ChatResponse, Role},
    provider::LlmProvider,
    runtime::Runtime,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Mutex;

/// 一个最小可运行的示例：演示"自定义 Runtime"。
///
/// 运行方式：
/// - `cargo run -p agentkit --example agent_loop_demo`
#[tokio::main]
async fn main() {
    let provider = TestProvider {
        calls: Mutex::new(0),
    };

    let rt = SingleShotRuntime { provider };

    let out = rt
        .run(AgentInput::new("你好"))
        .await
        .expect("runtime.run 失败");

    if let Some(content) = out.text() {
        println!("assistant: {}", content);
    } else {
        println!("assistant: {:?}", out.value);
    }
}

/// 一个用于演示的 provider：
/// - 每次调用都会把 `calls` +1
/// - 返回固定的 assistant 消息
struct TestProvider {
    calls: Mutex<u32>,
}

#[async_trait]
impl LlmProvider for TestProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut c = self.calls.lock().unwrap();
        *c += 1;

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: format!("ok(calls={})", *c),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

pub struct SingleShotRuntime<P> {
    provider: P,
}

#[async_trait]
impl<P> Runtime for SingleShotRuntime<P>
where
    P: LlmProvider,
{
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        // 将 AgentInput 转换为 ChatRequest
        let messages = vec![ChatMessage::user(input.text)];

        let req = ChatRequest {
            messages,
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: None,
        };

        let resp = self
            .provider
            .chat(req)
            .await
            .map_err(|e| AgentError::Message(e.to_string()))?;

        Ok(AgentOutput::with_history(
            json!({"content": resp.message.content}),
            vec![resp.message],
            vec![],
        ))
    }
}
