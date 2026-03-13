//! agentkit 的最小运行时（runtime）示例。
//!
//! 该 crate 的职责是提供“编排层”的实现（如何调用 provider、如何循环、如何调用工具等）。
//! 目前仅提供一个最小的 `SimpleAgent`，用于演示如何基于 `agentkit-core` 的 trait 进行组装。

use std::{collections::HashMap, sync::Arc};

use agentkit_core::{
    agent::{Agent, types::{AgentInput, AgentOutput}},
    error::AgentError,
    provider::{LlmProvider, types::{ChatMessage, ChatRequest, Role}},
    tool::{Tool, types::{ToolCall, ToolDefinition, ToolResult}},
};
use async_trait::async_trait;
use serde_json::Value;

/// 一个最简 Agent：仅调用一次 `LlmProvider::chat`，不包含 tool loop。
pub struct SimpleAgent<P> {
    /// LLM provider 实例。
    provider: P,
    /// 可选系统提示词。
    system_prompt: Option<String>,
}

/// Tool 注册表。
///
/// 用途：把一组 `Tool` 按名称组织起来，便于在 tool-calling loop 中按 `ToolCall.name` 查找。
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// 创建一个空注册表。
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册一个工具。
    ///
    /// 注意：如果同名工具重复注册，后者会覆盖前者。
    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
        self
    }

    /// 获取工具定义列表（用于发给 provider 进行 tool/function 注册）。
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            })
            .collect()
    }

    /// 按名称查找工具。
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}

/// 支持工具调用的最小 Agent。
///
/// 该 Agent 的核心逻辑：
/// 1. 调用 `provider.chat()` 让模型生成回复或请求工具（`tool_calls`）
/// 2. 如果模型请求工具：执行工具并把结果追加回 messages
/// 3. 重复 1-2 直到模型停止请求工具或达到步数上限
pub struct ToolCallingAgent<P> {
    /// LLM provider 实例。
    provider: P,
    /// 可选系统提示词。
    system_prompt: Option<String>,
    /// 可用工具注册表。
    tools: ToolRegistry,
    /// 最大循环步数，避免模型陷入无限调用。
    max_steps: usize,
}

impl<P> ToolCallingAgent<P> {
    /// 创建一个支持工具调用的 Agent。
    pub fn new(provider: P, tools: ToolRegistry) -> Self {
        Self {
            provider,
            system_prompt: None,
            tools,
            max_steps: 8,
        }
    }

    /// 设置系统提示词（会在运行时插入到 messages 开头）。
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// 设置最大循环步数。
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// 执行单个工具调用，并返回 `ToolResult`。
    async fn execute_tool_call(&self, call: &ToolCall) -> Result<ToolResult, AgentError> {
        let tool = self.tools.get(&call.name).ok_or_else(|| {
            AgentError::Message(format!("未找到工具：{} (tool_call_id={})", call.name, call.id))
        })?;

        let output = tool
            .call(call.input.clone())
            .await
            .map_err(|e| AgentError::Message(e.to_string()))?;

        Ok(ToolResult {
            tool_call_id: call.id.clone(),
            output,
        })
    }

    /// 将工具结果转换为“工具消息”追加回对话历史。
    fn tool_result_to_message(result: &ToolResult, tool_name: &str) -> ChatMessage {
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
}

impl<P> SimpleAgent<P> {
    /// 创建一个 `SimpleAgent`。
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            system_prompt: None,
        }
    }

    /// 设置系统提示词（会在运行时插入到 messages 开头）。
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }
}

#[async_trait]
impl<P> Agent for SimpleAgent<P>
where
    P: LlmProvider,
{
    /// 执行一次对话：把输入 messages 发送给 provider 并返回最终输出。
    async fn run(&self, mut input: AgentInput) -> Result<AgentOutput, AgentError> {
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

        let request = ChatRequest {
            messages: input.messages,
            model: None,
            tools: None,
            temperature: None,
            max_tokens: None,
            metadata: input.metadata,
        };

        let resp = self
            .provider
            .chat(request)
            .await
            .map_err(|e| AgentError::Message(e.to_string()))?;

        Ok(AgentOutput {
            message: resp.message,
            tool_results: vec![],
        })
    }
}

#[async_trait]
impl<P> Agent for ToolCallingAgent<P>
where
    P: LlmProvider,
{
    /// 执行带工具调用的对话循环。
    async fn run(&self, mut input: AgentInput) -> Result<AgentOutput, AgentError> {
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

        // tool 定义会提供给 provider，用于 function-calling/tool-calling 注册。
        let tool_defs = self.tools.definitions();

        let mut messages = input.messages;
        let mut tool_results: Vec<ToolResult> = Vec::new();

        for _step in 0..self.max_steps {
            let request = ChatRequest {
                messages: messages.clone(),
                model: None,
                tools: Some(tool_defs.clone()),
                temperature: None,
                max_tokens: None,
                metadata: input.metadata.clone(),
            };

            let resp = self
                .provider
                .chat(request)
                .await
                .map_err(|e| AgentError::Message(e.to_string()))?;

            // 追加 assistant 回复到对话历史。
            messages.push(resp.message.clone());

            // 如果没有工具调用，则直接返回最终消息。
            if resp.tool_calls.is_empty() {
                return Ok(AgentOutput {
                    message: resp.message,
                    tool_results,
                });
            }

            // 执行工具调用，并将结果追加回 messages。
            for call in resp.tool_calls.iter() {
                let result = self.execute_tool_call(call).await?;
                tool_results.push(result.clone());

                let tool_msg = Self::tool_result_to_message(&result, &call.name);
                messages.push(tool_msg);
            }
        }

        Err(AgentError::Message(format!(
            "超过最大步数限制（max_steps={}），仍未结束工具调用流程",
            self.max_steps
        )))
    }
}
