//! Agent（智能体）模块
//!
//! # 概述
//!
//! 本模块提供 DefaultAgent 的实现，包括增强的 DefaultAgent，支持：
//! - Tools（工具调用）
//! - MCP（Model Context Protocol）
//! - A2A（Agent-to-Agent）
//! - Skills（技能）
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::agent::DefaultAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::tools::EchoTool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建 Provider
//! let provider = OpenAiProvider::from_env()?;
//!
//! // 创建 DefaultAgent，支持 tools
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .system_prompt("你是有用的助手")
//!     .tool(EchoTool)
//!     .build();
//!
//! let output = agent.run("你好").await?;
//! # Ok(())
//! # }
//! ```

use agentkit_core::agent::{
    Agent, AgentContext, AgentDecision, AgentError, AgentInput, AgentOutput,
};
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::provider::LlmProvider;
use agentkit_core::tool::types::ToolDefinition;
use agentkit_core::tool::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 增强的 DefaultAgent 实现。
///
/// 支持：
/// - LLM Provider（对话）
/// - Tools（工具调用）
/// - MCP（Model Context Protocol，需要启用 `mcp` feature）
/// - A2A（Agent-to-Agent，需要启用 `a2a` feature）
/// - Skills（技能，需要启用 `skills` feature）
///
/// # 使用示例
///
/// ## 基本使用
///
/// ```rust,no_run
/// use agentkit::agent::DefaultAgent;
/// use agentkit::provider::OpenAiProvider;
/// use agentkit::tools::EchoTool;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .system_prompt("你是有用的助手")
///     .tool(EchoTool)
///     .build();
///
/// let output = agent.run("回显：Hello").await?;
/// println!("回复：{}", output.text().unwrap_or("无回复"));
/// # Ok(())
/// # }
/// ```
///
/// ## 使用 Skills
///
/// ```rust,no_run
/// use agentkit::agent::DefaultAgent;
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .with_skills("skills")
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct DefaultAgent<P> {
    /// LLM Provider。
    #[allow(dead_code)]
    provider: P,
    /// 系统提示词。
    system_prompt: Option<String>,
    /// 默认模型。
    default_model: Option<String>,
    /// 已注册的工具。
    tools: HashMap<String, Arc<dyn Tool>>,
    /// 最大步骤数。
    #[allow(dead_code)]
    max_steps: usize,
    /// Skills 目录路径
    #[cfg(feature = "skills")]
    skills_dir: Option<String>,
    /// MCP 服务器地址
    #[cfg(feature = "mcp")]
    mcp_server: Option<String>,
    /// A2A 代理 URL
    #[cfg(feature = "a2a")]
    a2a_agent_url: Option<String>,
}

impl<P> DefaultAgent<P> {
    /// 创建新的构建器。
    pub fn builder() -> DefaultAgentBuilder<P> {
        DefaultAgentBuilder::new()
    }

    /// 获取 Agent 名称。
    pub fn name(&self) -> &str {
        "default_agent"
    }

    /// 获取工具列表。
    pub fn tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

#[async_trait]
impl<P> Agent for DefaultAgent<P>
where
    P: LlmProvider + Send + Sync,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 检查是否有工具调用结果需要处理
        if !context.tool_results.is_empty() {
            // 有工具结果，让 LLM 生成最终回复
            let mut request = context.default_chat_request();
            self._apply_config(&mut request);
            return AgentDecision::Chat { request };
        }

        // 默认：让 LLM 决定是否调用工具
        let mut request = context.default_chat_request();

        // 如果有工具，添加到请求中供 LLM 选择
        if !self.tools.is_empty() {
            request.tools = Some(self._get_tool_definitions());
        }

        self._apply_config(&mut request);

        AgentDecision::Chat { request }
    }

    fn name(&self) -> &str {
        "default_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("增强的 DefaultAgent 实现，支持 Tools/MCP/A2A/Skills")
    }
}

