//! ToolCallingAgent 的基本行为测试。
//!
//! 目标：验证 tool-calling loop 能够：
//! - 读取 provider 返回的 tool_calls
//! - 执行工具并把结果追加回 messages
//! - 在下一轮 provider 返回最终 assistant 消息时结束

use agentkit_core::{
    agent::Agent,
    agent::types::AgentInput,
    error::{ProviderError, ToolError},
    provider::LlmProvider,
    provider::types::{ChatMessage, ChatRequest, ChatResponse, Role},
    tool::Tool,
    tool::types::{ToolCall, ToolResult},
};
use agentkit_runtime::{ToolCallingAgent, ToolRegistry};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Mutex;

/// 一个测试用 provider：
/// - 第一次 chat 返回 tool_calls
/// - 第二次 chat 返回最终消息
struct TestProvider {
    step: Mutex<u32>,
}

#[async_trait]
impl LlmProvider for TestProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut step = self.step.lock().unwrap();
        *step += 1;

        match *step {
            1 => {
                // 第一次必须带着 tools 注册进来（ToolCallingAgent 会传）。
                if request.tools.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
                    return Err(ProviderError::Message("tools 未注册".to_string()));
                }

                Ok(ChatResponse {
                    message: ChatMessage {
                        role: Role::Assistant,
                        content: "我需要调用工具".to_string(),
                        name: None,
                    },
                    tool_calls: vec![ToolCall {
                        id: "call-1".to_string(),
                        name: "echo".to_string(),
                        input: json!({"text": "hello"}),
                    }],
                    usage: None,
                    finish_reason: None,
                })
            }
            2 => {
                // 第二次应该能看到 tool 消息被追加回 messages。
                let has_tool_msg = request.messages.iter().any(|m| m.role == Role::Tool);
                if !has_tool_msg {
                    return Err(ProviderError::Message(
                        "未看到 tool 消息被追加回 messages".to_string(),
                    ));
                }

                Ok(ChatResponse {
                    message: ChatMessage {
                        role: Role::Assistant,
                        content: "工具调用完成".to_string(),
                        name: None,
                    },
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                })
            }
            _ => Err(ProviderError::Message("不应出现第三次调用".to_string())),
        }
    }
}

/// 一个测试用 Echo 工具。
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn input_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        Ok(input)
    }
}

#[tokio::test]
async fn tool_calling_agent_should_execute_tool_and_finish() {
    let provider = TestProvider {
        step: Mutex::new(0),
    };

    let tools = ToolRegistry::new().register(EchoTool);
    let agent = ToolCallingAgent::new(provider, tools).with_max_steps(4);

    let output = agent
        .run(AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "请帮我".to_string(),
                name: None,
            }],
            metadata: None,
        })
        .await
        .expect("agent 运行失败");

    assert_eq!(output.message.role, Role::Assistant);
    assert_eq!(output.message.content, "工具调用完成");

    // tool_results 至少包含一次 echo 工具调用结果。
    assert_eq!(output.tool_results.len(), 1);
    let ToolResult {
        tool_call_id,
        output,
    } = &output.tool_results[0];
    assert_eq!(tool_call_id, "call-1");
    assert_eq!(output, &json!({"text": "hello"}));
}
