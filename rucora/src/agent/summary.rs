//! SummaryAgent - 文本摘要 Agent
//!
//! # 概述
//!
//! SummaryAgent 专门用于文本摘要任务，支持多种摘要模式。
//! 对于长文本自动采用分块-合并（map-reduce）策略：
//!
//! 1. **Map**: 将长文本分块，逐块生成局部摘要
//! 2. **Reduce**: 将所有局部总结合并为完整最终摘要
//!
//! # 适用场景
//!
//! - 长文档/文章/网页内容总结
//! - 会议纪要、对话记录摘要
//! - 论文/报告要点提取
//! - 信息浓缩和关键信息提取
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use rucora::agent::SummaryAgent;
//! use rucora::provider::OpenAiProvider;
//! use rucora::prelude::Agent;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OpenAiProvider::from_env()?;
//!
//! let agent = SummaryAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .mode(rucora::agent::SummaryMode::Concise)
//!     .try_build()?;
//!
//! let output = agent.run("这是一篇很长的文章...".into()).await?;
//! println!("{}", output.text().unwrap_or("无回复"));
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use rucora_core::agent::{Agent, AgentContext, AgentDecision, AgentError, AgentInput, AgentOutput};
use rucora_core::provider::LlmProvider;
use rucora_core::provider::types::{ChatMessage, ChatRequest, LlmParams};
use serde_json::json;
use std::sync::Arc;
use text_splitter::TextSplitter;

use crate::agent::execution::{build_default_execution, DefaultExecution};

/// 摘要模式。
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SummaryMode {
    /// 简洁摘要（2-3 句话）。
    #[default]
    Concise,
    /// 详细摘要（保留重要细节和论据）。
    Detailed,
    /// 要点列表。
    BulletPoints,
    /// 关键信息提取。
    KeyPoints,
    /// 自定义指令。
    Custom(String),
}

impl SummaryMode {
    /// 获取模式对应的指令文本。
    fn instruction(&self) -> &str {
        match self {
            Self::Concise => "请用简洁的 2-3 句话总结以下文本的核心内容。直接给出总结，不要添加额外说明。",
            Self::Detailed => "请详细总结以下文本的内容，保留重要细节、论据和结论。确保覆盖所有关键部分。",
            Self::BulletPoints => "请用要点列表形式总结以下文本的核心内容。每个要点应独立且完整。",
            Self::KeyPoints => "请提取以下文本的关键信息和核心观点。只列出最重要的内容，忽略次要细节。",
            Self::Custom(instruction) => instruction,
        }
    }
}

impl From<String> for SummaryMode {
    fn from(instruction: String) -> Self {
        Self::Custom(instruction)
    }
}

impl From<&str> for SummaryMode {
    fn from(instruction: &str) -> Self {
        Self::Custom(instruction.to_string())
    }
}

const DEFAULT_PROMPT_TEMPLATE: &str = "\
请总结以下文本：\n\
\n\
{text}\n\
\n\
{mode}";

const DEFAULT_CHUNK_TEMPLATE: &str = "\
以下是待总结文本的第 {index}/{total} 部分：\n\
\n\
{text}\n\
\n\
请总结以上文本部分的要点。";

const DEFAULT_COMBINE_TEMPLATE: &str = "\
以上是文本各部分的局部总结。请将它们合并为一份完整的最终总结。\n\
\n\
{mode}";

/// SummaryAgent - 文本摘要 Agent
///
/// 特点：
/// - 长文本自动分块处理（map-reduce）
/// - 多模式输出（简洁、详细、要点、关键信息、自定义）
/// - 纯 LLM 调用，不依赖工具
/// - 支持自定义提示词模板
pub struct SummaryAgent<P> {
    provider: Arc<P>,
    model: String,
    system_prompt: Option<String>,
    llm_params: LlmParams,
    mode: SummaryMode,
    chunk_size: usize,
    prompt_template: String,
    chunk_template: String,
    combine_template: String,
    execution: DefaultExecution,
}

