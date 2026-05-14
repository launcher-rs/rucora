//! Deep Research 核心 trait 定义
//!
//! 本模块定义了深度研究的核心抽象，包括：
//! - `DeepResearchEngine` - 研究引擎 trait
//! - `StrategyTrait` - 搜索策略 trait
//! - `ResearchQualityAssessor` - 研究质量评估器
//! - `CitationHandler` - 引用处理器

use crate::provider::LlmProvider;
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};

use super::{
    InfoPiece, ResearchConfig, ResearchPhase, ResearchProgress, ResearchReport,
    ScoringConfig, SuggestionType,
};

/// 提取 URL 的正则表达式，使用 OnceLock 避免重复编译。
fn url_regex() -> &'static regex::Regex {
    static INSTANCE: OnceLock<regex::Regex> = OnceLock::new();
    INSTANCE.get_or_init(|| regex::Regex::new(r"https?://[^\s\)]+").unwrap())
}

/// 深度研究引擎 trait
///
/// 负责执行完整的深度研究流程。
#[async_trait]
pub trait DeepResearchEngine: Send + Sync {
    /// 执行研究
    ///
    /// # 参数
    ///
    /// * `provider` - LLM Provider
    /// * `topic` - 研究主题
    ///
    /// # 返回
    ///
    /// 研究报告
    async fn research(
        &self,
        provider: &Arc<dyn LlmProvider>,
        topic: &str,
    ) -> Result<ResearchReport, ResearchError>;

    /// 获取研究进度
    fn progress(&self) -> ResearchProgress;
}

/// 研究策略 trait
///
/// 定义不同研究策略的行为。
#[async_trait]
pub trait StrategyTrait: Send + Sync {
    /// 策略名称
    fn name(&self) -> &'static str;

    /// 策略描述
    fn description(&self) -> &'static str;

    /// 执行搜索
    async fn search(
        &self,
        provider: &Arc<dyn LlmProvider>,
        topic: &str,
        context: &mut ResearchContext,
    ) -> Result<StrategyResult, ResearchError>;

    /// 判断是否需要继续搜索
    fn should_continue(&self, result: &StrategyResult) -> bool;

    /// 获取最大迭代次数
    fn max_iterations(&self) -> Option<u32>;

    /// 获取配置
    fn config(&self) -> &ResearchConfig;
}

/// 研究上下文
///
/// 贯穿整个研究流程的共享状态。
#[derive(Debug)]
pub struct ResearchContext {
    /// 研究主题
    pub topic: String,
    /// 当前阶段
    pub phase: ResearchPhase,
    /// 已收集的信息片段
    pub collected_info: Vec<super::InfoPiece>,
    /// 已访问的 URL
    pub visited_urls: Vec<String>,
    /// 搜索历史
    pub search_history: Vec<super::SearchHistory>,
    /// 引用列表
    pub citations: Vec<super::Citation>,
    /// 内部状态（用于策略特定数据）
    pub state: std::collections::HashMap<String, serde_json::Value>,
}

impl ResearchContext {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            phase: ResearchPhase::Init,
            collected_info: Vec::new(),
            visited_urls: Vec::new(),
            search_history: Vec::new(),
            citations: Vec::new(),
            state: std::collections::HashMap::new(),
        }
    }

    pub fn add_info(&mut self, info: super::InfoPiece) {
        if let Some(url) = &info.source_url
            && !self.visited_urls.contains(url)
        {
            self.visited_urls.push(url.clone());
        }
        self.collected_info.push(info);
    }

    pub fn add_citation(&mut self, citation: super::Citation) {
        if !self.citations.iter().any(|c| c.url == citation.url) {
            self.citations.push(citation);
        }
    }

pub fn has_visited(&self, url: &str) -> bool {
        self.visited_urls.contains(&url.to_string())
    }

    pub fn visit_url(&mut self, url: String) {
        if !self.visited_urls.contains(&url) {
            self.visited_urls.push(url);
        }
    }

    pub fn set_phase(&mut self, phase: ResearchPhase) {
        self.phase = phase;
    }

    pub fn add_search_history(&mut self, query: String, result_count: usize) {
        self.search_history.push(super::SearchHistory::new(query, result_count));
    }

    pub fn set_state(&mut self, key: &str, value: serde_json::Value) {
        self.state.insert(key.to_string(), value);
    }

    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// 收集的信息总字符数
    pub fn total_content_length(&self) -> usize {
        self.collected_info.iter().map(|i| i.content.len()).sum()
    }
}

