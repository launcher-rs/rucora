//! Deep Research 核心 trait 定义

use crate::provider::LlmProvider;
use async_trait::async_trait;
use std::sync::Arc;

use super::{ResearchConfig, ResearchPhase, ResearchProgress, ResearchReport};

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
        if let Some(url) = &info.source_url {
            if !self.visited_urls.contains(url) {
                self.visited_urls.push(url.clone());
            }
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
            ResearchError::Provider(msg) => write!(f, "Provider 错误: {}", msg),
            ResearchError::Tool(msg) => write!(f, "工具错误: {}", msg),
            ResearchError::Timeout => write!(f, "超时"),
            ResearchError::InvalidConfig(msg) => write!(f, "无效配置: {}", msg),
            ResearchError::Failed(msg) => write!(f, "研究失败: {}", msg),
            ResearchError::Storage(msg) => write!(f, "存储错误: {}", msg),
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

/// 搜索策略工厂
pub trait StrategyFactory: Send + Sync {
    /// 创建策略
    fn create(&self, config: &ResearchConfig) -> Box<dyn StrategyTrait>;
}