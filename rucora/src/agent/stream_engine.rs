//! 流式执行引擎
//!
//! # 概述
//!
//! `StreamEngine` 专注于流式输出控制（`StreamEngine`），从 `DefaultExecution`
//! 中独立出来，遵循单一职责原则。`DefaultExecution` 通过委托的方式组合本组件。

use std::sync::Arc;

use async_stream::try_stream;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tracing::info;

use rucora_core::agent::{AgentError, AgentInput, ToolCallRecord};
use rucora_core::channel::types::{ChannelEvent, ErrorEvent, TokenDeltaEvent};
use rucora_core::channel::ChannelObserver;
use rucora_core::provider::types::{ChatMessage, ChatRequest, LlmParams};
use rucora_core::provider::LlmProvider;
use rucora_core::tool::types::{ToolCall, ToolResult};

use crate::agent::loop_detector::{LoopDetectionResult, LoopDetector, LoopDetectorConfig};
use crate::agent::policy::ToolPolicy;
use crate::agent::tool_call_config::{ToolCallEnhancedConfig, ToolCallEnhancedRuntime};
use crate::agent::tool_execution::{
    execute_tool_call_enhanced, tool_result_to_message,
};
use crate::agent::tool_registry::ToolRegistry;
use crate::conversation::ConversationManager;
use crate::middleware::MiddlewareChain;

/// 流式执行引擎。
///
/// 封装流式输出所需的全部依赖，提供统一的流式执行入口。
#[derive(Clone)]
pub struct StreamEngine {
    /// LLM Provider
    pub(crate) provider: Arc<dyn LlmProvider>,
    /// Agent 级模型覆盖；为空时使用 Provider 默认模型。
    pub(crate) model: Option<String>,
    /// 系统提示词
    pub(crate) system_prompt: Option<String>,
    /// 工具注册表
    pub(crate) tools: ToolRegistry,
    /// 工具策略
    pub(crate) policy: Arc<dyn ToolPolicy>,
    /// 观测器
    pub(crate) observer: Arc<dyn ChannelObserver>,
    /// 最大执行步数
    pub(crate) max_steps: usize,
    /// 对话管理器（可选）
    pub(crate) conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// 循环检测器配置
    pub(crate) loop_detector_config: LoopDetectorConfig,
    /// LLM 请求参数（temperature、top_p 等）
    pub(crate) llm_params: LlmParams,
    /// 中间件链
    pub(crate) middleware_chain: MiddlewareChain,
    /// 工具调用增强配置（重试、超时、熔断、缓存等）
    pub(crate) enhanced_config: ToolCallEnhancedConfig,
    /// 工具调用增强运行时状态
    pub(crate) enhanced_runtime: ToolCallEnhancedRuntime,
}

