# rucora Deep Research 0.2 实现思路

## 架构设计

### 核心组件关系

```
┌─────────────────────────────────────────────────────────────┐
│                     DeepResearchEngine                       │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐   │
│  │   Search     │  │   Read       │  │   Synthesize   │   │
│  │   Strategy   │  │   Strategy   │  │   Strategy     │   │
│  └──────────────┘  └──────────────┘  └─────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│                      ToolRegistry                            │
├──────────┬──────────┬──────────┬──────────┬────────────────┤
│  Tavily  │  Serper  │ Arxiv   │ PubMed  │   WebFetch    │
└──────────┴──────────┴──────────┴──────────┴────────────────┘
```

### Trait 设计

#### SearchStrategy Trait

```rust
use async_trait::async_trait;
use rucora_core::agent::AgentInput;

#[async_trait]
pub trait SearchStrategy: Send + Sync {
    /// 策略名称
    fn name(&self) -> &'static str;
    
    /// 策略描述
    fn description(&self) -> &'static str;
    
    /// 执行搜索
    async fn search(
        &self,
        provider: &Arc<dyn LlmProvider>,
        tools: &ToolRegistry,
        topic: &str,
        context: &ResearchContext,
    ) -> Result<SearchResult, ResearchError>;
    
    /// 判断是否需要继续搜索
    fn should_continue(&self, result: &SearchResult) -> bool;
    
    /// 获取最大迭代次数
    fn max_iterations(&self) -> Option<u32>;
}
```

#### ResearchContext

```rust
/// 研究上下文 - 贯穿整个研究流程
pub struct ResearchContext {
    /// 研究主题
    pub topic: String,
    
    /// 当前阶段
    pub phase: ResearchPhase,
    
    /// 已收集的信息片段
    pub collected_info: Vec<InfoPiece>,
    
    /// 已访问的 URL
    pub visited_urls: Vec<String>,
    
    /// 搜索历史
    pub search_history: Vec<SearchHistory>,
    
    /// 引用列表
    pub citations: Vec<Citation>,
    
    /// 内部状态（用于策略特定数据）
    state: HashMap<String, serde_json::Value>,
}

impl ResearchContext {
    pub fn new(topic: &str) -> Self;
    pub fn add_info(&mut self, info: InfoPiece);
    pub fn add_citation(&mut self, citation: Citation);
    pub fn has_visited(&self, url: &str) -> bool;
}
```

#### ResearchPhase

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
```

## 策略实现

### 1. StandardStrategy（标准多阶段）

基于现有的 0.1 版本实现：

```rust
pub struct StandardStrategy {
    max_search_rounds: u32,
    max_deep_read: u32,
    include_citations: bool,
}

impl StandardStrategy {
    pub fn new() -> Self {
        Self {
            max_search_rounds: 3,
            max_deep_read: 5,
            include_citations: true,
        }
    }
}

#[async_trait]
impl SearchStrategy for StandardStrategy {
    // 实现标准三阶段流程：
    // 1. 搜索收集（多轮）
    // 2. 深度精读
    // 3. 综合报告
}
```

### 2. FastStrategy（快速模式）

```rust
pub struct FastStrategy {
    max_searches: u32,
    quick_timeout: Duration,
    max_output_length: usize,
}

impl FastStrategy {
    pub fn new() -> Self {
        Self {
            max_searches: 3,
            quick_timeout: Duration::from_secs(60),
            max_output_length: 1000,
        }
    }
}

#[async_trait]
impl SearchStrategy for FastStrategy {
    // 快速流程：
    // 1. 单次搜索获取基本信息
    // 2. 快速综合输出
    // 3. 限制输出长度
}
```

### 3. AgenticStrategy（自主模式）

核心是实现一个自主循环：

```rust
pub struct AgenticStrategy {
    max_iterations: u32,
    confidence_threshold: f32,
    use_chain_of_thought: bool,
}

