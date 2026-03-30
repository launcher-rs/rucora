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
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::tool::Tool;
use agentkit_core::tool::types::ToolDefinition;
use agentkit_core::tool::types::ToolResult;
use async_trait::async_trait;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::conversation::ConversationManager;

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

/// MCP 服务器配置
///
/// 支持多个 MCP 服务器，每个服务器可以独立配置认证信息
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::agent::McpServerConfig;
///
/// // 基本配置（无认证）
/// let config = McpServerConfig::new("http://localhost:8080");
///
/// // 带 Token 认证
/// let config = McpServerConfig::with_auth(
///     "http://localhost:8080",
///     "Bearer my-token"
/// );
///
/// // 带自定义超时
/// let config = McpServerConfig::new("http://localhost:8080")
///     .with_timeout_secs(60);
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "mcp", derive(Debug))]
pub struct McpServerConfig {
    /// 服务器 URL
    pub url: String,
    /// 认证头（可选），例如 "Bearer token123"
    pub auth_header: Option<String>,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 自定义元数据（可选）
    pub metadata: HashMap<String, Value>,
}

#[cfg(feature = "mcp")]
impl McpServerConfig {
    /// 创建新的 MCP 服务器配置
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_header: None,
            timeout_secs: 30,
            metadata: HashMap::new(),
        }
    }

    /// 创建带认证的配置
    pub fn with_auth(url: impl Into<String>, auth_header: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_header: Some(auth_header.into()),
            timeout_secs: 30,
            metadata: HashMap::new(),
        }
    }

    /// 设置超时时间（秒）
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// 添加自定义元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// A2A 代理配置
///
/// 支持多个 A2A 代理，每个代理可以独立配置认证信息
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::agent::A2aAgentConfig;
///
/// // 基本配置
/// let config = A2aAgentConfig::new("http://agent.example.com");
///
/// // 带 Token 认证
/// let config = A2aAgentConfig::with_auth(
///     "http://agent.example.com",
///     "Bearer my-token"
/// );
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "a2a", derive(Debug))]
pub struct A2aAgentConfig {
    /// 代理 URL
    pub url: String,
    /// 认证头（可选）
    pub auth_header: Option<String>,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 自定义元数据（可选）
    pub metadata: HashMap<String, Value>,
}

#[cfg(feature = "a2a")]
impl A2aAgentConfig {
    /// 创建新的 A2A 代理配置
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_header: None,
            timeout_secs: 30,
            metadata: HashMap::new(),
        }
    }

    /// 创建带认证的配置
    pub fn with_auth(url: impl Into<String>, auth_header: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_header: Some(auth_header.into()),
            timeout_secs: 30,
            metadata: HashMap::new(),
        }
    }

    /// 设置超时时间（秒）
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// 添加自定义元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Skills 目录配置
///
/// 支持多个 Skills 目录，每个目录可以独立配置
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::agent::SkillsDirConfig;
///
/// // 基本配置
/// let config = SkillsDirConfig::new("skills");
///
/// // 带自定义配置
/// let config = SkillsDirConfig::new("skills")
///     .with_enabled(true);
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "skills", derive(Debug))]
pub struct SkillsDirConfig {
    /// 目录路径
    pub path: PathBuf,
    /// 是否启用（默认 true）
    pub enabled: bool,
    /// 自定义元数据（可选）
    pub metadata: HashMap<String, Value>,
}