/// 策略执行结果
#[derive(Debug)]
pub struct StrategyResult {
    /// 是否完成
    pub is_complete: bool,
    /// 新收集的信息
    pub new_info: Vec<super::InfoPiece>,
    /// 新发现的 URL
    pub discovered_urls: Vec<String>,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// 搜索次数
    pub search_count: u32,
    /// 使用的 token 数量
    pub tokens_used: u32,
}

impl Default for StrategyResult {
    fn default() -> Self {
        Self {
            is_complete: false,
            new_info: Vec::new(),
            discovered_urls: Vec::new(),
            confidence: 0.0,
            search_count: 0,
            tokens_used: 0,
        }
    }
}

impl StrategyResult {
    pub fn complete() -> Self {
        Self {
            is_complete: true,
            ..Default::default()
        }
    }

    pub fn complete_with(mut self, confidence: f32, tokens_used: u32, search_count: u32) -> Self {
        self.is_complete = true;
        self.confidence = confidence;
        self.tokens_used = tokens_used;
        self.search_count = search_count;
        self
    }

    pub fn with_info(mut self, info: Vec<super::InfoPiece>) -> Self {
        self.new_info = info;
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = tokens;
        self
    }
}

/// 研究错误
///
/// 在深度研究过程中可能出现的各类错误。
#[derive(Debug, thiserror::Error)]
pub enum ResearchError {
    /// Provider 错误
    Provider {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    /// 工具错误
    Tool {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    /// 超时
    Timeout,
    /// 无效配置
    InvalidConfig(String),
    /// 研究失败
    Failed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    /// 存储错误
    Storage {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl std::fmt::Display for ResearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchError::Provider { message, .. } => write!(f, "Provider 错误: {message}"),
            ResearchError::Tool { message, .. } => write!(f, "工具错误: {message}"),
            ResearchError::Timeout => write!(f, "超时"),
            ResearchError::InvalidConfig(msg) => write!(f, "无效配置: {msg}"),
            ResearchError::Failed { message, .. } => write!(f, "研究失败: {message}"),
            ResearchError::Storage { message, .. } => write!(f, "存储错误: {message}"),
        }
    }
}

impl From<crate::error::AgentError> for ResearchError {
    fn from(e: crate::error::AgentError) -> Self {
        ResearchError::Failed {
            message: e.to_string(),
            source: Some(Box::new(e)),
        }
    }
}

/// 研究库 trait
///
/// 负责存储和检索研究结果。
#[async_trait]
pub trait ResearchLibrary: Send + Sync {
    /// 保存研究结果
    async fn save(&self, report: &ResearchReport) -> Result<String, ResearchError>;

    /// 搜索历史研究
    async fn search(&self, query: &str) -> Result<Vec<ResearchReport>, ResearchError>;

    /// 获取特定研究
    async fn get(&self, id: &str) -> Result<Option<ResearchReport>, ResearchError>;

    /// 列出所有研究
    async fn list(&self, limit: usize) -> Result<Vec<ResearchReport>, ResearchError>;

    /// 删除研究
    async fn delete(&self, id: &str) -> Result<(), ResearchError>;
}

/// 引用处理器 trait
///
/// 负责处理和格式化引用。
pub trait CitationHandler: Send + Sync {
    /// 从内容中提取引用
    fn extract_citations(&self, content: &str) -> Vec<super::Citation>;

    /// 格式化单个引用
    fn format_citation(&self, citation: &super::Citation) -> String;

    /// 格式化引用列表
    fn format_reference_list(&self, citations: &[super::Citation]) -> String;
}

/// 默认引用处理器
///
/// 提供基于正则表达式的 URL 提取和引用格式化功能。
#[allow(dead_code)]
pub struct DefaultCitationHandler;

impl DefaultCitationHandler {
    /// 创建新的默认引用处理器实例
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultCitationHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CitationHandler for DefaultCitationHandler {
    fn extract_citations(&self, content: &str) -> Vec<super::Citation> {
        let mut citations = Vec::new();

        for cap in url_regex().find_iter(content) {
            let url = cap.as_str().to_string();
            // 避免重复
            if !citations.iter().any(|c: &super::Citation| c.url == url) {
                citations.push(super::Citation::new(
                    url,
                    "".to_string(),
                    "".to_string(),
                ));
            }
        }

        citations
    }

    fn format_citation(&self, citation: &super::Citation) -> String {
        citation.format_apa()
    }

    fn format_reference_list(&self, citations: &[super::Citation]) -> String {
        if citations.is_empty() {
            return "无可用引用".to_string();
        }

        let mut result = String::from("## 参考来源\n\n");
        for (i, citation) in citations.iter().enumerate() {
            result.push_str(&format!("{}. {}\n\n", i + 1, citation.format_apa()));
        }
        result
    }
}

/// 搜索策略工厂
///
/// 用于创建不同类型的搜索策略实例。
pub trait StrategyFactory: Send + Sync {
    /// 创建策略
    fn create(&self, config: &ResearchConfig) -> Box<dyn StrategyTrait>;
}

/// 研究质量评估器
///
/// 用于评估研究质量、生成改进建议。
pub struct ResearchQualityAssessor {
    /// 评分配置
    pub config: ScoringConfig,
    /// 主题关键词
    pub topic_keywords: Vec<String>,
}

impl ResearchQualityAssessor {
    pub fn new(config: ScoringConfig, topic_keywords: Vec<String>) -> Self {
        Self {
            config,
            topic_keywords,
        }
    }