impl StreamEngine {
    /// 创建新的流式执行引擎。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        model: Option<String>,
        system_prompt: Option<String>,
        tools: ToolRegistry,
        policy: Arc<dyn ToolPolicy>,
        observer: Arc<dyn ChannelObserver>,
        max_steps: usize,
        conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
        loop_detector_config: LoopDetectorConfig,
        llm_params: LlmParams,
        middleware_chain: MiddlewareChain,
        enhanced_config: ToolCallEnhancedConfig,
        enhanced_runtime: ToolCallEnhancedRuntime,
    ) -> Self {
        Self {
            provider,
            model,
            system_prompt,
            tools,
            policy,
            observer,
            max_steps,
            conversation_manager,
            loop_detector_config,
            llm_params,
            middleware_chain,
            enhanced_config,
            enhanced_runtime,
        }
    }

    /// 流式运行（简单模式）。
    ///
    /// 逐步执行对话循环，并将每个事件（TokenDelta、Message、ToolCall、ToolResult）通过
    /// 事件流产出。适用于所有不需要分块-合并逻辑的 Agent。
    pub fn run_stream(
        &self,
        input: AgentInput,
    ) -> BoxStream<'static, Result<ChannelEvent, AgentError>> {
        let provider = self.provider.clone();
        let tools = self.tools.clone();
        let policy = self.policy.clone();
        let observer = self.observer.clone();
        let max_steps = self.max_steps;
        let model = self.model.clone();
        let system_prompt = self.system_prompt.clone();
        let llm_params = self.llm_params.clone();
        let loop_detector_config = self.loop_detector_config.clone();
        let conversation_manager = self.conversation_manager.clone();
        let middleware_chain = self.middleware_chain.clone();
        let enhanced_config = self.enhanced_config.clone();
        let enhanced_runtime = self.enhanced_runtime.clone();

        let stream = try_stream! {
            let mut messages = Vec::new();

            // 添加系统提示词
            if let Some(ref prompt) = system_prompt {
                messages.push(ChatMessage::system(prompt.clone()));
            }

            // 添加用户消息
            messages.push(ChatMessage::user(input.text.clone()));

            // 保存用户消息到会话管理器
            if let Some(ref conv_arc) = conversation_manager {
                let mut conv = conv_arc.lock().await;
                conv.add_user_message(input.text.clone());
            }

            let tool_defs = tools.definitions();
            let mut tool_call_records: Vec<ToolCallRecord> = Vec::new();
            let mut loop_detector = LoopDetector::new(loop_detector_config);

            info!(
                tool_count = tool_defs.len(),
                max_steps,
                "stream_execution.start"
            );

            for step in 0..max_steps {
                let mut request = ChatRequest {
                    messages: messages.clone(),
                    model: model.clone(),
                    tools: if !tool_defs.is_empty() { Some(tool_defs.clone()) } else { None },
                    ..Default::default()
                };
                llm_params.apply_to(&mut request);

                let mut assistant_text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                let mut s = match provider.stream_chat(request) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = AgentError::ProviderError { source: e };
                        let ev = ChannelEvent::Error(ErrorEvent {
                            kind: "provider".to_string(),
                            message: err.to_string(),
                            data: Some(json!({"step": step})),
                        });
                        observer.on_event(ev.clone());
                        yield ev;
                        break;
                    }
                };

                while let Some(item) = s.next().await {
                    let chunk = match item {
                        Ok(v) => v,
                        Err(e) => {
                            let err = AgentError::ProviderError { source: e };
                            let ev = ChannelEvent::Error(ErrorEvent {
                                kind: "provider".to_string(),
                                message: err.to_string(),
                                data: Some(json!({"step": step})),
                            });
                            observer.on_event(ev.clone());
                            yield ev;
                            break;
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

                let assistant_msg = if !tool_calls.is_empty() {
                    ChatMessage::assistant_with_tool_calls(assistant_text, tool_calls.clone())
                } else {
                    ChatMessage::assistant(assistant_text)
                };

                messages.push(assistant_msg.clone());
                let ev = ChannelEvent::Message(assistant_msg);
                observer.on_event(ev.clone());
                yield ev;

                if tool_calls.is_empty() {
                    // 保存助手回复到会话管理器
                    if let Some(ref conv_arc) = conversation_manager {
                        let mut conv = conv_arc.lock().await;
                        conv.add_assistant_message(
                            messages.last()
                                .map(|m| m.content_text().to_string())
                                .unwrap_or_default()
                        );
                        conv.add_tool_call_records(tool_call_records.clone());
                    }
                    break;
                }

                info!(
                    step,
                    tool_call_count = tool_calls.len(),
                    "stream_execution.tool_calls"
                );

                // 手动实现工具执行（闭包中无法访问 self），使用增强配置
                let mut results: Vec<(usize, ToolResult)> = Vec::new();
                for (idx, call) in tool_calls.iter().enumerate() {
                    let r = execute_tool_call_enhanced(
                        &tools,
                        &policy,
                        &observer,
                        call,
                        &middleware_chain,
                        &enhanced_config,
                        &enhanced_runtime,
                    )
                    .await
                    .map_err(|e| AgentError::Message(format!("工具执行失败：{e}")))?;

                    let detection = loop_detector.record(&call.name, &call.input, &r.output.to_string());
                    match detection {
                        LoopDetectionResult::Ok => {
                            tool_call_records.push(ToolCallRecord {
                                name: call.name.clone(),
                                tool_call_id: r.tool_call_id.clone(),
                                input: call.input.clone(),
                                result: r.output.clone(),
                            });
                            results.push((idx, r));
                        }
                        LoopDetectionResult::Warning(msg) => {
                            tracing::warn!(tool = %call.name, "{}", msg);
                            let system_msg = ChatMessage::system(msg);
                            messages.push(system_msg.clone());
                            let ev = ChannelEvent::Message(system_msg);
                            observer.on_event(ev.clone());
                            yield ev;
                            results.push((idx, r));
                        }
                        LoopDetectionResult::Block(msg) => {
                            tracing::warn!(tool = %call.name, "{}", msg);
                            let blocked = ToolResult {
                                tool_call_id: r.tool_call_id.clone(),
                                output: Value::String(msg),
                                ..Default::default()
                            };
                            tool_call_records.push(ToolCallRecord {
                                name: call.name.clone(),
                                tool_call_id: r.tool_call_id.clone(),
                                input: call.input.clone(),
                                result: blocked.output.clone(),
                            });
                            results.push((idx, blocked));
                        }
                        LoopDetectionResult::Break(msg) => {
                            tracing::error!(tool = %call.name, "{}", msg);
                            Err(AgentError::Message(format!("[LoopDetector] {msg}")))?;
                        }
                    }
                }

                for (idx, result) in &results {
                    let call = &tool_calls[*idx];

                    let ev = ChannelEvent::ToolCall(call.clone());
                    observer.on_event(ev.clone());
                    yield ev;

                    let ev = ChannelEvent::ToolResult(result.clone());
                    observer.on_event(ev.clone());
                    yield ev;

                    let tool_msg = tool_result_to_message(result, &call.name);
                    messages.push(tool_msg);
                }
            }

            info!("stream_execution.done");
        };

        Box::pin(stream)
    }

    /// 高层流式 API：运行并返回拼接后的最终文本。
    ///
    /// 此方法消费事件流，自动拼接 `TokenDelta` 为完整文本返回。
    /// 适用于只需要最终文本、不需要逐帧处理的场景。
    ///
    /// # 参数
    ///
    /// - `input`: 用户输入
    ///
    /// # 返回
    ///
    /// 返回拼接后的完整文本内容。如果遇到错误则返回错误。
    pub async fn run_stream_text(
        &self,
        input: AgentInput,
    ) -> Result<String, AgentError> {
        use rucora_core::channel::types::ChannelEvent;

        let mut stream = self.run_stream(input);
        let mut text = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ChannelEvent::TokenDelta(delta) => {
                    text.push_str(&delta.delta);
                }
                ChannelEvent::Error(err) => {
                    return Err(AgentError::Message(err.message));
                }
                _ => {}
            }
        }

        Ok(text)
    }
}