#[cfg(feature = "skills")]
impl SkillsDirConfig {
    /// 创建新的 Skills 目录配置
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            enabled: true,
            metadata: HashMap::new(),
        }
    }

    /// 设置是否启用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 添加自定义元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

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
/// // Agent 必须指定 model
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .model("gpt-4o-mini")  // ← 必须设置
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
/// ## 使用多个 MCP 服务器
///
/// ```rust,no_run
/// use agentkit::agent::{DefaultAgent, McpServerConfig};
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .model("gpt-4o-mini")
///     .mcp_server(McpServerConfig::new("http://localhost:8080"))
///     .mcp_server(McpServerConfig::with_auth(
///         "http://mcp.example.com",
///         "Bearer token123"
///     ))
///     .build();
/// # Ok(())
/// # }
/// ```
///
/// ## 使用多个 Skills 目录
///
/// ```rust,no_run
/// use agentkit::agent::{DefaultAgent, SkillsDirConfig};
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .model("gpt-4o-mini")
///     .skills_dir(SkillsDirConfig::new("skills"))
///     .skills_dir(SkillsDirConfig::new("custom_skills"))
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
    /// 默认使用的模型。
    model: String,
    /// 已注册的工具。
    tools: HashMap<String, Arc<dyn Tool>>,
    /// 最大步骤数。
    #[allow(dead_code)]
    max_steps: usize,
    /// 对话管理器（可选，用于自动管理对话历史）
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// Skills 目录配置列表
    #[cfg(feature = "skills")]
    skills_dirs: Vec<SkillsDirConfig>,
    /// MCP 服务器配置列表
    #[cfg(feature = "mcp")]
    mcp_servers: Vec<McpServerConfig>,
    /// A2A 代理配置列表
    #[cfg(feature = "a2a")]
    a2a_agents: Vec<A2aAgentConfig>,
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

    /// 获取 MCP 服务器配置列表
    #[cfg(feature = "mcp")]
    pub fn mcp_servers(&self) -> &[McpServerConfig] {
        &self.mcp_servers
    }

    /// 获取 A2A 代理配置列表
    #[cfg(feature = "a2a")]
    pub fn a2a_agents(&self) -> &[A2aAgentConfig] {
        &self.a2a_agents
    }

    /// 获取 Skills 目录配置列表
    #[cfg(feature = "skills")]
    pub fn skills_dirs(&self) -> &[SkillsDirConfig] {
        &self.skills_dirs
    }

    /// 获取对话历史（如果启用了对话历史）。
    ///
    /// # 返回
    ///
    /// - `Some(Vec<ChatMessage>)`: 如果启用了对话历史，返回历史消息的副本
    /// - `None`: 如果没有启用对话历史
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::DefaultAgent;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .model("gpt-4o-mini")
    ///     .with_conversation(true)
    ///     .build();
    ///
    /// // 获取对话历史
    /// if let Some(history) = agent.get_conversation_history().await {
    ///     println!("历史消息数：{}", history.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_conversation_history(&self) -> Option<Vec<ChatMessage>> {
        match &self.conversation_manager {
            Some(conv_arc) => {
                let conv = conv_arc.lock().await;
                Some(conv.get_messages().to_vec())
            }
            None => None,
        }
    }

    /// 清空对话历史（如果启用了对话历史）。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::DefaultAgent;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .model("gpt-4o-mini")
    ///     .with_conversation(true)
    ///     .build();
    ///
    /// // 清空对话历史
    /// agent.clear_conversation().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear_conversation(&self) {
        if let Some(ref conv_arc) = self.conversation_manager {
            let mut conv = conv_arc.lock().await;
            conv.clear();
            // 重新添加系统提示
            if let Some(ref prompt) = self.system_prompt {
                conv.ensure_system_prompt(prompt);
            }
        }
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

        request.model = Some(self.model.clone());
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
    ///
    /// # 对话历史
    ///
    /// 如果启用了对话历史（`with_conversation(true)`），此方法会自动：
    /// 1. 添加用户消息到历史
    /// 2. 添加助手回复到历史
    /// 3. 在下次调用时包含历史记录
    pub async fn run(
        &self,
        input: impl Into<AgentInput> + Send,
    ) -> Result<AgentOutput, AgentError> {
        let input = input.into();
        let mut messages: Vec<ChatMessage> = Vec::new();
        let mut tool_call_records: Vec<agentkit_core::agent::ToolCallRecord> = Vec::new();
        let mut step = 0;
        let max_steps = self.max_steps;

        // 如果启用了对话历史，从历史中获取消息
        let conversation_history: Vec<ChatMessage> =
            if let Some(ref conv_arc) = self.conversation_manager {
                let conv = conv_arc.lock().await;
                conv.get_messages().to_vec()
            } else {
                Vec::new()
            };
        messages.extend(conversation_history);

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
                            let output_value = if let Some(tool) = self.tools.get(&tool_call.name) {
                                match tool.call(tool_call.input.clone()).await {
                                    Ok(v) => json!({"ok": true, "output": v}),
                                    Err(e) => json!({
                                        "ok": false,
                                        "error": {"kind": "tool_error", "message": e.to_string()}
                                    }),
                                }
                            } else {
                                json!({
                                    "ok": false,
                                    "error": {"kind": "not_found", "message": format!("未找到工具：{}", tool_call.name)}
                                })
                            };

                            tool_call_records.push(agentkit_core::agent::ToolCallRecord {
                                name: tool_call.name.clone(),
                                input: tool_call.input.clone(),
                                result: output_value.clone(),
                            });

                            let result = ToolResult {
                                tool_call_id: tool_call.id.clone(),
                                output: output_value,
                            };
                            messages.push(tool_result_to_message(&result, &tool_call.name));
                        }

                        // 继续循环，让 LLM 生成最终回复
                        step += 1;
                    } else {
                        // 没有工具调用，返回最终结果
                        let output = Ok(AgentOutput::with_history(
                            Value::Object(serde_json::Map::from_iter(vec![(
                                "content".to_string(),
                                Value::String(response.message.content.clone()),
                            )])),
                            messages.clone(),
                            tool_call_records,
                        ));

                        // 如果启用了对话历史，保存消息到历史
                        if let Some(ref conv_arc) = self.conversation_manager {
                            let mut conv = conv_arc.lock().await;
                            // 添加用户消息
                            conv.add_user_message(input.text.clone());
                            // 添加助手回复
                            conv.add_assistant_message(response.message.content.clone());
                        }

                        return output;
                    }
                }
                AgentDecision::ToolCall { name, input } => {
                    // 直接工具调用（来自 think 方法的决策）
                    if let Some(tool) = self.tools.get(&name) {
                        let tool_call_id = format!("local_call_{}_{}", step, name);
                        let output_value = match tool.call(input.clone()).await {
                            Ok(v) => json!({"ok": true, "output": v}),
                            Err(e) => json!({
                                "ok": false,
                                "error": {"kind": "tool_error", "message": e.to_string()}
                            }),
                        };

                        tool_call_records.push(agentkit_core::agent::ToolCallRecord {
                            name: name.clone(),
                            input: input.clone(),
                            result: output_value.clone(),
                        });

                        let result = ToolResult {
                            tool_call_id,
                            output: output_value,
                        };
                        messages.push(tool_result_to_message(&result, &name));

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
/// ## 使用多个 MCP 服务器
///
/// ```rust,no_run
/// use agentkit::agent::{DefaultAgent, McpServerConfig};
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .mcp_server(McpServerConfig::new("http://localhost:8080"))
///     .mcp_server(McpServerConfig::with_auth(
///         "http://mcp.example.com",
///         "Bearer token123"
///     ))
///     .build();
/// # Ok(())
/// # }
/// ```
///
/// ## 使用多个 Skills 目录
///
/// ```rust,no_run
/// use agentkit::agent::{DefaultAgent, SkillsDirConfig};
/// use agentkit::provider::OpenAiProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAiProvider::from_env()?;
///
/// // Agent 必须指定 model
/// let agent = DefaultAgent::builder()
///     .provider(provider)
///     .model("gpt-4o-mini")  // ← 必须设置
///     .skills_dir(SkillsDirConfig::new("skills"))
///     .skills_dir(SkillsDirConfig::new("custom_skills"))
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct DefaultAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: HashMap<String, Arc<dyn Tool>>,
    max_steps: usize,
    /// 对话管理器配置
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// 对话历史最大消息数（在 build 时应用）
    max_conversation_messages: Option<usize>,
    /// Skills 目录配置列表
    #[cfg(feature = "skills")]
    skills_dirs: Vec<SkillsDirConfig>,
    /// MCP 服务器配置列表
    #[cfg(feature = "mcp")]
    mcp_servers: Vec<McpServerConfig>,
    /// A2A 代理配置列表
    #[cfg(feature = "a2a")]
    a2a_agents: Vec<A2aAgentConfig>,
}

impl<P> DefaultAgentBuilder<P> {
    /// 创建新的构建器。
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            tools: HashMap::new(),
            max_steps: 10,
            conversation_manager: None,
            max_conversation_messages: None,
            #[cfg(feature = "skills")]
            skills_dirs: Vec::new(),
            #[cfg(feature = "mcp")]
            mcp_servers: Vec::new(),
            #[cfg(feature = "a2a")]
            a2a_agents: Vec::new(),
        }
    }
}

