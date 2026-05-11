//! Deep Research 类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 研究阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchPhase {
    /// 初始化阶段
    Init,
    /// 搜索阶段
    Search,
    /// 深度阅读阶段
    DeepRead,
    /// 综合阶段
    Synthesize,
    /// 完成
    Complete,
}

impl Default for ResearchPhase {
    fn default() -> Self {
        Self::Init
    }
}

impl std::fmt::Display for ResearchPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchPhase::Init => write!(f, "初始化"),
            ResearchPhase::Search => write!(f, "搜索"),
            ResearchPhase::DeepRead => write!(f, "深度阅读"),
            ResearchPhase::Synthesize => write!(f, "综合"),
            ResearchPhase::Complete => write!(f, "完成"),
        }
    }
}

/// 研究策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchStrategy {
    /// 快速模式（30秒-3分钟）
    Fast,
    /// 标准多阶段模式
    Standard,
    /// Agentic 自主模式
    Agentic,
    /// 研究库模式
    Library,
    /// 学术研究模式
    Academic,
}

impl Default for ResearchStrategy {
    fn default() -> Self {
        Self::Standard
    }
}

impl std::fmt::Display for ResearchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchStrategy::Fast => write!(f, "快速模式"),
            ResearchStrategy::Standard => write!(f, "标准模式"),
            ResearchStrategy::Agentic => write!(f, "自主模式"),
            ResearchStrategy::Library => write!(f, "研究库模式"),
            ResearchStrategy::Academic => write!(f, "学术模式"),
        }
    }
}

/// 信息片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoPiece {
    /// 内容
    pub content: String,
    /// 来源 URL
    pub source_url: Option<String>,
    /// 来源类型
    pub source_type: SourceType,
    ///  relevance_score: f32,
    pub relevance_score: f32,
    /// 采集时间
    pub collected_at: DateTime<Utc>,
}

impl InfoPiece {
    pub fn new(content: String, source_url: Option<String>, source_type: SourceType) -> Self {
        Self {
            content,
            source_url,
            source_type,
            relevance_score: 1.0,
            collected_at: Utc::now(),
        }
    }

    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance_score = score;
        self
    }
}

/// 来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    /// 新闻
    News,
    /// 学术论文
    Academic,
    /// 博客
    Blog,
    /// 官方网站
    Official,
    /// 社交媒体
    SocialMedia,
    /// 未知
    Unknown,
}

impl Default for SourceType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 引用条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// URL
    pub url: String,
    /// 标题
    pub title: String,
    /// 来源类型
    pub source_type: SourceType,
    /// 访问时间
    pub accessed_at: DateTime<Utc>,
    /// 摘要片段
    pub snippet: String,
    /// 相关性分数
    pub relevance_score: f32,
}

impl Citation {
    pub fn new(url: String, title: String, snippet: String) -> Self {
        Self {
            url,
            title,
            source_type: SourceType::Unknown,
            accessed_at: Utc::now(),
            snippet,
            relevance_score: 1.0,
        }
    }

    pub fn with_source_type(mut self, source_type: SourceType) -> Self {
        self.source_type = source_type;
        self
    }

    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance_score = score;
        self
    }

    /// 生成 APA 格式引用
    pub fn format_apa(&self) -> String {
        format!(
            "{} (访问于 {}). {}.",
            self.title,
            self.accessed_at.format("%Y年%m月%d日"),
            self.url
        )
    }

    /// 生成简单引用格式
    pub fn format_simple(&self) -> String {
        format!("[{}] {}", self.url, self.title)
    }
}

/// 搜索历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistory {
    /// 搜索查询
    pub query: String,
    /// 搜索时间
    pub searched_at: DateTime<Utc>,
    /// 结果数量
    pub result_count: usize,
}

impl SearchHistory {
    pub fn new(query: String, result_count: usize) -> Self {
        Self {
            query,
            searched_at: Utc::now(),
            result_count,
        }
    }
}