#[async_trait]
impl SearchStrategy for AgenticStrategy {
    async fn search(&self, provider, tools, topic, context) -> Result<SearchResult> {
        let mut iteration = 0;
        let mut current_result = SearchResult::default();
        
        while self.should_continue(&current_result) && iteration < self.max_iterations {
            iteration += 1;
            
            // 1. 让 LLM 分析当前状态，决定下一步
            let decision = self.decide_next_action(provider, topic, context).await?;
            
            // 2. 执行决定的动作
            match decision.action {
                Action::Search(query) => {
                    let results = self.execute_search(tools, query).await?;
                    current_result.add_search_results(results);
                }
                Action::Read(url) => {
                    let content = self.read_content(tools, url).await?;
                    current_result.add_content(content);
                }
                Action::Synthesize => {
                    // 综合当前所有信息
                    current_result.is_complete = true;
                    break;
                }
            }
            
            // 3. 评估当前状态
            current_result.confidence = self.evaluate_confidence(provider, topic, context).await?;
        }
        
        Ok(current_result)
    }
    
    fn should_continue(&self, result: &SearchResult) -> bool {
        !result.is_complete && result.confidence < self.confidence_threshold
    }
}

/// Agent 决策
struct AgentDecision {
    action: Action,
    reasoning: String,
    confidence: f32,
}

enum Action {
    Search(String),
    Read(String),
    Synthesize,
    Done,
}
```

### 4. LibraryStrategy（研究库模式）

```rust
pub struct LibraryStrategy {
    storage_path: PathBuf,
    enable_embedding: bool,
    max_similar_results: usize,
}

struct LibraryResearchEngine {
    library: Arc<ResearchLibrary>,
    strategy: Box<dyn SearchStrategy>,
}

impl LibraryStrategy {
    // 关键方法：
    // 1. 先查询本地库是否有相关研究
    // 2. 如有，合并历史研究结果
    // 3. 执行新的搜索
    // 4. 保存新结果到库中
    
    async fn search(&self, provider, tools, topic, context) -> Result<SearchResult> {
        // 检查本地研究库
        let similar = self.library.search(topic).await?;
        
        let mut result = SearchResult::default();
        
        // 如果有相似研究，加入上下文
        if !similar.is_empty() {
            result.add_context("已有研究", &similar);
        }
        
        // 执行新研究
        // ... 搜索逻辑 ...
        
        // 保存结果
        self.library.save(result).await?;
        
        Ok(result)
    }
}
```

### 5. AcademicStrategy（学术模式）

```rust
pub struct AcademicStrategy {
    // 学术专用工具
    preferred_engines: Vec<SearchEngine>,
    min_citation_count: u32,
    require_peer_review: bool,
}

impl AcademicStrategy {
    // 1. 优先使用学术搜索引擎
    // 2. 要求更多引用来源
    // 3. 检查来源权威性
    // 4. 生成学术格式引用
}
```

## 工具集成

### 现有工具

```rust
use rucora_tools::{
    TavilyTool,      // 已有
    BrowseTool,      // 已有
    WebFetchTool,    // 已有
    DatetimeTool,    // 已有
};
```

### 新增工具（需要实现）

```rust
// 学术搜索工具
pub struct ArxivTool;
pub struct PubMedTool;
pub struct SemanticScholarTool;

// 更多搜索工具
pub struct SerperTool;      // Google 搜索 API
pub struct SearXNGTool;     // 自托管搜索
pub struct DuckDuckGoTool;  // 免费搜索
```

## 报告生成

### 报告结构

```rust
pub struct ResearchReport {
    pub title: String,
    pub topic: String,
    pub created_at: DateTime<Utc>,
    pub phases: Vec<PhaseReport>,
    pub citations: Vec<Citation>,
    pub summary: String,
    pub full_content: String,
    pub metadata: ReportMetadata,
}

