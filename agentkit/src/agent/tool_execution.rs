//! 工具执行模块
//!
//! 负责执行工具调用，包括策略检查、观测器通知、重试、超时、熔断、缓存等。

use std::sync::Arc;

use agentkit_core::channel::ChannelObserver;
use agentkit_core::error::{AgentError, ToolError};
use agentkit_core::provider::types::{ChatMessage, Role};
use agentkit_core::tool::types::{DEFAULT_TOOL_OUTPUT_MAX_BYTES, ToolCall, ToolResult};
use serde_json::{Value, json};
use tracing::{debug, info, warn};

use crate::agent::policy::{ToolCallContext, ToolPolicy};
use crate::agent::tool_call_config::{
    ToolCallEnhancedConfig, ToolCallEnhancedRuntime,
    TimeoutConfig,
};
use crate::agent::tool_registry::ToolRegistry;
use crate::middleware::MiddlewareChain;

// ========== 工具函数 ==========

fn truncate_utf8_to_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s[..end].to_string();
    out.push_str("\n... [output truncated]");
    out
}

fn apply_output_limit(payload: Value, max_bytes: usize) -> Value {
    let serialized = payload.to_string();
    let truncated = serialized.len() > max_bytes;
    let limited_payload = if truncated {
        Value::String(truncate_utf8_to_bytes(&serialized, max_bytes))
    } else {
        payload
    };

    let mut obj = match limited_payload {
        Value::Object(map) => Value::Object(map),
        other => json!({"value": other}),
    };

    if let Some(map) = obj.as_object_mut() {
        map.insert("truncated".to_string(), Value::Bool(truncated));
        map.insert("max_bytes".to_string(), json!(max_bytes));
    }
    obj
}

pub(crate) async fn execute_tool_call_with_policy_and_observer(
    tools: &ToolRegistry,
    policy: &Arc<dyn ToolPolicy>,
    observer: &Arc<dyn ChannelObserver>,
    call: &ToolCall,
) -> Result<ToolResult, AgentError> {
    // 调用带中间件的版本（无中间件）
    execute_tool_call_with_middleware(tools, policy, observer, call, &MiddlewareChain::new()).await
}

