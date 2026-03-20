use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream;
use futures_util::{StreamExt, stream::BoxStream};
use serde_json::json;
use tracing::{debug, info, warn};

use agentkit_core::agent::types::{AgentInput, AgentOutput};
use agentkit_core::channel::types::{ChannelEvent, DebugEvent, ErrorEvent, TokenDeltaEvent};
use agentkit_core::error::AgentError;
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::runtime::{NoopRuntimeObserver, Runtime, RuntimeObserver};
use agentkit_core::tool::types::{ToolCall, ToolResult};

use crate::policy::{DefaultToolPolicy, ToolPolicy};
use crate::tool_execution::{execute_tool_call_with_policy_and_observer, tool_result_to_message};
use crate::tool_registry::ToolRegistry;

/// 默认的最完整 runtime：
/// - 支持 tool-calling loop（非流式 run）
/// - 支持流式输出（run_stream，输出 ChannelEvent）
/// - 支持 tool policy
/// - 支持统一观测协议 RuntimeObserver
pub struct DefaultRuntime {
    provider: Arc<dyn LlmProvider>,
    system_prompt: Option<String>,
    tools: ToolRegistry,
    policy: Arc<dyn ToolPolicy>,
    observer: Arc<dyn RuntimeObserver>,
    max_steps: usize,
    max_tool_concurrency: usize,
}

impl DefaultRuntime {
    pub fn new(provider: Arc<dyn LlmProvider>, tools: ToolRegistry) -> Self {
        Self {
            provider,
            system_prompt: None,
            tools,
            policy: Arc::new(DefaultToolPolicy::new()),
            observer: Arc::new(NoopRuntimeObserver),
            max_steps: 8,
            max_tool_concurrency: 1,
        }
    }

    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicy>) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_observer(mut self, observer: Arc<dyn RuntimeObserver>) -> Self {
        self.observer = observer;
        self
    }

    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn with_max_tool_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_tool_concurrency = max_concurrency.max(1);
        self
    }

    fn emit(&self, event: ChannelEvent) {
        self.observer.on_event(event);
    }

    async fn execute_tool_calls(&self, calls: &[ToolCall]) -> Result<Vec<ToolResult>, AgentError> {
        if calls.is_empty() {
            return Ok(vec![]);
        }

        let max = self.max_tool_concurrency.max(1);
        if max == 1 || calls.len() == 1 {
            let mut out: Vec<ToolResult> = Vec::with_capacity(calls.len());
            for call in calls {
                out.push(
                    execute_tool_call_with_policy_and_observer(
                        &self.tools,
                        &self.policy,
                        &self.observer,
                        call,
                    )
                    .await?,
                );
            }
            return Ok(out);
        }

        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let observer = self.observer.clone();

        let results: Vec<Result<(usize, ToolResult), AgentError>> =
            stream::iter(calls.iter().cloned().enumerate().map(|(idx, call)| {
                let tools = tools.clone();
                let policy = policy.clone();
                let observer = observer.clone();
                async move {
                    let r = execute_tool_call_with_policy_and_observer(
                        &tools, &policy, &observer, &call,
                    )
                    .await?;
                    Ok((idx, r))
                }
            }))
            .buffer_unordered(max)
            .collect()
            .await;

        let mut ok: Vec<(usize, ToolResult)> = Vec::with_capacity(results.len());
        for r in results {
            ok.push(r?);
        }
        ok.sort_by_key(|(idx, _)| *idx);
        Ok(ok.into_iter().map(|(_, v)| v).collect())
    }

    pub fn run_stream(
        &self,
        mut input: AgentInput,
    ) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let observer = self.observer.clone();
        let system_prompt = self.system_prompt.clone();
        let max_steps = self.max_steps;
        let max_tool_concurrency = self.max_tool_concurrency;

        let stream = async_stream::try_stream! {
            if let Some(system_prompt) = &system_prompt {
                input.messages.insert(
                    0,
                    ChatMessage {
                        role: Role::System,
                        content: system_prompt.clone(),
                        name: None,
                    },
                );
            }

            let tool_defs = tools.definitions();
            let mut messages = input.messages;

            for step in 0..max_steps {
                debug!(step, messages_len = messages.len(), "stream_runtime.step.start");

                let ev = ChannelEvent::Debug(DebugEvent {
                    message: "step.start".to_string(),
                    data: Some(json!({"step": step, "messages_len": messages.len()})),
                });
                observer.on_event(ev.clone());
                yield ev;

                let request = ChatRequest {
                    messages: messages.clone(),
                    model: None,
                    tools: Some(tool_defs.clone()),
                    temperature: None,
                    max_tokens: None,
                    response_format: None,
                    metadata: input.metadata.clone(),
                };

                let mut assistant_text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                let s = provider
                    .stream_chat(request)
                    .map_err(|e| AgentError::Message(e.to_string()));

                let mut s = match s {
                    Ok(v) => v,
                    Err(e) => {
                        let ev = ChannelEvent::Error(ErrorEvent {
                            kind: "provider".to_string(),
                            message: e.to_string(),
                            data: Some(json!({"step": step})),
                        });
                        observer.on_event(ev.clone());
                        yield ev;
                        Err(e)?;
                        unreachable!();
                    }
                };

                while let Some(item) = s.next().await {
                    let chunk = match item {
                        Ok(v) => v,
                        Err(e) => {
                            let err = AgentError::Message(e.to_string());
                            let ev = ChannelEvent::Error(ErrorEvent {
                                kind: "provider".to_string(),
                                message: err.to_string(),
                                data: Some(json!({"step": step})),
                            });
                            observer.on_event(ev.clone());
                            yield ev;
                            Err(err)?;
                            unreachable!();
                        }
                    };

                    if let Some(delta) = chunk.delta {
                        assistant_text.push_str(&delta);
                        let ev = ChannelEvent::TokenDelta(TokenDeltaEvent { delta });
                        observer.on_event(ev.clone());
                        yield ev;
                    }

                    if !chunk.tool_calls.is_empty() {
                        tool_calls.extend(chunk.tool_calls);
                    }
                }

                let assistant_msg = ChatMessage {
                    role: Role::Assistant,
                    content: assistant_text,
                    name: None,
                };

                messages.push(assistant_msg.clone());
                let ev = ChannelEvent::Message(assistant_msg);
                observer.on_event(ev.clone());
                yield ev;

                if tool_calls.is_empty() {
                    let ev = ChannelEvent::Debug(DebugEvent {
                        message: "step.end(no_tool_calls)".to_string(),
                        data: Some(json!({"step": step})),
                    });
                    observer.on_event(ev.clone());
                    yield ev;
                    break;
                }

                let max = max_tool_concurrency.max(1);
                let results: Vec<Result<(usize, ToolResult), AgentError>> = stream::iter(
                    tool_calls
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(idx, call)| {
                            let tools = tools.clone();
                            let policy = policy.clone();
                            let observer = observer.clone();
                            async move {
                                let r = execute_tool_call_with_policy_and_observer(
                                    &tools, &policy, &observer, &call,
                                )
                                .await?;
                                Ok((idx, r))
                            }
                        }),
                )
                .buffer_unordered(max)
                .collect()
                .await;

                let mut ok: Vec<(usize, ToolResult)> = Vec::with_capacity(results.len());
                for r in results {
                    match r {
                        Ok(v) => ok.push(v),
                        Err(e) => {
                            let ev = ChannelEvent::Error(ErrorEvent {
                                kind: "tool".to_string(),
                                message: e.to_string(),
                                data: Some(json!({"step": step})),
                            });
                            observer.on_event(ev.clone());
                            yield ev;
                            Err(e)?;
                            unreachable!();
                        }
                    }
                }
                ok.sort_by_key(|(idx, _)| *idx);

                for (idx, result) in ok.into_iter() {
                    let call = &tool_calls[idx];

                    let ev = ChannelEvent::ToolCall(call.clone());
                    observer.on_event(ev.clone());
                    yield ev;

                    let ev = ChannelEvent::ToolResult(result.clone());
                    observer.on_event(ev.clone());
                    yield ev;

                    let tool_msg = tool_result_to_message(&result, &call.name);
                    messages.push(tool_msg);
                }

                let ev = ChannelEvent::Debug(DebugEvent {
                    message: "step.end".to_string(),
                    data: Some(json!({"step": step, "tool_calls": tool_calls.len()})),
                });
                observer.on_event(ev.clone());
                yield ev;
            }
        };

        Box::pin(stream)
    }
}

