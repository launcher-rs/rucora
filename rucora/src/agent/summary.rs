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
use rucora_core::provider::types::{ChatMessage, ChatRequest, LlmParams, Role, Usage};
use serde_json::json;
use std::sync::Arc;
use text_splitter::{ChunkConfig, ChunkSizer, TextSplitter};
use tracing::{debug, info, warn};

use crate::agent::execution::DefaultExecution;
use crate::agent::tool_registry::ToolRegistry;

/// 类型擦除的 ChunkSizer 包装。
/// 类型擦除的 ChunkSizer 包装（内部实现，不推荐直接使用）。
pub struct DynSizer {
    pub(crate) inner: Box<dyn ChunkSizer + Send + Sync>,
}

impl ChunkSizer for DynSizer {
    fn size(&self, chunk: &str) -> usize {
        self.inner.size(chunk)
    }
}

/// 类型擦除的 TextSplitter。
///
/// 用户可通过 `text_splitter()` 或 `text_splitter_with_sizer()` 创建。
pub type DynTextSplitter = TextSplitter<DynSizer>;

/// 创建基于字符数的 TextSplitter。
///
/// ```rust,ignore
/// use rucora::agent::summary::text_splitter;
///
/// let splitter = text_splitter(2000);
/// ```
pub fn text_splitter(capacity: usize) -> DynTextSplitter {
    TextSplitter::new(ChunkConfig::new(capacity).with_sizer(DynSizer {
        inner: Box::new(text_splitter::Characters),
    }))
}

/// 创建带自定义 Sizer 的 TextSplitter。
///
/// ```rust,ignore
/// use rucora::agent::summary::text_splitter_with_sizer;
///
/// let splitter = text_splitter_with_sizer(500, my_tokenizer);
/// ```
pub fn text_splitter_with_sizer<S: ChunkSizer + Send + Sync + 'static>(
    capacity: usize,
    sizer: S,
) -> DynTextSplitter {
    TextSplitter::new(ChunkConfig::new(capacity).with_sizer(DynSizer {
        inner: Box::new(sizer),
    }))
}

/// 创建带重叠的 TextSplitter。
///
/// ```rust,ignore
/// use rucora::agent::text_splitter_with_overlap;
///
/// // 按 300 字符分块，块间重叠 30 字符
/// let splitter = text_splitter_with_overlap(300, 30);
/// ```
///
/// # Panics
///
/// 如果 overlap >= capacity 则 panic。
pub fn text_splitter_with_overlap(capacity: usize, overlap: usize) -> DynTextSplitter {
    TextSplitter::new(
        ChunkConfig::new(capacity)
            .with_overlap(overlap)
            .expect("重叠应小于块大小")
            .with_sizer(DynSizer {
                inner: Box::new(text_splitter::Characters),
            }),
    )
}

/// 分块摘要结果。
#[derive(Debug, Clone)]
pub struct ChunkSummary {
    /// 块索引（从 0 开始）。
    pub index: usize,
    /// 摘要内容。
    pub summary: String,
    /// Token 使用统计。
    pub usage: Option<Usage>,
}

/// 渲染模板字符串。
///
/// 将模板中的 `{key}` 占位符替换为对应的值。
fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        result = result.replace(key, value);
    }
    result
}

/// 文本分块器 trait。
///
/// 将长文本分割为适合 LLM 处理的小块。
/// SummaryAgent 通过此 trait 解耦具体的文本分块策略。
pub trait TextChunker: Send + Sync {
    /// 将文本分割为块。
    fn chunk_text<'a>(&self, text: &'a str) -> Vec<&'a str>;
}