impl<P> DefaultAgentBuilder<P>
where
    P: LlmProvider,
{
    /// 设置 Provider（必需）。
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    /// 设置系统提示词。
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 设置默认模型（必需）。
    ///
    /// # 参数
    ///
    /// - `model`: 默认使用的模型名称
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
    ///     .model("gpt-4o-mini")  // ← 必须设置
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
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

    /// 启用对话历史管理（自动管理多轮对话上下文）。
    ///
    /// # 参数
    ///
    /// - `enabled`: 是否启用对话历史
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
    ///     .model("gpt-4o-mini")
    ///     .with_conversation(true)  // 启用对话历史
    ///     .build();
    ///
    /// // 第一轮
    /// agent.run("你好，我叫小明").await?;
    ///
    /// // 第二轮（自动记住上一轮）
    /// agent.run("你还记得我叫什么吗？").await?;  // 回答：小明
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_conversation(mut self, enabled: bool) -> Self {
        if enabled {
            let mut conv = ConversationManager::new();
            if let Some(ref prompt) = self.system_prompt {
                conv = conv.with_system_prompt(prompt.clone());
            }
            // 应用 max_messages 设置
            if let Some(max_msgs) = self.max_conversation_messages {
                conv = conv.with_max_messages(max_msgs);
            }
            self.conversation_manager = Some(Arc::new(Mutex::new(conv)));
        } else {
            self.conversation_manager = None;
        }
        self
    }

    /// 设置对话历史最大消息数（仅在启用对话时有效）。
    ///
    /// # 参数
    ///
    /// - `max_messages`: 最大消息数（0 表示无限制）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::DefaultAgent;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .model("gpt-4o-mini")
    ///     .with_conversation(true)
    ///     .with_max_messages(20)  // 保留最近 20 条消息
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_max_messages(mut self, max_messages: usize) -> Self {
        self.max_conversation_messages = Some(max_messages);
        self
    }

    /// 添加 Skills 目录配置
    ///
    /// # 参数
    ///
    /// - `config`: Skills 目录配置
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `skills` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::{DefaultAgent, SkillsDirConfig};
    /// use agentkit::provider::OpenAiProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .skills_dir(SkillsDirConfig::new("skills"))
    ///     .skills_dir(SkillsDirConfig::new("custom_skills"))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "skills")]
    pub fn skills_dir(mut self, config: SkillsDirConfig) -> Self {
        self.skills_dirs.push(config);
        self
    }

    /// 添加多个 Skills 目录配置
    ///
    /// # 参数
    ///
    /// - `configs`: Skills 目录配置列表
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `skills` feature。
    #[cfg(feature = "skills")]
    pub fn skills_dirs<I>(mut self, configs: I) -> Self
    where
        I: IntoIterator<Item = SkillsDirConfig>,
    {
        self.skills_dirs.extend(configs);
        self
    }

    /// 添加 MCP 服务器配置
    ///
    /// # 参数
    ///
    /// - `config`: MCP 服务器配置
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `mcp` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::{DefaultAgent, McpServerConfig};
    /// use agentkit::provider::OpenAiProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .mcp_server(McpServerConfig::new("http://localhost:8080"))
    ///     .mcp_server(McpServerConfig::with_auth(
    ///         "http://mcp.example.com",
    ///         "Bearer token123"
    ///     ))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "mcp")]
    pub fn mcp_server(mut self, config: McpServerConfig) -> Self {
        self.mcp_servers.push(config);
        self
    }

    /// 添加多个 MCP 服务器配置
    ///
    /// # 参数
    ///
    /// - `configs`: MCP 服务器配置列表
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `mcp` feature。
    #[cfg(feature = "mcp")]
    pub fn mcp_servers<I>(mut self, configs: I) -> Self
    where
        I: IntoIterator<Item = McpServerConfig>,
    {
        self.mcp_servers.extend(configs);
        self
    }

    /// 添加 A2A 代理配置
    ///
    /// # 参数
    ///
    /// - `config`: A2A 代理配置
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `a2a` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::agent::{DefaultAgent, A2aAgentConfig};
    /// use agentkit::provider::OpenAiProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = DefaultAgent::builder()
    ///     .provider(provider)
    ///     .a2a_agent(A2aAgentConfig::new("http://agent.example.com"))
    ///     .a2a_agent(A2aAgentConfig::with_auth(
    ///         "http://agent2.example.com",
    ///         "Bearer token456"
    ///     ))
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "a2a")]
    pub fn a2a_agent(mut self, config: A2aAgentConfig) -> Self {
        self.a2a_agents.push(config);
        self
    }

    /// 添加多个 A2A 代理配置
    ///
    /// # 参数
    ///
    /// - `configs`: A2A 代理配置列表
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `a2a` feature。
    #[cfg(feature = "a2a")]
    pub fn a2a_agents<I>(mut self, configs: I) -> Self
    where
        I: IntoIterator<Item = A2aAgentConfig>,
    {
        self.a2a_agents.extend(configs);
        self
    }

    /// 构建 Agent。
    ///
    /// # Panics
    ///
    /// 如果没有调用 `model()` 方法，此方法会 panic。
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
    ///     .model("gpt-4o-mini")  // ← 必须设置
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> DefaultAgent<P> {
        DefaultAgent {
            provider: self
                .provider
                .expect("Provider is required for DefaultAgent"),
            system_prompt: self.system_prompt.clone(),
            model: self.model.expect(
                "model is required for DefaultAgent. Please call .model() method before build().",
            ),
            tools: self.tools,
            max_steps: self.max_steps,
            conversation_manager: self.conversation_manager,
            #[cfg(feature = "skills")]
            skills_dirs: self.skills_dirs,
            #[cfg(feature = "mcp")]
            mcp_servers: self.mcp_servers,
            #[cfg(feature = "a2a")]
            a2a_agents: self.a2a_agents,
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
            .model("gpt-4")
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
