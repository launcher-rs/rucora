//! Deep Research 类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 研究阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResearchPhase {
    /// 初始化阶段
    #[default]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResearchStrategy {
    /// 快速模式（30秒-3分钟）
    Fast,
    /// 标准多阶段模式
    #[default]
    Standard,
    /// Agentic 自主模式
    Agentic,
    /// 研究库模式
    Library,
    /// 学术研究模式
    Academic,
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
            relevance_score: 0.5,
            collected_at: Utc::now(),
        }
    }

    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance_score = score;
        self
    }
}

/// 来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
    #[default]
    Unknown,
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
            relevance_score: 0.5,
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
        let title = format!("{topic} - 研究报告");
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

    /// 批量添加引用
    pub fn add_citations(&mut self, citations: impl IntoIterator<Item = Citation>) {
        self.citations.extend(citations);
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

/// 研究质量评分
///
/// 用于评估深度研究的质量，综合考虑信息质量、完整性、置信度等因素。
///
/// # 计算公式
///
/// - `info_quality`: 高质量信息占比 × 平均相关性分数
/// - `completeness`: 主题覆盖率（基于关键词匹配）
/// - `confidence`: 信息质量×0.4 + 完整性×0.3 + 来源多样性×0.3
/// - `overall`: 信息质量×0.3 + 完整性×0.4 + 置信度×0.3
///
/// # 评级
///
/// - 优秀: overall >= 0.8
/// - 良好: overall >= 0.6
/// - 一般: overall >= 0.4
/// - 需改进: overall < 0.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQualityScore {
    /// 信息质量评分 (0.0 - 1.0)
    pub info_quality: f32,
    /// 研究完整性评分 (0.0 - 1.0)
    pub completeness: f32,
    /// 置信度评分 (0.0 - 1.0)
    pub confidence: f32,
    /// 综合评分 (0.0 - 1.0)
    pub overall: f32,
    /// 评分详情
    pub details: ScoreDetails,
}

/// 评分详情
///
/// 包含评估过程中的各项统计数据，用于分析研究质量的具体方面。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreDetails {
    /// 信息数量
    pub info_count: usize,
    /// 高质量信息数量（相关性 >= 0.7）
    pub high_quality_count: usize,
    /// 来源多样性（不同来源类型的数量）
    pub source_diversity: usize,
    /// 引用数量
    pub citation_count: usize,
    /// 搜索轮次
    pub search_rounds: usize,
    /// 信息覆盖的主题数
    pub topic_coverage: usize,
    /// 重复信息比例 (0.0 - 1.0)
    pub duplicate_ratio: f32,
}

impl ResearchQualityScore {
    pub fn zero() -> Self {
        Self {
            info_quality: 0.0,
            completeness: 0.0,
            confidence: 0.0,
            overall: 0.0,
            details: ScoreDetails::default(),
        }
    }

    pub fn calculate(
        info_pieces: &[InfoPiece],
        citations: &[Citation],
        search_rounds: usize,
        topic_keywords: &[String],
    ) -> Self {
        let info_count = info_pieces.len();
        let citation_count = citations.len();

        // 信息质量评分
        let high_quality_count = info_pieces
            .iter()
            .filter(|i| i.relevance_score >= 0.7)
            .count();
        let info_quality = if info_count > 0 {
            (high_quality_count as f32 / info_count as f32)
                * info_pieces.iter().map(|i| i.relevance_score).sum::<f32>()
                / info_count as f32
        } else {
            0.0
        };

        // 来源多样性
        let source_diversity = info_pieces
            .iter()
            .map(|i| i.source_type)
            .collect::<std::collections::HashSet<_>>()
            .len();

        // 重复信息检测
        let mut content_set = std::collections::HashSet::new();
        let mut duplicate_count = 0;
        for info in info_pieces {
            let mut hasher = DefaultHasher::new();
            info.content.hash(&mut hasher);
            let hash = hasher.finish();
            if content_set.contains(&hash) {
                duplicate_count += 1;
            } else {
                content_set.insert(hash);
            }
        }
        let duplicate_ratio = if info_count > 0 {
            duplicate_count as f32 / info_count as f32
        } else {
            0.0
        };

        // 主题覆盖度 (简化版：检查关键词出现频率)
        let topic_coverage = topic_keywords
            .iter()
            .filter(|kw| {
                info_pieces
                    .iter()
                    .any(|i| i.content.to_lowercase().contains(&kw.to_lowercase()))
            })
            .count();

        // 完整性评分
        let completeness = if !topic_keywords.is_empty() {
            (topic_coverage as f32 / topic_keywords.len() as f32).min(1.0)
        } else if info_count > 5 {
            0.8
        } else {
            0.3
        };

        // 置信度评分
        let confidence =
            (info_quality * 0.4 + completeness * 0.3 + (source_diversity as f32 / 10.0).min(0.3))
                .min(1.0);

        // 综合评分
        let overall = info_quality * 0.3 + completeness * 0.4 + confidence * 0.3;

        let details = ScoreDetails {
            info_count,
            high_quality_count,
            source_diversity,
            citation_count,
            search_rounds,
            topic_coverage,
            duplicate_ratio,
        };

        Self {
            info_quality: (info_quality * 100.0).round() / 100.0,
            completeness: (completeness * 100.0).round() / 100.0,
            confidence: (confidence * 100.0).round() / 100.0,
            overall: (overall * 100.0).round() / 100.0,
            details,
        }
    }

    pub fn level(&self) -> &'static str {
        if self.overall >= 0.8 {
            "优秀"
        } else if self.overall >= 0.6 {
            "良好"
        } else if self.overall >= 0.4 {
            "一般"
        } else {
            "需改进"
        }
    }

    pub fn is_sufficient(&self, threshold: f32) -> bool {
        self.overall >= threshold
    }
}