impl TextChunker for DynTextSplitter {
    fn chunk_text<'a>(&self, text: &'a str) -> Vec<&'a str> {
        self.chunks(text).collect()
    }
}

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
            Self::Concise => {
                "请用简洁的 2-3 句话总结以下文本的核心内容。直接给出总结，不要添加额外说明。"
            }
            Self::Detailed => {
                "请详细总结以下文本的内容，保留重要细节、论据和结论。确保覆盖所有关键部分。"
            }
            Self::BulletPoints => "请用要点列表形式总结以下文本的核心内容。每个要点应独立且完整。",
            Self::KeyPoints => {
                "请提取以下文本的关键信息和核心观点。只列出最重要的内容，忽略次要细节。"
            }
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
以下是文本各部分的局部总结：\n\
\n\
{summaries}\n\
\n\
请将以上 {total} 份局部总结合并为一份完整的最终总结。\n\
\n\
{mode}";

/// SummaryAgent - 文本摘要 Agent
///
/// 特点：
/// - 长文本自动分块处理（map-reduce）
/// - 多模式输出（简洁、详细、要点、关键信息、自定义）
/// - 纯 LLM 调用，不依赖工具
/// - 支持自定义提示词模板
/// - 分块摘要支持并发处理，加快速度
/// - 支持任意 text-splitter 分词器（字符、Token、Markdown 等）
pub struct SummaryAgent<P> {
    provider: Arc<P>,
    model: Option<String>,
    system_prompt: Option<String>,
    llm_params: LlmParams,
    mode: SummaryMode,
    splitter: DynTextSplitter,
    max_concurrency: usize,
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

        let chunks = self.split_text(text);
        let total = chunks.len();
        let max_steps = if total <= 1 { 1 } else { 2 }; // MapAll(0) → Reduce(1) → Return(2)

        info!(
            text_len = text.len(),
            total_chunks = total,
            step,
            max_steps,
            "SummaryAgent::think"
        );

        match step {
            0 if total <= 1 => {
                debug!("单块模式，直接发送摘要请求");
                AgentDecision::Chat {
                    request: Box::new(self.build_single_request(text)),
                }
            }
            0 => {
                debug!(
                    "多块模式，并发处理 {} 块（并发数 {}）",
                    total, self.max_concurrency
                );
                let requests: Vec<ChatRequest> = chunks
                    .iter()
                    .enumerate()
                    .map(|(i, chunk)| {
                        let content = render_template(
                            &self.chunk_template,
                            &[
                                ("{index}", &(i + 1).to_string()),
                                ("{total}", &total.to_string()),
                                ("{text}", chunk),
                                ("{mode}", self.mode.instruction()),
                            ],
                        );
                        let mut messages = Vec::new();
                        if let Some(ref prompt) = self.system_prompt {
                            messages.push(ChatMessage::system(prompt.clone()));
                        }
                        messages.push(ChatMessage::user(content));
                        let mut request = ChatRequest {
                            messages,
                            model: self.model.clone(),
                            tools: None,
                            ..Default::default()
                        };
                        self.llm_params.apply_to(&mut request);
                        request
                    })
                    .collect();
                AgentDecision::MapAll {
                    requests,
                    max_concurrency: self.max_concurrency,
                }
            }
            1 if total > 1 => {
                let summaries = self.extract_chunk_summaries(context);
                let content = render_template(
                    &self.combine_template,
                    &[
                        ("{summaries}", &summaries),
                        ("{total}", &total.to_string()),
                        ("{mode}", self.mode.instruction()),
                    ],
                );
                info!(
                    summary_count = summaries.lines().count(),
                    summaries_len = summaries.len(),
                    "多块模式，进入合并阶段（{} 个局部摘要）",
                    total
                );
                AgentDecision::Reduce {
                    request: Box::new(self.build_multi_request(context, content)),
                }
            }
            _ => {
                debug!("步骤 {}，返回最终结果", step);
                self.return_last_assistant(context)
            }
        }
    }

    fn name(&self) -> &str {
        "summary_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("文本摘要 Agent，支持长文档自动分块和多模式输出")
    }

    /// 委托执行器运行。
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        info!(
            text_len = input.text().len(),
            "SummaryAgent::run 委托给 DefaultExecution"
        );
        self.execution.run(self, input).await
    }

    fn run_stream(
        &self,
        input: AgentInput,
    ) -> futures_util::stream::BoxStream<
        'static,
        Result<rucora_core::channel::types::ChannelEvent, AgentError>,
    > {
        // 使用基础流式执行器，不包含分块-合并摘要逻辑
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
    ) -> Result<String, AgentError> {
        let output = self.run(input.into()).await?;
        Ok(output.text_unwrap().to_string())
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

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn mode(&self) -> &SummaryMode {
        &self.mode
    }

    pub fn splitter(&self) -> &DynTextSplitter {
        &self.splitter
    }

    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// 将文本分割为块。
    pub fn split_text<'a>(&self, text: &'a str) -> Vec<&'a str> {
        self.splitter.chunks(text).collect()
    }
}