#[async_trait]
impl Runtime for DefaultRuntime {
    async fn run(&self, mut input: AgentInput) -> Result<AgentOutput, AgentError> {
        info!(max_steps = self.max_steps, "runtime.run.start");

        if let Some(system_prompt) = &self.system_prompt {
            input.messages.insert(
                0,
                ChatMessage {
                    role: Role::System,
                    content: system_prompt.clone(),
                    name: None,
                },
            );
        }

        let tool_defs = self.tools.definitions();
        let mut messages = input.messages;
        let mut tool_results: Vec<ToolResult> = Vec::new();

        for step in 0..self.max_steps {
            debug!(step, messages_len = messages.len(), "runtime.step.start");
            self.emit(ChannelEvent::Debug(DebugEvent {
                message: "step.start".to_string(),
                data: Some(json!({"step": step, "messages_len": messages.len()})),
            }));

            let request = ChatRequest {
                messages: messages.clone(),
                model: None,
                tools: Some(tool_defs.clone()),
                temperature: None,
                max_tokens: None,
                response_format: None,
                metadata: input.metadata.clone(),
            };

            let resp = self
                .provider
                .chat(request)
                .await
                .map_err(|e| AgentError::Message(e.to_string()))?;

            messages.push(resp.message.clone());
            self.emit(ChannelEvent::Message(resp.message.clone()));

            if resp.tool_calls.is_empty() {
                self.emit(ChannelEvent::Debug(DebugEvent {
                    message: "step.end(no_tool_calls)".to_string(),
                    data: Some(json!({"step": step})),
                }));
                info!(
                    step,
                    tool_results_len = tool_results.len(),
                    "runtime.run.done"
                );
                return Ok(AgentOutput {
                    message: resp.message,
                    tool_results,
                });
            }

            let results = self.execute_tool_calls(&resp.tool_calls).await?;
            for (idx, result) in results.into_iter().enumerate() {
                let call = &resp.tool_calls[idx];
                tool_results.push(result.clone());

                self.emit(ChannelEvent::ToolCall(call.clone()));
                self.emit(ChannelEvent::ToolResult(result.clone()));

                let tool_msg = tool_result_to_message(&result, call.name.as_str());
                messages.push(tool_msg);
            }

            self.emit(ChannelEvent::Debug(DebugEvent {
                message: "step.end".to_string(),
                data: Some(json!({"step": step, "tool_calls": resp.tool_calls.len()})),
            }));
        }

        warn!(
            max_steps = self.max_steps,
            tool_results_len = tool_results.len(),
            "runtime.run.max_steps_exceeded"
        );

        Err(AgentError::Message(format!(
            "超过最大步数限制（max_steps={}），仍未结束工具调用流程",
            self.max_steps
        )))
    }
}