    pub fn with_default(topic: &str) -> Self {
        Self {
            config: ScoringConfig::default(),
            topic_keywords: extract_keywords(topic),
        }
    }

    /// 评估研究质量
    pub fn assess(
        &self,
        info_pieces: &[InfoPiece],
        citations: &[super::Citation],
        search_rounds: usize,
    ) -> super::ResearchQualityScore {
        super::ResearchQualityScore::calculate(
            info_pieces,
            citations,
            search_rounds,
            &self.topic_keywords,
        )
    }

    /// 生成改进建议
    pub fn suggest(
        &self,
        score: &super::ResearchQualityScore,
    ) -> super::ResearchSuggestion {
        // 检查是否已达到目标
        if score.is_sufficient(self.config.quality_threshold) {
            return super::ResearchSuggestion::sufficient();
        }

        // 检查信息数量
        if score.details.info_count < self.config.min_info_count {
            return super::ResearchSuggestion::need_more_info(
                score.details.info_count,
                self.config.min_info_count,
            );
        }

        // 检查重复率
        if score.details.duplicate_ratio > self.config.duplicate_threshold {
            return super::ResearchSuggestion::need_new_angle(score.details.duplicate_ratio);
        }

        // 检查来源多样性
        if score.details.source_diversity < self.config.min_source_diversity {
            return super::ResearchSuggestion::need_more_sources(score.details.source_diversity);
        }

        // 检查置信度
        if score.confidence < self.config.confidence_threshold {
            return super::ResearchSuggestion::need_validation(score.confidence);
        }

        // 默认：信息不足
        super::ResearchSuggestion::need_more_info(
            score.details.info_count,
            self.config.min_info_count,
        )
    }

    /// 检查是否应该继续搜索
    pub fn should_continue(&self, score: &super::ResearchQualityScore, current_round: u32, max_rounds: u32) -> bool {
        // 达到最大轮次，不再继续
        if current_round >= max_rounds {
            return false;
        }

        // 已达到质量阈值，可以停止
        if score.is_sufficient(self.config.quality_threshold) {
            return false;
        }

        // 质量不足但还有轮次，继续
        true
    }

    /// 根据评分获取下一轮搜索建议
    pub fn get_next_search_hint(&self, score: &super::ResearchQualityScore, current_topic: &str) -> String {
        let suggestion = self.suggest(score);

        let mut hints = vec![current_topic.to_string()];

        // 添加主题关键词
        if !self.topic_keywords.is_empty() {
            hints.extend(self.topic_keywords.iter().take(2).cloned());
        }

        // 根据建议类型添加关键词
        match suggestion.suggestion_type {
            SuggestionType::NeedMoreInfo => {
                hints.push("详细介绍".to_string());
                hints.push("详细说明".to_string());
            }
            SuggestionType::NeedMoreSources => {
                hints.extend(suggestion.suggested_keywords);
            }
            SuggestionType::NeedNewAngle => {
                hints.extend(suggestion.suggested_keywords);
            }
            SuggestionType::NeedValidation => {
                hints.push("官方".to_string());
                hints.push("验证".to_string());
            }
            SuggestionType::Sufficient => {
                return "研究已完成".to_string();
            }
        }

        hints.join(" ")
    }
}

/// 提取中文/英文关键词用于主题分析。
///
/// 按常见分隔符（空格、逗号、中文标点等）分割主题文本，
/// 过滤短词（<=1字符），最多返回 5 个关键词。
///
/// # 参数
///
/// - `topic`: 研究主题文本
///
/// # 返回
///
/// 关键词列表，最多 5 个
///
/// # 示例
///
/// ```
/// let keywords = extract_keywords("Rust 异步编程");
/// assert!(keywords.len() <= 5);
/// ```
pub(crate) fn extract_keywords(topic: &str) -> Vec<String> {
    topic.split(&[' ', ',', '，', '。', '、', '？', '?'][..])
        .filter(|s| !s.is_empty() && s.len() > 1)
        .take(5)
        .map(|s| s.to_string())
        .collect()
}