// ===== 请求构建方法 =====

impl<P> SummaryAgent<P> {
    fn build_single_request(&self, text: &str) -> ChatRequest {
        let content = render_template(
            &self.prompt_template,
            &[("{text}", text), ("{mode}", self.mode.instruction())],
        );

        let mut messages = Vec::new();
        if let Some(ref prompt) = self.system_prompt {
            messages.push(ChatMessage::system(prompt.clone()));
        }
        messages.push(ChatMessage::user(content));

        let mut request = ChatRequest {
            messages,
            model: self.model.clone(),
            tools: None,
            ..Default::default()
        };
        self.llm_params.apply_to(&mut request);
        request
    }

    fn build_multi_request(&self, context: &AgentContext, content: String) -> ChatRequest {
        let mut messages = context.messages.clone();

        if let Some(ref sys_prompt) = self.system_prompt
            && (messages.is_empty() || messages.first().map(|m| &m.role) != Some(&Role::System))
        {
            messages.insert(0, ChatMessage::system(sys_prompt.clone()));
        }

        messages.push(ChatMessage::user(content));

        debug!(
            message_count = messages.len(),
            last_user_content_len = messages
                .iter()
                .rev()
                .find(|m| m.role == Role::User)
                .map_or(0, |m| m.content_text().len()),
            "构建多块请求"
        );

        let mut request = ChatRequest {
            messages,
            model: self.model.clone(),
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
            .find(|m| m.role == Role::Assistant)
            .map(|m| m.content_text().to_string())
            .unwrap_or_default();
        AgentDecision::Return(json!({"content": content}))
    }

    /// 从对话历史中提取各块的局部摘要。
    ///
    /// 在多块模式下，每个块的摘要由 LLM 以 assistant 消息返回。
    /// 此方法收集所有 assistant 消息（跳过 system 消息）作为局部摘要。
    fn extract_chunk_summaries(&self, context: &AgentContext) -> String {
        let mut summaries = Vec::new();
        let mut chunk_index = 1;

        for msg in &context.messages {
            if msg.role == Role::Assistant && !msg.content_text().trim().is_empty() {
                summaries.push(format!("【部分 {}】\n{}", chunk_index, msg.content_text()));
                chunk_index += 1;
            }
        }

        if summaries.is_empty() {
            warn!("未找到任何局部摘要，合并阶段可能无法正常工作");
            return String::from("（未找到局部摘要）");
        }

        summaries.join("\n\n")
    }
}

/// SummaryAgent 构建器
pub struct SummaryAgentBuilder<P> {
    provider: Option<P>,
    system_prompt: Option<String>,
    model: Option<String>,
    llm_params: LlmParams,
    mode: SummaryMode,
    splitter: Option<DynTextSplitter>,
    max_concurrency: usize,
    prompt_template: Option<String>,
    chunk_template: Option<String>,
    combine_template: Option<String>,
}

impl<P> SummaryAgentBuilder<P> {
    pub fn new() -> Self {
        Self {
            provider: None,
            system_prompt: None,
            model: None,
            llm_params: LlmParams::default(),
            mode: SummaryMode::Concise,
            splitter: None,
            max_concurrency: 8,
            prompt_template: None,
            chunk_template: None,
            combine_template: None,
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

    /// 设置分块大小（字符数，默认 4000）。
    ///
    /// 当文本超过此大小时，自动分块处理。使用 `text-splitter` 在语义边界（段落、句子）处拆分。
    /// 注意：`text-splitter` 按字符数计算，中文一个字算一个字符。
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.splitter = Some(text_splitter(size));
        self
    }

    /// 设置自定义 TextSplitter 分词器。
    ///
    /// 传入一个 `DynTextSplitter` 实例，完全控制分词行为。
    /// 可使用 `text_splitter()` 或 `text_splitter_with_sizer()` 辅助函数创建。
    ///
    /// ```rust,ignore
    /// use rucora::agent::summary::{text_splitter, text_splitter_with_sizer};
    ///
    /// // 字符数分词
    /// .splitter(text_splitter(2000))
    ///
    /// // 自定义 Sizer（如 tiktoken）
    /// .splitter(text_splitter_with_sizer(500, my_tokenizer))
    /// ```
    pub fn splitter(mut self, splitter: DynTextSplitter) -> Self {
        self.splitter = Some(splitter);
        self
    }

    /// 设置最大并发数（默认 8）。
    ///
    /// 分块摘要时，同时处理的最大块数。增加并发数可以加快处理速度，
    /// 但会增加 API 的并发压力。建议根据 API 限流和网络状况调整。
    pub fn max_concurrency(mut self, concurrency: usize) -> Self {
        self.max_concurrency = concurrency.max(1);
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
    /// - `{summaries}` — 所有块的局部摘要（自动从对话历史中提取）
    /// - `{total}` — 总块数
    /// - `{mode}` — 模式指令文本
    ///
    /// 默认：`"以下是文本各部分的局部总结：\n\n{summaries}\n\n请将以上 {total} 份局部总结合并为一份完整的最终总结。\n\n{mode}"`
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

    /// 尝试构建 Agent。
    pub fn try_build(self) -> Result<SummaryAgent<P>, AgentError> {
        let provider = self.provider.ok_or_else(|| {
            AgentError::Message("构建 SummaryAgent 失败：缺少 provider".to_string())
        })?;
        let splitter = self.splitter.unwrap_or_else(|| text_splitter(4000));

        let provider_arc = Arc::new(provider);

        let execution = DefaultExecution::new(
            provider_arc.clone() as Arc<dyn LlmProvider>,
            self.model.clone(),
            ToolRegistry::new(),
        )
        .with_llm_params(self.llm_params.clone())
        .with_system_prompt_opt(self.system_prompt.clone())
        .with_max_steps(10);

        Ok(SummaryAgent {
            provider: provider_arc,
            model: self.model,
            system_prompt: self.system_prompt,
            llm_params: self.llm_params,
            mode: self.mode,
            splitter,
            max_concurrency: self.max_concurrency,
            prompt_template: self
                .prompt_template
                .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string()),
            chunk_template: self
                .chunk_template
                .unwrap_or_else(|| DEFAULT_CHUNK_TEMPLATE.to_string()),
            combine_template: self
                .combine_template
                .unwrap_or_else(|| DEFAULT_COMBINE_TEMPLATE.to_string()),
            execution,
        })
    }

    /// 构建 Agent。
    ///
    /// 推荐优先使用 [`Self::try_build`] 处理配置错误。
    #[deprecated(note = "请使用 try_build() 处理配置错误")]
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
#[allow(deprecated)]
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
        let count = agent.splitter.chunks(text).count();
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
            text.push_str(&format!(
                "第{}段落。{}",
                i + 1,
                "这是该段的内容说明。这里有一些补充信息用于填充。\n\n"
            ));
        }
        let chunks: Vec<&str> = agent.splitter.chunks(&text).collect();
        assert!(
            chunks.len() > 1,
            "长文本应被分块，但得到 {} 块",
            chunks.len()
        );
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
        assert!(
            !SummaryMode::Custom("自定义指令".to_string())
                .instruction()
                .is_empty()
        );
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
    fn test_long_text_chunking_flow() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .chunk_size(100)
            .mode(SummaryMode::Concise)
            .build();

