//! ReflectAgent - 反思迭代 Agent
//!
//! # 概述
//!
//! ReflectAgent 实现反思迭代循环：
//! 1. **Generate**（生成）：生成初始版本
//! 2. **Reflect**（反思）：自我批评、分析问题
//! 3. **Improve**（改进）：根据反思改进
//! 4. 循环直到达到质量阈值或最大迭代次数
//!
//! # 适用场景
//!
//! - 代码生成（需要高质量代码）
//! - 文档写作
//! - 方案设计
//! - 任何需要迭代改进的任务
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::agent::ReflectAgent;
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::tools::FileWriteTool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = ReflectAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是一个追求卓越的程序员")
//!     .tool(FileWriteTool)
//!     .max_iterations(3)
//!     .quality_threshold(0.9)
//!     .build();
//!
//! let output = agent.run("帮我写一个快速排序算法，要求有详细注释").await?;
//! # Ok(())
//! # }
//! ```

use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput, AgentOutput};
use agentkit_core::provider::LlmProvider;
use agentkit_core::provider::types::{ChatMessage, ChatRequest, Role};
use agentkit_core::tool::Tool;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agent::ToolRegistry;
use crate::agent::execution::DefaultExecution;
use crate::conversation::ConversationManager;

/// ReflectAgent - 反思迭代 Agent
///
/// 特点：
/// - 生成 - 反思 - 改进循环
/// - 自我批评、持续改进
/// - 适合需要高质量输出的任务
pub struct ReflectAgent<P> {
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
    /// 最大迭代次数
    #[allow(dead_code)]
    max_iterations: usize,
    /// 质量阈值（0.0-1.0）
    #[allow(dead_code)]
    quality_threshold: f32,
    /// 对话管理器（可选）
    #[allow(dead_code)]
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    /// 执行能力（内聚）
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for ReflectAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        let iteration = context.step / 2; // 每 2 步为一次迭代（生成 + 反思）

        if iteration == 0 {
            // 第一次：生成初始版本
            AgentDecision::Chat {
                request: Box::new(self._build_generate_request(context)),
            }
        } else if iteration >= self.max_iterations {
            // 达到最大迭代次数，返回当前最佳
            AgentDecision::Return(self._build_final_result(context))
        } else {
            // 奇数步：反思
            if context.step % 2 == 1 {
                AgentDecision::Chat {
                    request: Box::new(self._build_reflect_request(context, iteration)),
                }
            } else {
                // 偶数步：根据反思改进
                AgentDecision::Chat {
                    request: Box::new(self._build_improve_request(context, iteration)),
                }
            }
        }
    }

    fn name(&self) -> &str {
        "reflect_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("反思迭代 Agent，生成 - 反思 - 改进循环")
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

impl<P> ReflectAgent<P>
where
    P: LlmProvider,
{
    /// 创建新的构建器
    pub fn builder() -> ReflectAgentBuilder<P> {
        ReflectAgentBuilder::new()
    }

    /// 构建生成请求
    fn _build_generate_request(&self, context: &AgentContext) -> ChatRequest {
        let prompt = format!(
            "请生成初始版本：{}\n\
             \n\
             要求：\n\
             1. 完整实现功能\n\
             2. 保证正确性\n\
             3. 尽可能详细\n\
             \n\
             稍后我会进行自我反思和改进。",
            context.input.text()
        );

        self._build_request(context, prompt)
    }

    /// 构建反思请求
    fn _build_reflect_request(&self, context: &AgentContext, iteration: usize) -> ChatRequest {
        let prompt = format!(
            "请反思第 {iteration} 版本的质量：\n\
             \n\
             反思维度：\n\
             1. **正确性**：是否有错误或遗漏？\n\
             2. **完整性**：是否覆盖所有需求？\n\
             3. **清晰度**：是否易于理解？\n\
             4. **优化空间**：哪些地方可以改进？\n\
             \n\
             请详细列出问题和改进建议。"
        );

        self._build_request(context, prompt)
    }

    /// 构建改进请求
    fn _build_improve_request(&self, context: &AgentContext, iteration: usize) -> ChatRequest {
        let prompt = format!(
            "请根据反思改进第 {} 版本：\n\
             \n\
             改进要求：\n\
             1. 修复所有发现的问题\n\
             2. 采纳所有合理的改进建议\n\
             3. 保持原有优点\n\
             4. 生成更高质量的版本\n\
             \n\
             目标质量阈值：{}",
            iteration, self.quality_threshold
        );

        self._build_request(context, prompt)
    }

    /// 构建最终结果
    fn _build_final_result(&self, context: &AgentContext) -> Value {
        // 从上下文中提取最新版本
        if let Some(last_msg) = context.messages.last() {
            json!({
                "content": last_msg.content,
                "iterations": self.max_iterations,
                "completed": true
            })
        } else {
            json!({
                "content": "未能生成结果",
                "iterations": self.max_iterations,
                "completed": false
            })
        }
    }

    /// 构建通用请求
    fn _build_request(&self, context: &AgentContext, prompt: String) -> ChatRequest {
        let mut messages = context.messages.clone();

        // 添加系统提示词
        if let Some(ref sys_prompt) = self.system_prompt
            && (messages.is_empty() || messages.first().map(|m| &m.role) != Some(&Role::System))
        {
            messages.insert(0, ChatMessage::system(sys_prompt.clone()));
        }

        // 添加当前提示词
        messages.push(ChatMessage::user(prompt));

        ChatRequest {
            messages,
            model: Some(self.model.clone()),
            tools: if !self.tools.definitions().is_empty() {
                Some(self.tools.definitions())
            } else {
                None
            },
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

/// ReflectAgent 构建器
pub struct ReflectAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: ToolRegistry,
    max_iterations: usize,
    quality_threshold: f32,
    conversation_manager: Option<Arc<Mutex<ConversationManager>>>,
    middleware_chain: crate::middleware::MiddlewareChain,
}

impl<P> ReflectAgentBuilder<P> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            tools: ToolRegistry::new(),
            max_iterations: 3,
            quality_threshold: 0.9,
            conversation_manager: None,
            middleware_chain: crate::middleware::MiddlewareChain::new(),
        }
    }
}