#[async_trait]
impl<P> Agent for SummaryAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        let text = context.input.text();
        let step = context.step;

        let splitter = TextSplitter::new(self.chunk_size);
        let chunks: Vec<&str> = splitter.chunks(text).collect();
        let total = chunks.len();

        if total <= 1 {
            if step == 0 {
                AgentDecision::Chat {
                    request: Box::new(self.build_single_request(text)),
                }
            } else {
                self.return_last_assistant(context)
            }
        } else if step < total {
            let content = self.chunk_template
                .replace("{index}", &(step + 1).to_string())
                .replace("{total}", &total.to_string())
                .replace("{text}", chunks[step])
                .replace("{mode}", self.mode.instruction());
            AgentDecision::Chat {
                request: Box::new(self.build_multi_request(context, content)),
            }
        } else if step == total {
            let content = self.combine_template
                .replace("{total}", &total.to_string())
                .replace("{mode}", self.mode.instruction());
            AgentDecision::Chat {
                request: Box::new(self.build_multi_request(context, content)),
            }
        } else {
            self.return_last_assistant(context)
        }
    }

    fn name(&self) -> &str {
        "summary_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("文本摘要 Agent，支持长文档自动分块和多模式输出")
    }

    async fn run(&self, input: AgentInput) -> Result<AgentOutput, rucora_core::agent::AgentError> {
        self.execution.run(self, input).await
    }

    fn run_stream(
        &self,
        input: AgentInput,
    ) -> futures_util::stream::BoxStream<
        'static,
        Result<rucora_core::channel::types::ChannelEvent, rucora_core::agent::AgentError>,
    > {
        self.execution.run_stream_simple(input)
    }
}

impl<P> SummaryAgent<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    pub async fn run_stream_text(
        &self,
        input: impl Into<AgentInput>,
    ) -> Result<String, rucora_core::agent::AgentError> {
        self.execution.run_stream_text(input.into()).await
    }
}

impl<P> SummaryAgent<P> {
    #[must_use = "构建器必须调用 try_build() 来创建 Agent"]
    pub fn builder() -> SummaryAgentBuilder<P> {
        SummaryAgentBuilder::new()
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn mode(&self) -> &SummaryMode {
        &self.mode
    }
}

// ===== 请求构建方法 =====

impl<P> SummaryAgent<P> {
    fn build_single_request(&self, text: &str) -> ChatRequest {
        let content = self.prompt_template
            .replace("{text}", text)
            .replace("{mode}", self.mode.instruction());

        let mut messages = Vec::new();
        if let Some(ref prompt) = self.system_prompt {
            messages.push(ChatMessage::system(prompt.clone()));
        }
        messages.push(ChatMessage::user(content));

        let mut request = ChatRequest {
            messages,
            model: Some(self.model.clone()),
            tools: None,
            ..Default::default()
        };
        self.llm_params.apply_to(&mut request);
        request
    }

    fn build_multi_request(&self, context: &AgentContext, content: String) -> ChatRequest {
        let mut messages = context.messages.clone();

        if let Some(ref sys_prompt) = self.system_prompt
            && (messages.is_empty()
                || messages
                    .first()
                    .map(|m| &m.role)
                    != Some(&rucora_core::provider::types::Role::System))
        {
            messages.insert(0, ChatMessage::system(sys_prompt.clone()));
        }

        messages.push(ChatMessage::user(content));

        let mut request = ChatRequest {
            messages,
            model: Some(self.model.clone()),
            tools: None,
            ..Default::default()
        };
        self.llm_params.apply_to(&mut request);
        request
    }

    fn return_last_assistant(&self, context: &AgentContext) -> AgentDecision {
        let content = context
            .messages
            .iter()
            .rev()
            .find(|m| m.role == rucora_core::provider::types::Role::Assistant)
            .map(|m| m.content.clone())
            .unwrap_or_default();
        AgentDecision::Return(json!({"content": content}))
    }
}

/// SummaryAgent 构建器
pub struct SummaryAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    llm_params: LlmParams,
    mode: SummaryMode,
    chunk_size: usize,
    prompt_template: Option<String>,
    chunk_template: Option<String>,
    combine_template: Option<String>,
    middleware_chain: crate::middleware::MiddlewareChain,
}

impl<P> SummaryAgentBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            llm_params: LlmParams::default(),
            mode: SummaryMode::Concise,
            chunk_size: 4000,
            prompt_template: None,
            chunk_template: None,
            combine_template: None,
            middleware_chain: crate::middleware::MiddlewareChain::new(),
        }
    }
}