        let mut text = String::new();
        for i in 0..10 {
            text.push_str(&format!(
                "第{}段落。这是该段的内容说明。这里有一些补充信息用于填充文本长度。\n\n",
                i + 1
            ));
        }

        let chunks: Vec<&str> = agent.splitter.chunks(&text).collect();
        let total = chunks.len();

        assert!(total > 1, "长文本应被分为多块，实际得到 {total} 块");
        eprintln!("文本长度: {} 字符，分为 {total} 块", text.len());

        for (i, chunk) in chunks.iter().enumerate() {
            let preview: String = chunk.chars().take(20).collect();
            eprintln!("块 {}: 长度 {} 字符，预览: {preview}", i + 1, chunk.len());
            assert!(!chunk.is_empty(), "块 {i} 不应为空");
        }

        let combine_summaries = (1..=total)
            .map(|i| format!("【部分 {i}】\nMock response"))
            .collect::<Vec<_>>()
            .join("\n\n");
        assert!(
            combine_summaries.contains("【部分 1】")
                && combine_summaries.contains(&format!("【部分 {total}】")),
            "摘要应包含 {total} 个部分标记",
        );
    }

    #[test]
    fn test_extract_chunk_summaries() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .build();

        let input = AgentInput::new("测试文本").unwrap();
        let mut context = rucora_core::agent::AgentContext::new(input, 10);