pub(crate) async fn execute_tool_call_with_middleware(
    tools: &ToolRegistry,
    policy: &Arc<dyn ToolPolicy>,
    observer: &Arc<dyn ChannelObserver>,
    call: &ToolCall,
    middleware_chain: &MiddlewareChain,
) -> Result<ToolResult, AgentError> {
    // 创建可变副本用于中间件处理
    let mut call_mut = call.clone();

    // 执行工具调用前中间件钩子
    middleware_chain
        .process_tool_call_before(&mut call_mut)
        .await
        .map_err(|e| AgentError::Message(format!("工具调用前中间件处理失败：{e}")))?;

    let input_str = call_mut.input.to_string();
    let input_len = input_str.len();
    let input_preview = if input_len <= 800 {
        input_str.clone()
    } else {
        format!("{}...<truncated:{}>", &input_str[..800], input_len)
    };

    observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
        agentkit_core::channel::types::DebugEvent {
            message: "tool_call.start".to_string(),
            data: Some(json!({
                "tool_name": call_mut.name.clone(),
                "tool_call_id": call_mut.id.clone(),
                "input_len": input_len
            })),
        },
    ));

    info!(
        tool.name = %call_mut.name,
        tool.call_id = %call_mut.id,
        tool.input_len = input_len,
        "tool_call.execute.start"
    );
    debug!(
        tool.name = %call_mut.name,
        tool.call_id = %call_mut.id,
        tool.input = %input_preview,
        "tool_call.execute.input"
    );

    let ctx = ToolCallContext {
        tool_call: call_mut.clone(),
    };

    if let Err(e) = policy.check(&ctx).await {
        match &e {
            ToolError::PolicyDenied { rule_id, reason } => {
                observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
                    agentkit_core::channel::types::DebugEvent {
                        message: "tool_call.denied".to_string(),
                        data: Some(json!({
                            "tool_name": call.name.clone(),
                            "tool_call_id": call.id.clone(),
                            "rule_id": rule_id,
                            "reason": reason
                        })),
                    },
                ));
                let out = apply_output_limit(
                    json!({
                        "ok": false,
                        "error": {
                            "kind": "policy_denied",
                            "rule_id": rule_id,
                            "reason": reason
                        }
                    }),
                    DEFAULT_TOOL_OUTPUT_MAX_BYTES,
                );
                debug!(
                    tool.name = %call.name,
                    tool.call_id = %call.id,
                    policy.rule_id = %rule_id,
                    "tool_call.execute.denied"
                );
                return Ok(ToolResult {
                    tool_call_id: call.id.clone(),
                    output: out,
                });
            }
            _ => {
                observer.on_event(agentkit_core::channel::types::ChannelEvent::Error(
                    agentkit_core::channel::types::ErrorEvent {
                        kind: "policy".to_string(),
                        message: e.to_string(),
                        data: Some(json!({
                            "tool_name": call.name.clone(),
                            "tool_call_id": call.id.clone()
                        })),
                    },
                ));
                let out = apply_output_limit(
                    json!({
                        "ok": false,
                        "error": {
                            "kind": "policy_error",
                            "message": e.to_string()
                        }
                    }),
                    DEFAULT_TOOL_OUTPUT_MAX_BYTES,
                );
                debug!(
                    tool.name = %call.name,
                    tool.call_id = %call.id,
                    error = %e.to_string(),
                    "tool_call.execute.policy_error"
                );
                return Ok(ToolResult {
                    tool_call_id: call.id.clone(),
                    output: out,
                });
            }
        }
    }

    let start = std::time::Instant::now();

    let tool = tools.get(&call.name).ok_or_else(|| {
        AgentError::Message(format!(
            "未找到工具：{} (tool_call_id={})",
            call.name, call.id
        ))
    })?;

    let tool_output = match tool.call(call.input.clone()).await {
        Ok(v) => json!({"ok": true, "output": v}),
        Err(ToolError::PolicyDenied { rule_id, reason }) => {
            observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
                agentkit_core::channel::types::DebugEvent {
                    message: "tool_call.denied".to_string(),
                    data: Some(json!({
                        "tool_name": call.name.clone(),
                        "tool_call_id": call.id.clone(),
                        "rule_id": rule_id,
                        "reason": reason
                    })),
                },
            ));
            json!({
                "ok": false,
                "error": {"kind": "policy_denied", "rule_id": rule_id, "reason": reason}
            })
        }
        Err(e) => {
            observer.on_event(agentkit_core::channel::types::ChannelEvent::Error(
                agentkit_core::channel::types::ErrorEvent {
                    kind: "tool".to_string(),
                    message: e.to_string(),
                    data: Some(json!({
                        "tool_name": call.name.clone(),
                        "tool_call_id": call.id.clone()
                    })),
                },
            ));
            json!({
                "ok": false,
                "error": {"kind": "tool_error", "message": e.to_string()}
            })
        }
    };

    let tool_output = apply_output_limit(tool_output, DEFAULT_TOOL_OUTPUT_MAX_BYTES);

    let output_preview = {
        const MAX: usize = 1200;
        let s = tool_output.to_string();
        if s.len() <= MAX {
            s
        } else {
            // 在字符边界处截断，避免 UTF-8 错误
            // 找到第一个超过 MAX 字节的位置的前一个字符边界
            let truncation_point = s
                .char_indices()
                .find(|(idx, _)| *idx >= MAX)
                .map_or(s.len(), |(idx, _)| idx);
            
            format!("{}...<truncated:{}>", &s[..truncation_point], s.len())
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let output_len = tool_output.to_string().len();

    observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
        agentkit_core::channel::types::DebugEvent {
            message: "tool_call.done".to_string(),
            data: Some(json!({
                "tool_name": call.name.clone(),
                "tool_call_id": call.id.clone(),
                "output_len": output_len,
                "elapsed_ms": elapsed_ms
            })),
        },
    ));

    info!(
        tool.name = %call.name,
        tool.call_id = %call.id,
        tool.output_len = output_len,
        tool.elapsed_ms = elapsed_ms,
        "tool_call.execute.done"
    );

    debug!(
        tool.name = %call.name,
        tool.call_id = %call.id,
        tool.output = %output_preview,
        "tool_call.execute.output"
    );

    // 构建结果
    let mut result = ToolResult {
        tool_call_id: call.id.clone(),
        output: tool_output,
    };

    // 执行工具调用后中间件钩子
    middleware_chain
        .process_tool_call_after(&mut result)
        .await
        .map_err(|e| AgentError::Message(format!("工具调用后中间件处理失败：{e}")))?;

    Ok(result)
}