/// 研究改进建议
///
/// 根据当前研究质量评分生成的改进建议，帮助指导后续研究流程。
///
/// # 用法
///
/// ```rust
/// use rucora_core::research::{ResearchQualityAssessor, SuggestionType, InfoPiece, SourceType};
///
/// let assessor = ResearchQualityAssessor::with_default("test topic");
/// let info_pieces = vec![
///     InfoPiece::new("content".to_string(), None, SourceType::News),
/// ];
/// let citations = vec![];
/// let score = assessor.assess(&info_pieces, &citations, 1);
/// let suggestion = assessor.suggest(&score);
///
/// match suggestion.suggestion_type {
///     SuggestionType::Sufficient => { /* 可以停止研究 */ }
///     SuggestionType::NeedMoreInfo => { /* 继续搜索更多信息 */ }
///     SuggestionType::NeedMoreSources => { /* 尝试不同来源 */ }
///     SuggestionType::NeedNewAngle => { /* 换个搜索角度 */ }
///     SuggestionType::NeedValidation => { /* 验证信息准确性 */ }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSuggestion {
    /// 建议类型
    pub suggestion_type: SuggestionType,
    /// 建议描述
    pub description: String,
    /// 推荐的搜索关键词
    pub suggested_keywords: Vec<String>,
    /// 优先级 (1-5, 5为最高)
    pub priority: u8,
}

/// 建议类型
///
/// 根据评分结果生成的具体改进建议类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionType {
    /// 信息不足，需要更多搜索
    ///
    /// 当收集的信息数量少于最小要求时触发。
    NeedMoreInfo,
    /// 来源单一，需要多元化
    ///
    /// 当来源类型过于单一时触发，建议拓展不同类型的来源。
    NeedMoreSources,
    /// 重复信息过多，需要新角度
    ///
    /// 当重复信息比例超过阈值时触发，建议换个角度搜索。
    NeedNewAngle,
    /// 置信度不足，需要深入验证
    ///
    /// 当置信度低于阈值时触发，建议通过权威来源验证信息。
    NeedValidation,
    /// 已达到目标
    ///
    /// 当研究质量达到预设阈值时触发，可以结束研究流程。
    Sufficient,
}

impl ResearchSuggestion {
    pub fn sufficient() -> Self {
        Self {
            suggestion_type: SuggestionType::Sufficient,
            description: "研究质量已达到目标要求".to_string(),
            suggested_keywords: vec![],
            priority: 5,
        }
    }

    pub fn need_more_info(current_count: usize, target: usize) -> Self {
        Self {
            suggestion_type: SuggestionType::NeedMoreInfo,
            description: format!("信息量不足 ({current_count}/{target}), 建议继续搜索"),
            suggested_keywords: vec![],
            priority: 4,
        }
    }

    pub fn need_more_sources(diversity: usize) -> Self {
        let keywords = match diversity {
            0 | 1 => vec![
                "官方文档".to_string(),
                "学术论文".to_string(),
                "技术博客".to_string(),
            ],
            2 => vec!["案例分析".to_string(), "社区讨论".to_string()],
            _ => vec![],
        };
        Self {
            suggestion_type: SuggestionType::NeedMoreSources,
            description: format!("来源类型较少 ({diversity}种), 建议拓展来源"),
            suggested_keywords: keywords,
            priority: 3,
        }
    }

    pub fn need_new_angle(duplicate_ratio: f32) -> Self {
        Self {
            suggestion_type: SuggestionType::NeedNewAngle,
            description: format!(
                "重复信息较多 ({:.1}%), 建议换个角度搜索",
                duplicate_ratio * 100.0
            ),
            suggested_keywords: vec![
                "最新".to_string(),
                "发展趋势".to_string(),
                "对比分析".to_string(),
            ],
            priority: 4,
        }
    }

    pub fn need_validation(confidence: f32) -> Self {
        Self {
            suggestion_type: SuggestionType::NeedValidation,
            description: format!(
                "置信度较低 ({:.0}%), 建议验证信息准确性",
                confidence * 100.0
            ),
            suggested_keywords: vec![
                "官方".to_string(),
                "权威来源".to_string(),
                "数据来源".to_string(),
            ],
            priority: 4,
        }
    }
}

/// 评分配置
///
/// 用于配置评分系统的各项阈值和参数。
///
/// # 示例
///
/// ```rust
/// use rucora_core::research::ScoringConfig;
///
/// // 创建自定义配置
/// let config = ScoringConfig {
///     quality_threshold: 0.8,      // 提高质量要求
///     confidence_threshold: 0.9,   // 提高置信度要求
///     duplicate_threshold: 0.2,    // 更严格控制重复
///     min_info_count: 10,          // 要求更多信息
///     min_source_diversity: 3,    // 要求更多来源类型
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// 质量阈值，低于此值会触发改进建议
    ///
    /// 默认值: 0.6
    pub quality_threshold: f32,
    /// 置信度阈值，低于此值会建议继续搜索
    ///
    /// 默认值: 0.7
    pub confidence_threshold: f32,
    /// 重复信息阈值，超过此比例会建议换角度
    ///
    /// 默认值: 0.3 (30%)
    pub duplicate_threshold: f32,
    /// 最少信息数量
    ///
    /// 默认值: 5
    pub min_info_count: usize,
    /// 最少来源多样性
    ///
    /// 默认值: 2 (至少2种不同来源类型)
    pub min_source_diversity: usize,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            quality_threshold: 0.6,
            confidence_threshold: 0.7,
            duplicate_threshold: 0.3,
            min_info_count: 5,
            min_source_diversity: 2,
        }
    }
}
