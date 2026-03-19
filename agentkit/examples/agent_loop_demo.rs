use agentkit_core::{
    agent::Agent,
    agent::types::AgentInput,
    error::ProviderError,
    provider::LlmProvider,
    provider::types::{ChatMessage, ChatRequest, ChatResponse, Role},
};
use agentkit_runtime::{AgentLoop, ToolCallingAgent, ToolRegistry};
use async_trait::async_trait;
use std::sync::Mutex;

/// 一个最小可运行的示例：演示 `agentkit-runtime` 的“可插拔 AgentLoop”。
///
/// 运行方式：
/// - `cargo run -p agentkit --example agent_loop_demo`
///
/// 该示例不依赖任何外部 API Key：
/// - 使用一个本地 `TestProvider` 来模拟 LLM 行为
/// - 使用一个自定义 `AgentLoop` 替换默认的 tool-calling loop
#[tokio::main]
async fn main() {
    let provider = TestProvider {
        calls: Mutex::new(0),
    };

    // ToolCallingAgent 仍然是主入口（带 tools/policy/audit 等通用能力）。
    // 但其内部的 loop 可以被替换。
    let agent = ToolCallingAgent::new(provider, ToolRegistry::new()).with_loop(SingleShotLoop);

    let out = agent
        .run(AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "你好".to_string(),
                name: None,
            }],
            metadata: None,
        })
        .await
        .expect("agent.run 失败");

    println!("assistant: {}", out.message.content);
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

/// 一个自定义的 loop：只调用一次 provider.chat 并直接返回。
///
/// 该 loop 的目的不是提供完整功能，而是示范：
/// - `ToolCallingAgent` 的执行流程由 `AgentLoop` 决定
/// - 你可以按需实现不同的 loop（Simple / ToolCalling / ReAct 等）
pub struct SingleShotLoop;

#[async_trait]
impl<P> AgentLoop<P> for SingleShotLoop
where
    P: LlmProvider,
{
    async fn run(
        &self,
        agent: &ToolCallingAgent<P>,
        input: AgentInput,
    ) -> Result<agentkit_core::agent::types::AgentOutput, agentkit_core::error::AgentError> {
        let req = ChatRequest {
            messages: input.messages,
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            metadata: input.metadata,
        };

        let resp = agent
            .provider()
            .chat(req)
            .await
            .map_err(|e| agentkit_core::error::AgentError::Message(e.to_string()))?;

        Ok(agentkit_core::agent::types::AgentOutput {
            message: resp.message,
            tool_results: vec![],
        })
    }
}