pub struct PhaseReport {
    pub phase: ResearchPhase,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub content: String,
    pub tokens_used: u32,
    pub tool_calls: Vec<ToolCallRecord>,
}
```

### 报告格式

支持多种输出格式：

```rust
pub enum ReportFormat {
    Markdown,
    Html,
    Pdf,  // 需要额外库
    Json,
    Text,
}

impl ResearchReport {
    pub fn to_format(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Markdown => self.to_markdown(),
            ReportFormat::Html => self.to_html(),
            ReportFormat::Json => self.to_json(),
            ReportFormat::Text => self.to_text(),
            ReportFormat::Pdf => unimplemented!(),
        }
    }
}
```

## 引用处理

### 引用格式

```rust
pub struct Citation {
    pub url: String,
    pub title: String,
    pub source_type: SourceType,
    pub accessed_at: DateTime<Utc>,
    pub snippet: String,
    pub relevance_score: f32,
}

pub enum SourceType {
    News,
    Academic,
    Blog,
    Official,
    SocialMedia,
    Unknown,
}

impl Citation {
    // 生成不同格式的引用
    pub fn format_apa(&self) -> String;
    pub fn format_mla(&self) -> String;
    pub fn format_chicago(&self) -> String;
}
```

## 存储设计

### 本地存储

```rust
// 文件系统存储
pub struct FileResearchLibrary {
    base_path: PathBuf,
}

// SQLite 存储（可选）
pub struct SqliteResearchLibrary {
    pool: SqlitePool,
}
```

### 数据模型

```sql
CREATE TABLE research_reports (
    id TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    title TEXT,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP,
    summary TEXT,
    full_content TEXT,
    format TEXT DEFAULT 'markdown',
    metadata JSON
);

CREATE TABLE citations (
    id TEXT PRIMARY KEY,
    report_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    source_type TEXT,
    accessed_at TIMESTAMP,
    snippet TEXT,
    relevance_score REAL,
    FOREIGN KEY (report_id) REFERENCES research_reports(id)
);

CREATE INDEX idx_topic ON research_reports(topic);
CREATE INDEX idx_created ON research_reports(created_at);
```

## 配置管理

### 环境变量

```bash
# 研究配置
RUCORA_RESEARCH_STRATEGY=agentic
RUCORA_RESEARCH_MAX_ITERATIONS=10
RUCORA_RESEARCH_TIMEOUT=600

# 搜索引擎
RUCORA_SEARCH_ENGINES=tavily,serper,arxiv

# 存储
RUCORA_LIBRARY_PATH=/path/to/library
RUCORA_LIBRARY_ENABLE_EMBEDDING=true
```

### 配置文件

```toml
[research]
strategy = "standard"
max_iterations = 10
max_concurrent = 3

[research.engines]
enabled = ["tavily", "serper", "arxiv"]

[library]
enabled = true
path = "~/.rucora/library"
```

## 示例代码结构

### rucora-deep-research-fast 示例

```
examples/rucora-deep-research-fast/
├── Cargo.toml
└── src/
    ├── main.rs          # 入口
    ├── fast_engine.rs   # 快速研究引擎
    └── config.rs        # 配置
```

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let config = FastConfig::load()?;
    let provider = create_provider(&config)?;
    
    let engine = FastResearchEngine::new(provider, config);
    let report = engine.research("主题").await?;
    
    println!("{}", report.to_markdown());
    Ok(())
}
```

## 测试计划

### 单元测试

- 各策略的独立测试
- 工具调用测试
- 报告生成测试

### 集成测试

- 端到端研究流程测试
- 多提供商测试
- 存储功能测试

### 性能测试

- Fast 模式响应时间
- 并发搜索性能
- 大规模研究测试

## 下一步行动

1. 完善 `research_agent.rs` 中的 trait 定义
2. 实现新的搜索工具（Serper, Arxiv 等）
3. 创建新示例目录结构
4. 实现 FastStrategy 作为第一个新策略
5. 逐步实现其他策略