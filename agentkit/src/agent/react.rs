//! ReActAgent - 推理 + 行动 Agent
//!
//! # 概述
//!
//! ReActAgent 实现显式的 ReAct（Reason + Act）循环：
//! 1. **Think**（思考）：分析问题，规划步骤
//! 2. **Act**（行动）：执行工具调用
//! 3. **Observe**（观察）：分析工具结果
//! 4. 循环直到完成任务
//!
//! # 适用场景
//!
//! - 需要多步推理的复杂任务
//! - 需要分析和规划的任务
//! - 代码分析、项目调研等
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::agent::ReActAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::tools::{ShellTool, FileReadTool, HttpTool};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ReActAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是一个善于推理的助手")
//!     .tools(vec![ShellTool, FileReadTool, HttpTool])
//!     .max_steps(15)
//!     .build();
//!
//! // 复杂任务：先分析，再分步执行
//! let output = agent.run("帮我分析这个项目的代码结构，找出所有 Rust 文件并统计行数").await?;
//! # Ok(())
//! # }
//! ```

use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::tool::Tool;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agent::ToolRegistry;
use crate::agent::execution::DefaultExecution;
use crate::conversation::ConversationManager;

/// ReActAgent - 推理 + 行动 Agent
///
/// 特点：
/// - 显式的思考 - 行动 - 观察循环
/// - 每一步都先思考再行动
/// - 适合多步推理任务
pub struct ReActAgent<P> {
    /// LLM Provider
    #[allow(dead_code)]
    provider: Arc<P>,
    /// 默认使用的模型
    #[allow(dead_code)]
    model: String,
    /// 系统提示词
    #[allow(dead_code)]
    system_prompt: Option<String>,
    /// 工具注册表
    #[allow(dead_code)]
    tools: ToolRegistry,
    /// 最大步骤数
    #[allow(dead_code)]
    max_steps: usize,
    /// 对话管理器（可选）
    #[allow(dead_code)]
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for ReActAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // ReAct 核心：显式思考步骤
        if context.step == 0 {
            // 第一步：先思考，不工具调用
            AgentDecision::Chat {
                request: self._build_react_prompt(context, "think"),
            }
        } else if !context.tool_results.is_empty() {
            // 有工具结果：观察后继续思考
            AgentDecision::Chat {
                request: self._build_react_prompt(context, "observe"),
            }
        } else {
            // 正常：决定行动
            AgentDecision::Chat {
                request: self._build_react_prompt(context, "act"),
            }
        }
    }

    fn name(&self) -> &str {
        "react_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("ReAct Agent，显式的推理 + 行动循环")
    }

    /// 运行 Agent（覆盖默认实现，使用 DefaultExecution）
    async fn run(
        &self,
        input: AgentInput,
    ) -> Result<AgentOutput, agentkit_core::agent::AgentError> {
        self.execution.run(self, input).await
    }

    /// 流式运行
    fn run_stream(
        &self,
        input: AgentInput,
    ) -> futures_util::stream::BoxStream<
        'static,
        Result<agentkit_core::channel::types::ChannelEvent, agentkit_core::agent::AgentError>,
    > {
        self.execution.run_stream_simple(input)
    }
}