        context.messages.push(ChatMessage::system("系统提示"));
        context.messages.push(ChatMessage::user("用户消息"));
        context
            .messages
            .push(ChatMessage::assistant("第一个块的摘要"));
        context.messages.push(ChatMessage::user("第二块请求"));
        context
            .messages
            .push(ChatMessage::assistant("第二个块的摘要"));

        let summaries = agent.extract_chunk_summaries(&context);
        assert!(summaries.contains("第一个块的摘要"), "应包含第一个摘要");
        assert!(summaries.contains("第二个块的摘要"), "应包含第二个摘要");
        assert!(summaries.contains("【部分 1】"), "应有部分标记");
        assert!(summaries.contains("【部分 2】"), "应有部分标记");
    }

    #[test]
    fn test_empty_summaries_warning() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .build();

        let input = AgentInput::new("测试文本").unwrap();
        let context = rucora_core::agent::AgentContext::new(input, 10);

        let summaries = agent.extract_chunk_summaries(&context);
        assert!(summaries.contains("未找到"), "无摘要时应返回提示信息");
    }

    #[test]
    fn test_text_splitter() {
        let splitter = text_splitter(1000);
        let text = "Hello world. This is a test. ".repeat(10);
        let chunks: Vec<&str> = splitter.chunks(&text).collect();
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.is_empty());
        }
    }

    #[test]
    fn test_text_splitter_with_builder() {
        let agent = SummaryAgentBuilder::<MockProvider>::new()
            .provider(MockProvider)
            .model("gpt-4o-mini")
            .splitter(text_splitter(2000))
            .build();

        let text = "Hello world. This is a test. ".repeat(200);
        assert!(agent.splitter.chunks(&text).next().is_some());
    }

    #[test]
    fn test_text_splitter_with_overlap() {
        let splitter = text_splitter_with_overlap(100, 20);
        let text = "Hello world. This is a test. ".repeat(10);
        assert!(splitter.chunks(&text).next().is_some());
    }

    #[test]
    #[should_panic(expected = "重叠应小于块大小")]
    fn test_text_splitter_with_overlap_panics_on_invalid() {
        text_splitter_with_overlap(50, 100);
    }

    #[test]
    fn test_chunk_integrity() {
        let mut text = String::new();
        for i in 0..10 {
            text.push_str(&format!(
                "Paragraph {} with some content to make it longer than a very short chunk.\n\n",
                i + 1
            ));
        }
        let splitter = text_splitter(200);
        let chunks: Vec<&str> = splitter.chunks(&text).collect();
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(text.contains(chunk), "分块 {chunk} 应是原文的子串");
        }
        for chunk in &chunks {
            assert_eq!(
                text.matches(chunk).count(),
                1,
                "分块 {chunk} 应在原文中恰好出现一次"
            );
        }
    }
}