impl<P> DefaultAgent<P>
where
    P: LlmProvider,
{
    /// 应用配置到聊天请求。
    fn _apply_config(&self, request: &mut ChatRequest) {
        if let Some(ref prompt) = self.system_prompt {
            if request.messages.is_empty()
                || request.messages.first().map(|m| &m.role) != Some(&Role::System)
            {
                request
                    .messages
                    .insert(0, ChatMessage::system(prompt.clone()));
            }
        }

        if let Some(ref model) = self.default_model {
            request.model = Some(model.clone());
        }
    }

    /// 获取工具定义列表。
    fn _get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter_map(|tool| {
                tool.description().map(|desc| ToolDefinition {
                    name: tool.name().to_string(),
                    description: Some(desc.to_string()),
                    input_schema: tool.input_schema(),
                })
            })
            .collect()
    }

    /// 处理工具调用决策。
    pub fn handle_tool_call(&self, tool_name: &str, input: Value) -> Option<AgentDecision> {
        if self.tools.contains_key(tool_name) {
            Some(AgentDecision::ToolCall {
                name: tool_name.to_string(),
                input,
            })
        } else {
            None
        }
    }

    /// 运行 Agent（支持工具调用）。
    ///
    /// 这个方法会循环执行，直到：
    /// - 返回最终结果
    /// - 达到最大步骤数
    /// - 发生错误
    ///
    /// # 参数
    ///
    /// - `input`: 用户输入
    ///
    /// # 返回
    ///
    /// 返回 AgentOutput，包含回复内容、对话历史和工具调用记录。
    pub async fn run(
        &self,
        input: impl Into<AgentInput> + Send,
    ) -> Result<AgentOutput, AgentError> {
        let input = input.into();
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut tool_call_records: Vec<agentkit_core::agent::ToolCallRecord> = Vec::new();
        let mut step = 0;
        let max_steps = self.max_steps;

        // 添加用户消息
        messages.push(ChatMessage::user(input.text.clone()));

        loop {
            if step >= max_steps {
                return Err(AgentError::MaxStepsReached);
            }

            // 创建上下文
            let context = AgentContext {
                input: input.clone(),
                messages: messages.clone(),
                tool_results: Vec::new(),
                step,
                max_steps,
            };

            // 思考
            let decision = self.think(&context).await;

            match decision {
                AgentDecision::Chat { request } => {
                    // 调用 LLM
                    let response = self
                        .provider
                        .chat(request)
                        .await
                        .map_err(|e| AgentError::Message(format!("Provider 错误：{}", e)))?;

                    // 添加助手消息
                    messages.push(response.message.clone());

                    // 检查是否有工具调用
                    if !response.tool_calls.is_empty() {
                        // 执行工具调用
                        for tool_call in response.tool_calls {
                            if let Some(tool) = self.tools.get(&tool_call.name) {
                                // 执行工具
                                let result =
                                    tool.call(tool_call.input.clone()).await.map_err(|e| {
                                        AgentError::Message(format!("工具执行错误：{}", e))
                                    })?;

                                // 记录工具调用
                                tool_call_records.push(agentkit_core::agent::ToolCallRecord {
                                    name: tool_call.name.clone(),
                                    input: tool_call.input.clone(),
                                    result: result.clone(),
                                });

                                // 添加工具结果到消息
                                messages.push(ChatMessage::tool(
                                    tool_call.name.clone(),
                                    result.to_string(),
                                ));
                            }
                        }

                        // 继续循环，让 LLM 生成最终回复
                        step += 1;
                    } else {
                        // 没有工具调用，返回最终结果
                        return Ok(AgentOutput::with_history(
                            Value::Object(serde_json::Map::from_iter(vec![(
                                "content".to_string(),
                                Value::String(response.message.content.clone()),
                            )])),
                            messages,
                            tool_call_records,
                        ));
                    }
                }
                AgentDecision::ToolCall { name, input } => {
                    // 直接工具调用（来自 think 方法的决策）
                    if let Some(tool) = self.tools.get(&name) {
                        let result = tool
                            .call(input.clone())
                            .await
                            .map_err(|e| AgentError::Message(format!("工具执行错误：{}", e)))?;

                        tool_call_records.push(agentkit_core::agent::ToolCallRecord {
                            name: name.clone(),
                            input: input.clone(),
                            result: result.clone(),
                        });

                        messages.push(ChatMessage::tool(name.clone(), result.to_string()));

                        step += 1;
                    } else {
                        return Err(AgentError::Message(format!("未找到工具：{}", name)));
                    }
                }
                AgentDecision::Return(value) => {
                    return Ok(AgentOutput::with_history(
                        value,
                        messages,
                        tool_call_records,
                    ));
                }
                AgentDecision::Stop => {
                    return Ok(AgentOutput::with_history(
                        Value::Null,
                        messages,
                        tool_call_records,
                    ));
                }
                AgentDecision::ThinkAgain => {
                    step += 1;
                }
            }
        }
    }
}

