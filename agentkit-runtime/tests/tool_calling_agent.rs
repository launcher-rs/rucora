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
use agentkit_runtime::{ResilientProvider, RetryConfig, ToolCallingAgent, ToolRegistry};
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// 一个测试用 provider：
/// - 第一次 chat 返回 tool_calls
/// - 第二次 chat 返回最终消息
struct TestProvider {
    step: Mutex<u32>,
}

struct FlakyProvider {
    attempts: Mutex<u32>,
}

#[async_trait]
impl LlmProvider for FlakyProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut a = self.attempts.lock().unwrap();
        *a += 1;
        if *a < 3 {
            return Err(ProviderError::Message("transient".to_string()));
        }
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: "ok".to_string(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}

#[tokio::test]
async fn resilient_provider_should_retry_chat() {
    let inner = Arc::new(FlakyProvider {
        attempts: Mutex::new(0),
    });

    let rp = ResilientProvider::new(inner).with_config(RetryConfig {
        max_retries: 5,
        base_delay_ms: 1,
        max_delay_ms: 2,
        timeout_ms: None,
    });

    let resp = rp
        .chat(ChatRequest {
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
        })
        .await
        .expect("chat should succeed after retries");

    assert_eq!(resp.message.content, "ok");
}

struct InfiniteStreamProvider;

#[async_trait]
impl LlmProvider for InfiniteStreamProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        Err(ProviderError::Message("not used".to_string()))
    }

    fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<
        BoxStream<'static, Result<agentkit_core::provider::types::ChatStreamChunk, ProviderError>>,
        ProviderError,
    > {
        let s = async_stream::try_stream! {
            loop {
                yield agentkit_core::provider::types::ChatStreamChunk {
                    delta: Some("x".to_string()),
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                };
            }
        };
        Ok(Box::pin(s))
    }
}

#[tokio::test]
async fn resilient_provider_stream_should_be_cancellable() {
    let inner = Arc::new(InfiniteStreamProvider);
    let rp = ResilientProvider::new(inner);

    let (handle, mut stream) = rp
        .stream_chat_cancellable(ChatRequest {
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
        })
        .expect("stream should start");

    // 先读到一个 chunk
    let first = stream.next().await.expect("should have first item");
    assert!(first.is_ok());

    handle.cancel();
    // 取消后，下一次 poll 应该尽快结束（None）
    let next = stream.next().await;
    assert!(next.is_none());
}

#[tokio::test]
async fn tool_calling_agent_should_deny_dangerous_shell_by_default() {
    let provider = DangerousShellProvider {
        step: Mutex::new(0),
    };

    // 不需要真实 shell 工具：被 policy 拦截在 tool 查找之前。
    let tools = ToolRegistry::new();
    let agent = ToolCallingAgent::new(provider, tools).with_max_steps(4);

    let output = agent
        .run(AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "please".to_string(),
                name: None,
            }],
            metadata: None,
        })
        .await
        .expect("agent should finish");

    assert_eq!(output.message.content, "done");
    assert_eq!(output.tool_results.len(), 1);
    let tr = &output.tool_results[0];
    assert_eq!(tr.tool_call_id, "call-1");
    assert_eq!(
        tr.output,
        json!({
            "ok": false,
            "error": {
                "kind": "policy_denied",
                "rule_id": "default.dangerous_command",
                "reason": "dangerous command 'rm' is blocked by default"
            },
            "truncated": false,
            "max_bytes": 65536
        })
    );
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

/// provider：第一次请求执行 shell（危险命令），第二次结束。
struct DangerousShellProvider {
    step: Mutex<u32>,
}

#[async_trait]
impl LlmProvider for DangerousShellProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut step = self.step.lock().unwrap();
        *step += 1;

        match *step {
            1 => Ok(ChatResponse {
                message: ChatMessage {
                    role: Role::Assistant,
                    content: "run dangerous shell".to_string(),
                    name: None,
                },
                tool_calls: vec![ToolCall {
                    id: "call-1".to_string(),
                    name: "shell".to_string(),
                    input: json!({"command": "rm", "args": ["-rf", "/"], "timeout": 1}),
                }],
                usage: None,
                finish_reason: None,
            }),
            2 => {
                let has_tool_msg = request.messages.iter().any(|m| m.role == Role::Tool);
                if !has_tool_msg {
                    return Err(ProviderError::Message("missing tool message".to_string()));
                }
                Ok(ChatResponse {
                    message: ChatMessage {
                        role: Role::Assistant,
                        content: "done".to_string(),
                        name: None,
                    },
                    tool_calls: vec![],
                    usage: None,
                    finish_reason: None,
                })
            }
            _ => Err(ProviderError::Message("unexpected".to_string())),
        }
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
    assert_eq!(
        output,
        &json!({"ok": true, "output": {"text": "hello"}, "truncated": false, "max_bytes": 65536})
    );
}