/// 研究报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    /// 报告 ID
    pub id: String,
    /// 标题
    pub title: String,
    /// 研究主题
    pub topic: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 摘要
    pub summary: String,
    /// 完整内容
    pub full_content: String,
    /// 引用列表
    pub citations: Vec<Citation>,
    /// 使用的策略
    pub strategy: ResearchStrategy,
    /// 使用的 token 数量
    pub tokens_used: u32,
    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl ResearchReport {
    pub fn new(topic: String, strategy: ResearchStrategy) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let title = format!("{} - 研究报告", topic);
        let now = Utc::now();

        Self {
            id,
            title,
            topic,
            created_at: now,
            updated_at: now,
            summary: String::new(),
            full_content: String::new(),
            citations: Vec::new(),
            strategy,
            tokens_used: 0,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.full_content = content;
        self
    }

    pub fn add_citation(&mut self, citation: Citation) {
        self.citations.push(citation);
    }

    pub fn set_tokens(&mut self, tokens: u32) {
        self.tokens_used = tokens;
    }

    /// 生成 Markdown 格式报告
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", self.title));
        md.push_str(&format!("**研究主题**: {}\n", self.topic));
        md.push_str(&format!(
            "**研究日期**: {}\n",
            self.created_at.format("%Y年%m月%d日")
        ));
        md.push_str(&format!("**研究策略**: {}\n", self.strategy));
        md.push_str(&format!("**消耗 Token**: {}\n\n", self.tokens_used));

        if !self.summary.is_empty() {
            md.push_str(&format!("## 摘要\n\n{}\n\n", self.summary));
        }

        md.push_str(&format!("## 研究内容\n\n{}\n\n", self.full_content));

        if !self.citations.is_empty() {
            md.push_str("## 参考来源\n\n");
            for (i, citation) in self.citations.iter().enumerate() {
                md.push_str(&format!("{}. {}\n\n", i + 1, citation.format_apa()));
            }
        }

        md
    }
}

/// 研究进度
#[derive(Debug, Clone, Default)]
pub struct ResearchProgress {
    /// 当前阶段
    pub phase: ResearchPhase,
    /// 当前步骤
    pub current_step: u32,
    /// 总步骤数
    pub total_steps: u32,
    /// 进度描述
    pub description: String,
    /// 已收集的信息数量
    pub info_count: usize,
    /// 已访问的 URL 数量
    pub url_count: usize,
}

impl ResearchProgress {
    pub fn new(phase: ResearchPhase, total_steps: u32) -> Self {
        Self {
            phase,
            current_step: 0,
            total_steps,
            description: String::new(),
            info_count: 0,
            url_count: 0,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_steps == 0 {
            return 0.0;
        }
        (self.current_step as f32 / self.total_steps as f32) * 100.0
    }
}

/// 研究配置
#[derive(Debug, Clone)]
pub struct ResearchConfig {
    /// 研究策略
    pub strategy: ResearchStrategy,
    /// 最大迭代次数
    pub max_iterations: u32,
    /// 最大并发搜索数
    pub max_concurrent_searches: u32,
    /// 搜索超时（秒）
    pub search_timeout: u64,
    /// 读取超时（秒）
    pub read_timeout: u64,
    /// 最大输出长度
    pub max_output_length: usize,
    /// 是否包含引用
    pub include_citations: bool,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            strategy: ResearchStrategy::Standard,
            max_iterations: 10,
            max_concurrent_searches: 3,
            search_timeout: 60,
            read_timeout: 45,
            max_output_length: 10000,
            include_citations: true,
        }
    }
}

impl ResearchConfig {
    pub fn fast() -> Self {
        Self {
            strategy: ResearchStrategy::Fast,
            max_iterations: 3,
            max_concurrent_searches: 2,
            search_timeout: 30,
            read_timeout: 20,
            max_output_length: 1000,
            include_citations: true,
        }
    }

    pub fn agentic() -> Self {
        Self {
            strategy: ResearchStrategy::Agentic,
            max_iterations: 20,
            max_concurrent_searches: 5,
            search_timeout: 120,
            read_timeout: 60,
            max_output_length: 50000,
            include_citations: true,
        }
    }

    pub fn academic() -> Self {
        Self {
            strategy: ResearchStrategy::Academic,
            max_iterations: 15,
            max_concurrent_searches: 3,
            search_timeout: 90,
            read_timeout: 60,
            max_output_length: 30000,
            include_citations: true,
        }
    }
}