/// DefaultAgent 构建器。
///
/// # 使用示例
///
/// ## 基本使用
///
/// ```rust,no_run
/// use agentkit::agent::DefaultAgent;
/// use agentkit::provider::OpenAiProvider;
/// use agentkit::tools::EchoTool;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .system_prompt("你是有用的助手")
///     .default_model("gpt-4o-mini")
///     .tool(EchoTool)
///     .max_steps(10)
///     .build();
/// # Ok(())
/// # }
/// ```
///
/// ## 使用 Skills 目录
///
/// ```rust,no_run
/// use agentkit::agent::DefaultAgent;
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .system_prompt("你是有用的助手")
///     .with_skills("skills")  // 加载 skills 目录
///     .build();
/// # Ok(())
/// # }
/// ```
///
/// ## 使用 MCP 服务器
///
/// ```rust,no_run
/// use agentkit::agent::DefaultAgent;
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .system_prompt("你是有用的助手")
///     .with_mcp("http://localhost:8080")  // MCP 服务器地址
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct DefaultAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    default_model: Option<String>,
    tools: HashMap<String, Arc<dyn Tool>>,
    max_steps: usize,
    /// Skills 目录路径
    skills_dir: Option<String>,
    /// MCP 服务器地址
    #[cfg(feature = "mcp")]
    mcp_server: Option<String>,
    /// A2A 代理 URL
    #[cfg(feature = "a2a")]
    a2a_agent_url: Option<String>,
}

impl<P> DefaultAgentBuilder<P> {
    /// 创建新的构建器。
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            default_model: None,
            tools: HashMap::new(),
            max_steps: 10,
            skills_dir: None,
            #[cfg(feature = "mcp")]
            mcp_server: None,
            #[cfg(feature = "a2a")]
            a2a_agent_url: None,
        }
    }
}

impl<P> Default for DefaultAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> DefaultAgentBuilder<P>
where
    P: LlmProvider,
{
    /// 设置 Provider。
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 设置系统提示词。
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置默认模型。
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// 注册工具。
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
        self
    }

    /// 注册多个工具。
    pub fn tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Tool>>,
    {
        for tool in tools {
            let name = tool.name().to_string();
            self.tools.insert(name, tool);
        }
        self
    }

    /// 设置最大步骤数。
    pub fn max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    /// 配置 Skills 目录
    ///
    /// # 参数
    ///
    /// - `dir`: Skills 目录路径
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `skills` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::DefaultAgent;
    /// use agentkit::provider::OpenAiProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .with_skills("skills")
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "skills")]
    pub fn with_skills(mut self, dir: impl Into<String>) -> Self {
        self.skills_dir = Some(dir.into());
        self
    }

    /// 配置 MCP 服务器
    ///
    /// # 参数
    ///
    /// - `server_url`: MCP 服务器地址
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `mcp` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::DefaultAgent;
    /// use agentkit::provider::OpenAiProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .with_mcp("http://localhost:8080")
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "mcp")]
    pub fn with_mcp(mut self, server_url: impl Into<String>) -> Self {
        self.mcp_server = Some(server_url.into());
        self
    }

    /// 配置 A2A 代理
    ///
    /// # 参数
    ///
    /// - `agent_url`: A2A 代理 URL
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `a2a` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::DefaultAgent;
    /// use agentkit::provider::OpenAiProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .with_a2a("http://agent.example.com")
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "a2a")]
    pub fn with_a2a(mut self, agent_url: impl Into<String>) -> Self {
        self.a2a_agent_url = Some(agent_url.into());
        self
    }

    /// 构建 Agent。
    pub fn build(self) -> DefaultAgent<P> {
        DefaultAgent {
            provider: self
                .provider
                .expect("Provider is required for DefaultAgent"),
            system_prompt: self.system_prompt,
            default_model: self.default_model,
            tools: self.tools,
            max_steps: self.max_steps,
            #[cfg(feature = "skills")]
            skills_dir: self.skills_dir,
            #[cfg(feature = "mcp")]
            mcp_server: self.mcp_server,
            #[cfg(feature = "a2a")]
            a2a_agent_url: self.a2a_agent_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        // 这个测试需要实际的 Provider，所以只测试 builder 的链式调用
        let _builder = DefaultAgentBuilder::<MockProvider>::new()
            .system_prompt("test")
            .default_model("gpt-4")
            .max_steps(5);
    }

    // Mock Provider 用于测试
    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(
            &self,
            _request: ChatRequest,
        ) -> Result<agentkit_core::provider::types::ChatResponse, agentkit_core::error::ProviderError>
        {
            unimplemented!()
        }

        fn stream_chat(
            &self,
            _request: ChatRequest,
        ) -> Result<
            futures_util::stream::BoxStream<
                'static,
                Result<
                    agentkit_core::provider::types::ChatStreamChunk,
                    agentkit_core::error::ProviderError,
                >,
            >,
            agentkit_core::error::ProviderError,
        > {
            unimplemented!()
        }
    }
}
