//! ToolCallingAgent 的基本行为测试。
//!
//! 目标：验证 tool-calling loop 能够：
//! - 读取 provider 返回的 tool_calls
//! - 执行工具并把结果追加回 messages
//! - 在下一轮 provider 返回最终 assistant 消息时结束

use agentkit_core::{
    agent::types::AgentInput,
    error::{ProviderError, ToolError},
    provider::types::{ChatMessage, ChatRequest, ChatResponse, Role},
    provider::LlmProvider,
    runtime::Runtime,
    tool::types::{ToolCall, ToolResult},
    tool::Tool,
};
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// 一个测试用 provider：
/// - 第一次 chat 返回 tool_calls
/// - 第二次 chat 返回最终消息
struct TestProvider {
    step: Mutex<u32>,
}

struct TwoToolCallsProvider;

#[async_trait]
impl LlmProvider for TwoToolCallsProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 第一次：返回两个 tool_calls；第二次：看到 tool 消息后结束。
        let has_tool_msg = request.messages.iter().any(|m| m.role == Role::Tool);
        if !has_tool_msg {
            return Ok(ChatResponse {
                message: ChatMessage {
                    role: Role::Assistant,
                    content: "need tools".to_string(),
                    name: None,
                },
                tool_calls: vec![
                    ToolCall {
                        id: "call-1".to_string(),
                        name: "slow".to_string(),
                        input: json!({"id": 1}),
                    },
                    ToolCall {
                        id: "call-2".to_string(),
                        name: "slow".to_string(),
                        input: json!({"id": 2}),
                    },
                ],
                usage: None,
                finish_reason: None,
            });
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
}

/// 一个用于测试并发的慢工具：
/// - 每个调用都会先把 `in_flight` +1，并更新 `max_in_flight`
/// - 然后等待直到所有调用都开始（started == total），再继续返回
struct SlowTool {
    total: usize,
    started: Arc<AtomicUsize>,
    in_flight: Arc<AtomicUsize>,
    max_in_flight: Arc<AtomicUsize>,
}

#[async_trait]
impl Tool for SlowTool {
    fn name(&self) -> &str {
        "slow"
    }

    fn input_schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;

        // CAS 方式更新最大并发。
        loop {
            let cur = self.max_in_flight.load(Ordering::SeqCst);
            if now <= cur {
                break;
            }
            if self
                .max_in_flight
                .compare_exchange(cur, now, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }

        self.started.fetch_add(1, Ordering::SeqCst);
        while self.started.load(Ordering::SeqCst) < self.total {
            tokio::task::yield_now().await;
        }

        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok(input)
    }
}

#[tokio::test]
async fn tool_calling_agent_should_deny_dangerous_shell_by_default() {
    let provider = DangerousShellProvider {
        step: Mutex::new(0),
    };

    // 不需要真实 shell 工具：被 policy 拦截在 tool 查找之前。
    let tools = ToolRegistry::new();
    let agent = DefaultRuntime::new(Arc::new(provider), tools).with_max_steps(4);

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
    let agent = DefaultRuntime::new(Arc::new(provider), tools).with_max_steps(4);

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

#[tokio::test]
async fn tool_calling_agent_should_execute_tools_concurrently_and_keep_order() {
    let provider = TwoToolCallsProvider;

    let started = Arc::new(AtomicUsize::new(0));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));

    let tool = SlowTool {
        total: 2,
        started: started.clone(),
        in_flight: in_flight.clone(),
        max_in_flight: max_in_flight.clone(),
    };

    let tools = ToolRegistry::new().register(tool);
    let agent = DefaultRuntime::new(Arc::new(provider), tools)
        .with_max_steps(4)
        .with_max_tool_concurrency(2);

    let output = agent
        .run(AgentInput {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "go".to_string(),
                name: None,
            }],
            metadata: None,
        })
        .await
        .expect("agent should finish");

    assert_eq!(output.message.content, "done");
    assert_eq!(output.tool_results.len(), 2);
    assert!(max_in_flight.load(Ordering::SeqCst) >= 2);
    assert_eq!(output.tool_results[0].tool_call_id, "call-1");
    assert_eq!(output.tool_results[1].tool_call_id, "call-2");
}
