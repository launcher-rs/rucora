//! Deep Research 核心 trait 定义

use crate::provider::LlmProvider;
use async_trait::async_trait;
use std::sync::Arc;

use super::{
    InfoPiece, ResearchConfig, ResearchPhase, ResearchProgress, ResearchReport, 
    ScoringConfig, SuggestionType,
};

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
            && !self.visited_urls.contains(url) {
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
#[derive(Debug)]
pub enum ResearchError {
    /// Provider 错误
    Provider(String),
    /// 工具错误
    Tool(String),
    /// 超时
    Timeout,
    /// 无效配置
    InvalidConfig(String),
    /// 研究失败
    Failed(String),
    /// 存储错误
    Storage(String),
}

impl std::fmt::Display for ResearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchError::Provider(msg) => write!(f, "Provider 错误: {msg}"),
            ResearchError::Tool(msg) => write!(f, "工具错误: {msg}"),
            ResearchError::Timeout => write!(f, "超时"),
            ResearchError::InvalidConfig(msg) => write!(f, "无效配置: {msg}"),
            ResearchError::Failed(msg) => write!(f, "研究失败: {msg}"),
            ResearchError::Storage(msg) => write!(f, "存储错误: {msg}"),
        }
    }
}

impl std::error::Error for ResearchError {}

impl From<crate::error::AgentError> for ResearchError {
    fn from(e: crate::error::AgentError) -> Self {
        ResearchError::Failed(e.to_string())
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
#[allow(dead_code)]
pub struct DefaultCitationHandler;

impl DefaultCitationHandler {
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
        // 简单的 URL 提取正则
        let url_regex = regex::Regex::new(r"https?://[^\s\)]+").unwrap();
        let mut citations = Vec::new();

        for cap in url_regex.find_iter(content) {
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
pub trait StrategyFactory: Send + Sync {
    /// 创建策略
    fn create(&self, config: &ResearchConfig) -> Box<dyn StrategyTrait>;
}

/// 研究质量评估器
///
/// 负责评估研究质量并生成改进建议。
///
/// # 功能
///
/// 1. **质量评估** (`assess`) - 基于收集的信息和引用评估研究质量
/// 2. **建议生成** (`suggest`) - 根据评分生成具体的改进建议
/// 3. **继续判断** (`should_continue`) - 判断是否需要继续搜索
/// 4. **搜索提示** (`get_next_search_hint`) - 生成下一轮搜索的关键词提示
///
/// # 示例
///
/// ```rust
/// use rucora_core::research::ResearchQualityAssessor;
///
/// // 方式 1: 使用默认配置
/// let assessor = ResearchQualityAssessor::with_default("Rust 异步编程");
///
/// // 方式 2: 自定义配置
/// let config = ScoringConfig {
///     quality_threshold: 0.7,
///     ..Default::default()
/// };
/// let assessor = ResearchQualityAssessor::new(config, vec!["Rust".to_string()]);
///
/// // 评估质量
/// let score = assessor.assess(&info_pieces, &citations, 3);
///
/// // 生成建议
/// let suggestion = assessor.suggest(&score);
///
/// // 判断是否继续
/// if assessor.should_continue(&score, 3, 10) {
///     let next_hint = assessor.get_next_search_hint(&score, "Rust");
///     // 使用提示进行下一轮搜索
/// }
/// ```
pub struct ResearchQualityAssessor {
    config: ScoringConfig,
    topic_keywords: Vec<String>,
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

fn extract_keywords(topic: &str) -> Vec<String> {
    topic.split(&[' ', ',', '，', '。', '、', '？', '?'][..])
        .filter(|s| !s.is_empty() && s.len() > 1)
        .take(5)
        .map(|s| s.to_string())
        .collect()
}