impl<P> SummaryAgentBuilder<P>
where
    P: LlmProvider + Send + Sync + 'static,
{
    pub fn provider(mut self, provider: P) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 设置摘要模式（默认：Concise）。
    ///
    /// 支持预定义模式和自定义指令：
    ///
    /// ```rust,ignore
    /// // 预定义模式
    /// .mode(SummaryMode::Concise)
    /// .mode(SummaryMode::BulletPoints)
    ///
    /// // 自定义指令
    /// .mode("请用英文总结以下文本，列出至少 5 个要点")
    /// ```
    pub fn mode(mut self, mode: impl Into<SummaryMode>) -> Self {
        self.mode = mode.into();
        self
    }

    /// 设置分块大小（字节数，默认 4000）。
    ///
    /// 当文本超过此大小时，自动分块处理。使用 `text-splitter` 在语义边界（段落、句子）处拆分。
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size.max(100);
        self
    }

    /// 设置单块摘要的提示词模板。
    ///
    /// 可用占位符：
    /// - `{text}` — 原文内容
    /// - `{mode}` — 模式指令文本
    ///
    /// 默认：`"请总结以下文本：\n\n{text}\n\n{mode}"`
    pub fn prompt_template(mut self, template: impl Into<String>) -> Self {
        self.prompt_template = Some(template.into());
        self
    }

    /// 设置分块摘要的提示词模板（map 阶段）。
    ///
    /// 可用占位符：
    /// - `{index}` — 当前块序号（从 1 开始）
    /// - `{total}` — 总块数
    /// - `{text}` — 当前块内容
    /// - `{mode}` — 模式指令文本
    ///
    /// 默认：`"以下是待总结文本的第 {index}/{total} 部分：\n\n{text}\n\n请总结以上文本部分的要点。"`
    pub fn chunk_template(mut self, template: impl Into<String>) -> Self {
        self.chunk_template = Some(template.into());
        self
    }

    /// 设置合并摘要的提示词模板（reduce 阶段）。
    ///
    /// 可用占位符：
    /// - `{total}` — 总块数
    /// - `{mode}` — 模式指令文本
    ///
    /// 默认：`"以上是文本各部分的局部总结。请将它们合并为一份完整的最终总结。\n\n{mode}"`
    pub fn combine_template(mut self, template: impl Into<String>) -> Self {
        self.combine_template = Some(template.into());
        self
    }

    pub fn temperature(mut self, value: f32) -> Self {
        self.llm_params.temperature = Some(value);
        self
    }

    pub fn top_p(mut self, value: f32) -> Self {
        self.llm_params.top_p = Some(value);
        self
    }

    pub fn top_k(mut self, value: u32) -> Self {
        self.llm_params.top_k = Some(value);
        self
    }

    pub fn max_tokens(mut self, value: u32) -> Self {
        self.llm_params.max_tokens = Some(value);
        self
    }

    pub fn frequency_penalty(mut self, value: f32) -> Self {
        self.llm_params.frequency_penalty = Some(value);
        self
    }

    pub fn presence_penalty(mut self, value: f32) -> Self {
        self.llm_params.presence_penalty = Some(value);
        self
    }

    pub fn stop(mut self, value: Vec<String>) -> Self {
        self.llm_params.stop = Some(value);
        self
    }

    pub fn extra_params(mut self, value: serde_json::Value) -> Self {
        self.llm_params.extra = Some(value);
        self
    }

    pub fn llm_params(mut self, params: LlmParams) -> Self {
        self.llm_params = params;
        self
    }

    pub fn with_middleware_chain(
        mut self,
        middleware_chain: crate::middleware::MiddlewareChain,
    ) -> Self {
        self.middleware_chain = middleware_chain;
        self
    }

    pub fn with_middleware<M: crate::middleware::Middleware + 'static>(
        mut self,
        middleware: M,
    ) -> Self {
        self.middleware_chain = self.middleware_chain.with(middleware);
        self
    }

    /// 尝试构建 Agent。
    pub fn try_build(self) -> Result<SummaryAgent<P>, AgentError> {
        let provider = self.provider.ok_or_else(|| {
            AgentError::Message("构建 SummaryAgent 失败：缺少 provider".to_string())
        })?;
        let model = self
            .model
            .ok_or_else(|| AgentError::Message("构建 SummaryAgent 失败：缺少 model".to_string()))?;

        let provider_arc = Arc::new(provider);
        let execution = build_default_execution(crate::agent::ExecutionBuildConfig {
            provider: provider_arc.clone(),
            model: model.clone(),
            tools: crate::agent::ToolRegistry::new(),
            system_prompt: self.system_prompt.clone(),
            max_steps: 20,
            max_tool_concurrency: 1,
            conversation_manager: None,
            middleware_chain: self.middleware_chain.clone(),
            enhanced_config: crate::agent::tool_call_config::ToolCallEnhancedConfig::default(),
            llm_params: self.llm_params.clone(),
        });

        Ok(SummaryAgent {
            provider: provider_arc,
            model,
            system_prompt: self.system_prompt,
            llm_params: self.llm_params,
            mode: self.mode,
            chunk_size: self.chunk_size,
            prompt_template: self.prompt_template.unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string()),
            chunk_template: self.chunk_template.unwrap_or_else(|| DEFAULT_CHUNK_TEMPLATE.to_string()),
            combine_template: self.combine_template.unwrap_or_else(|| DEFAULT_COMBINE_TEMPLATE.to_string()),
            execution,
        })
    }

    /// 构建 Agent。
    ///
    /// 推荐优先使用 [`Self::try_build`] 处理配置错误。
    pub fn build(self) -> SummaryAgent<P> {
        self.try_build()
            .unwrap_or_else(|err| panic!("SummaryAgentBuilder::build 失败：{err}"))
    }
}