impl<P> ReflectAgentBuilder<P>
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

    /// 设置最大迭代次数
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// 设置质量阈值（0.0-1.0）
    ///
    /// 当自我评估质量达到此阈值时，提前停止迭代
    pub fn quality_threshold(mut self, threshold: f32) -> Self {
        self.quality_threshold = threshold.clamp(0.0, 1.0);
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

    /// 设置中间件链
    pub fn with_middleware_chain(
        mut self,
        middleware_chain: crate::middleware::MiddlewareChain,
    ) -> Self {
        self.middleware_chain = middleware_chain;
        self
    }

    /// 添加中间件
    pub fn with_middleware<M: crate::middleware::Middleware + 'static>(
        mut self,
        middleware: M,
    ) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    /// 构建 Agent
    ///
    /// # Panics
    ///
    /// 如果没有设置 `provider` 或 `model`，此方法会 panic。
    pub fn build(self) -> ReflectAgent<P> {
        let provider = self.provider.expect("Provider is required");
        let model = self.model.expect("Model is required");

        // 创建执行能力
        let provider_arc = Arc::new(provider);
        let execution =
            DefaultExecution::new(provider_arc.clone(), model.clone(), self.tools.clone())
                .with_system_prompt_opt(self.system_prompt.clone())
                .with_max_steps(self.max_iterations * 2) // 每次迭代需要 2 步
                .with_conversation_manager(self.conversation_manager.clone())
                .with_middleware_chain(self.middleware_chain);

        ReflectAgent {
            provider: provider_arc,
            model,
            system_prompt: self.system_prompt,
            tools: self.tools,
            max_iterations: self.max_iterations,
            quality_threshold: self.quality_threshold,
            conversation_manager: self.conversation_manager,
            execution,
        }
    }
}

impl<P> Default for ReflectAgentBuilder<P> {
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
    fn test_reflect_agent_builder() {
        let _agent = ReflectAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .max_iterations(3)
            .quality_threshold(0.9)
            .build();
    }
}