/// 带增强配置（重试 + 超时 + 熔断 + 缓存）的工具执行函数。
///
/// 在现有 `execute_tool_call_with_middleware` 基础上增加可靠性增强。
/// 所有增强特性默认关闭，通过 `ToolCallEnhancedConfig` 启用。
pub(crate) async fn execute_tool_call_enhanced(
    tools: &ToolRegistry,
    policy: &Arc<dyn ToolPolicy>,
    observer: &Arc<dyn ChannelObserver>,
    call: &ToolCall,
    middleware_chain: &MiddlewareChain,
    enhanced_config: &ToolCallEnhancedConfig,
    runtime: &ToolCallEnhancedRuntime,
) -> Result<ToolResult, AgentError> {
    let tool_name = call.name.as_str();

    // --- 缓存命中检查 ---
    if enhanced_config.cache.enabled {
        if let Some(cached) = runtime.cache.get(tool_name, &call.input).await {
            observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
                agentkit_core::channel::types::DebugEvent {
                    message: "tool_call.cache_hit".to_string(),
                    data: Some(json!({
                        "tool_name": tool_name,
                        "tool_call_id": call.id,
                    })),
                },
            ));
            debug!(tool.name = %tool_name, tool.call_id = %call.id, "tool_call.cache_hit");
            return Ok(ToolResult {
                tool_call_id: call.id.clone(),
                output: cached,
            });
        }
    }

    // --- 熔断器检查 ---
    if enhanced_config.circuit_breaker.enabled
        && !runtime
            .circuit_breaker
            .can_pass(tool_name, &enhanced_config.circuit_breaker)
            .await
    {
        let state = runtime.circuit_breaker.get_state(tool_name).await;
        observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
            agentkit_core::channel::types::DebugEvent {
                message: "tool_call.circuit_breaker_open".to_string(),
                data: Some(json!({
                    "tool_name": tool_name,
                    "tool_call_id": call.id,
                    "circuit_state": format!("{:?}", state),
                })),
            },
        ));
        warn!(
            tool.name = %tool_name,
            tool.call_id = %call.id,
            "tool_call.circuit_breaker_open"
        );
        let out = apply_output_limit(
            json!({
                "ok": false,
                "error": {
                    "kind": "circuit_breaker_open",
                    "message": format!("工具 '{tool_name}' 熔断器已开启，调用被拒绝"),
                    "circuit_state": format!("{:?}", state),
                }
            }),
            DEFAULT_TOOL_OUTPUT_MAX_BYTES,
        );
        return Ok(ToolResult {
            tool_call_id: call.id.clone(),
            output: out,
        });
    }

    // --- 重试循环 ---
    let max_retries = enhanced_config.retry.max_retries;
    let mut attempt = 0u32;

    loop {
        let result = execute_single_with_timeout(
            tools,
            policy,
            observer,
            call,
            middleware_chain,
            &enhanced_config.timeout,
        )
        .await;

        match result {
            Ok(ref tool_result) => {
                // 判断是否是逻辑上的失败（ok=false）
                let is_error = tool_result
                    .output
                    .get("ok")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
                    == false;

                if is_error
                    && attempt < max_retries
                    && enhanced_config.retry.should_retry(&tool_result.output)
                {
                    let delay = enhanced_config.retry.delay_for(attempt);
                    observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
                        agentkit_core::channel::types::DebugEvent {
                            message: "tool_call.retry".to_string(),
                            data: Some(json!({
                                "tool_name": tool_name,
                                "tool_call_id": call.id,
                                "attempt": attempt + 1,
                                "max_retries": max_retries,
                                "delay_ms": delay.as_millis(),
                            })),
                        },
                    ));
                    warn!(
                        tool.name = %tool_name,
                        attempt = attempt + 1,
                        max_retries,
                        delay_ms = delay.as_millis(),
                        "tool_call.retry"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    continue;
                }

                // 记录熔断器结果
                if enhanced_config.circuit_breaker.enabled {
                    if is_error {
                        runtime
                            .circuit_breaker
                            .record_failure(tool_name, &enhanced_config.circuit_breaker)
                            .await;
                    } else {
                        runtime.circuit_breaker.record_success(tool_name).await;
                    }
                }

                // 写入缓存（仅成功结果）
                if enhanced_config.cache.enabled && !is_error {
                    let ttl = enhanced_config.cache.get_ttl(tool_name);
                    runtime
                        .cache
                        .set(
                            tool_name,
                            &call.input,
                            tool_result.output.clone(),
                            ttl,
                            enhanced_config.cache.max_entries,
                        )
                        .await;
                }

                return result;
            }
            Err(ref e) => {
                // 执行框架错误的重试
                if attempt < max_retries {
                    let delay = enhanced_config.retry.delay_for(attempt);
                    observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
                        agentkit_core::channel::types::DebugEvent {
                            message: "tool_call.retry".to_string(),
                            data: Some(json!({
                                "tool_name": tool_name,
                                "tool_call_id": call.id,
                                "attempt": attempt + 1,
                                "max_retries": max_retries,
                                "delay_ms": delay.as_millis(),
                                "error": e.to_string(),
                            })),
                        },
                    ));
                    warn!(
                        tool.name = %tool_name,
                        attempt = attempt + 1,
                        max_retries,
                        delay_ms = delay.as_millis(),
                        error = %e,
                        "tool_call.retry"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;

                    // 记录熔断器失败
                    if enhanced_config.circuit_breaker.enabled {
                        runtime
                            .circuit_breaker
                            .record_failure(tool_name, &enhanced_config.circuit_breaker)
                            .await;
                    }

                    continue;
                }

                // 记录最终失败到熔断器
                if enhanced_config.circuit_breaker.enabled {
                    runtime
                        .circuit_breaker
                        .record_failure(tool_name, &enhanced_config.circuit_breaker)
                        .await;
                }

                return result;
            }
        }
    }
}