impl<P> ReActAgent<P>
where
    P: LlmProvider,
{
    /// 创建新的构建器
    pub fn builder() -> ReActAgentBuilder<P> {
        ReActAgentBuilder::new()
    }

    /// 构建 ReAct 提示词
    fn _build_react_prompt(&self, context: &AgentContext, phase: &str) -> ChatRequest {
        let prompt = match phase {
            "think" => format!(
                "请分析问题：{}\n\
                 \n\
                 思考步骤：\n\
                 1. 理解用户需求\n\
                 2. 确定需要什么信息\n\
                 3. 规划使用哪些工具\n\
                 \n\
                 可用工具：{:?}\n\
                 \n\
                 请详细分析并规划步骤。",
                context.input.text(),
                self.tools.tool_names()
            ),
            "act" => format!(
                "基于以上思考，请选择合适的工具行动。\n\
                 \n\
                 可用工具：{:?}\n\
                 \n\
                 如果需要调用工具，请使用工具调用格式。",
                self.tools.tool_names()
            ),
            "observe" => format!(
                "观察工具执行结果，分析是否完成任务。\n\
                 \n\
                 如果完成，给出最终答案；否则继续思考下一步。\n\
                 \n\
                 当前步骤：{}/{}",
                context.step, self.max_steps
            ),
            _ => unreachable!(),
        };

        // 构建消息历史
        let mut messages = context.messages.clone();

        // 添加系统提示词
        if let Some(ref sys_prompt) = self.system_prompt {
            if messages.is_empty() || messages.first().map(|m| &m.role) != Some(&Role::System) {
                messages.insert(0, ChatMessage::system(sys_prompt.clone()));
            }
        }

        // 添加 ReAct 提示词
        messages.push(ChatMessage::user(prompt));

        ChatRequest {
            messages,
            model: Some(self.model.clone()),
            tools: Some(self.tools.definitions()),
            temperature: Some(0.7),
            ..Default::default()
        }
    }

    /// 获取工具列表
    pub fn tools(&self) -> Vec<&str> {
        self.tools
            .tool_names()
            .into_iter()
            .map(|s| s.as_str())
            .collect()
    }
}

/// ReActAgent 构建器
pub struct ReActAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: ToolRegistry,
    max_steps: usize,
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
}

impl<P> ReActAgentBuilder<P> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            tools: ToolRegistry::new(),
            max_steps: 15, // ReAct 通常需要更多步骤
            conversation_manager: None,
        }
    }
}

impl<P> ReActAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    /// 设置 Provider（必需）
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 设置系统提示词
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置默认模型（必需）
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 注册工具
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools = self.tools.register(tool);
        self
    }

    /// 注册多个工具
    pub fn tools<I, T>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Tool + 'static,
    {
        for tool in tools {
            self.tools = self.tools.register(tool);
        }
        self
    }

    /// 设置最大步骤数
    pub fn max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    /// 启用对话历史管理
    pub fn with_conversation(mut self, enabled: bool) -> Self {
        if enabled {
            let mut conv = ConversationManager::new();
            if let Some(ref prompt) = self.system_prompt {
                conv = conv.with_system_prompt(prompt.clone());
            }
            self.conversation_manager = Some(Arc::new(Mutex::new(conv)));
        } else {
            self.conversation_manager = None;
        }
        self
    }

    /// 构建 Agent
    ///
    /// # Panics
    ///
    /// 如果没有设置 `provider` 或 `model`，此方法会 panic。
    pub fn build(self) -> ReActAgent<P> {
        let provider = self.provider.expect("Provider is required");
        let model = self.model.expect("Model is required");

        // 创建执行能力
        let provider_arc = Arc::new(provider);
        let execution =
            DefaultExecution::new(provider_arc.clone(), model.clone(), self.tools.clone())
                .with_system_prompt_opt(self.system_prompt.clone())
                .with_max_steps(self.max_steps)
                .with_conversation_manager(self.conversation_manager.clone());

        ReActAgent {
            provider: provider_arc,
            model,
            system_prompt: self.system_prompt,
            tools: self.tools,
            max_steps: self.max_steps,
            conversation_manager: self.conversation_manager,
            execution,
        }
    }
}

impl<P> Default for ReActAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentkit_core::error::ProviderError;
    use agentkit_core::provider::types::{ChatResponse, ChatStreamChunk};
    use futures_util::stream;
    use futures_util::stream::BoxStream;

    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            Ok(ChatResponse {
                message: ChatMessage {
                    role: Role::Assistant,
                    content: "Mock response".to_string(),
                    name: None,
                },
                tool_calls: vec![],
                usage: None,
                finish_reason: None,
            })
        }

        fn stream_chat(
            &self,
            _request: ChatRequest,
        ) -> Result<BoxStream<'static, Result<ChatStreamChunk, ProviderError>>, ProviderError>
        {
            Ok(Box::pin(stream::empty()))
        }
    }

    #[test]
    fn test_react_agent_builder() {
        let _agent = ReActAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .max_steps(15)
            .build();
    }
}