impl<P> Default for SummaryAgentBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rucora_core::test_utils::MockProvider;

    #[test]
    fn test_summary_agent_builder() {
        let _agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .mode(SummaryMode::Concise)
            .chunk_size(2000)
            .build();
    }

    #[test]
    fn test_chunk_text_short() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .chunk_size(4000)
            .build();
        let text = "这是一段短文本。";
        let count = TextSplitter::new(agent.chunk_size).chunks(text).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_chunk_text_long() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .chunk_size(100)
            .build();
        let mut text = String::new();
        for i in 0..6 {
            text.push_str(&format!("第{}段落。{}", i + 1, "这是该段的内容说明。这里有一些补充信息用于填充。\n\n"));
        }
        let chunks: Vec<&str> = TextSplitter::new(agent.chunk_size).chunks(&text).collect();
        assert!(chunks.len() > 1, "长文本应被分块，但得到 {} 块", chunks.len());
        for chunk in &chunks {
            assert!(!chunk.is_empty(), "分块不应为空");
            assert!(text.contains(chunk), "每个分块应是原文的子串");
        }
    }

    #[test]
    fn test_summary_mode_instruction() {
        assert!(!SummaryMode::Concise.instruction().is_empty());
        assert!(!SummaryMode::Detailed.instruction().is_empty());
        assert!(!SummaryMode::BulletPoints.instruction().is_empty());
        assert!(!SummaryMode::KeyPoints.instruction().is_empty());
        assert!(!SummaryMode::Custom("自定义指令".to_string()).instruction().is_empty());
    }

    #[test]
    fn test_custom_mode() {
        let instruction = "请用英文总结，至少 5 个要点";
        let mode = SummaryMode::Custom(instruction.to_string());
        assert_eq!(mode.instruction(), instruction);
    }

    #[test]
    fn test_mode_from_str() {
        let mode: SummaryMode = "自定义指令".into();
        assert_eq!(mode, SummaryMode::Custom("自定义指令".to_string()));
    }

    #[test]
    fn test_custom_templates() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .prompt_template("自定义单块模板：{text} -- {mode}")
            .chunk_template("第{index}/{total}块：{text}")
            .combine_template("合并{total}块：{mode}")
            .build();
        assert!(agent.prompt_template.contains("自定义单块模板"));
        assert!(agent.chunk_template.contains("第{index}/{total}块"));
        assert!(agent.combine_template.contains("合并{total}块"));
    }

    #[test]
    fn test_chunk_integrity() {
        let mut text = String::new();
        for i in 0..10 {
            text.push_str(&format!("Paragraph {} with some content to make it longer than a very short chunk.\n\n", i + 1));
        }
        let chunks: Vec<&str> = TextSplitter::new(200).chunks(&text).collect();
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(text.contains(chunk), "分块 {chunk} 应是原文的子串");
        }
        for chunk in &chunks {
            assert_eq!(text.matches(chunk).count(), 1,
                "分块 {chunk} 应在原文中恰好出现一次");
        }
    }
}