/// 执行单次工具调用（含超时控制）
async fn execute_single_with_timeout(
    tools: &ToolRegistry,
    policy: &Arc<dyn ToolPolicy>,
    observer: &Arc<dyn ChannelObserver>,
    call: &ToolCall,
    middleware_chain: &MiddlewareChain,
    timeout_config: &TimeoutConfig,
) -> Result<ToolResult, AgentError> {
    let timeout = timeout_config.get_timeout(call.name.as_str());

    match timeout {
        Some(duration) => {
            match tokio::time::timeout(
                duration,
                execute_tool_call_with_middleware(tools, policy, observer, call, middleware_chain),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    let tool_name = call.name.as_str();
                    observer.on_event(agentkit_core::channel::types::ChannelEvent::Debug(
                        agentkit_core::channel::types::DebugEvent {
                            message: "tool_call.timeout".to_string(),
                            data: Some(json!({
                                "tool_name": tool_name,
                                "tool_call_id": call.id,
                                "timeout_ms": duration.as_millis(),
                            })),
                        },
                    ));
                    warn!(
                        tool.name = %tool_name,
                        tool.call_id = %call.id,
                        timeout_ms = duration.as_millis(),
                        "tool_call.timeout"
                    );
                    let out = apply_output_limit(
                        json!({
                            "ok": false,
                            "error": {
                                "kind": "timeout",
                                "message": format!("工具 '{tool_name}' 执行超时（{}ms）", duration.as_millis()),
                                "transient": true,
                            }
                        }),
                        DEFAULT_TOOL_OUTPUT_MAX_BYTES,
                    );
                    Ok(ToolResult {
                        tool_call_id: call.id.clone(),
                        output: out,
                    })
                }
            }
        }
        None => {
            execute_tool_call_with_middleware(tools, policy, observer, call, middleware_chain).await
        }
    }
}

pub(crate) fn tool_result_to_message(result: &ToolResult, tool_name: &str) -> ChatMessage {
    let payload = Value::Object(
        [
            (
                "tool_call_id".to_string(),
                Value::String(result.tool_call_id.clone()),
            ),
            ("output".to_string(), result.output.clone()),
        ]
        .into_iter()
        .collect(),
    );

    ChatMessage {
        role: Role::Tool,
        content: payload.to_string(),
        name: Some(tool_name.to_string()),
    }
}
