# rucora Deep Research 0.2 实施计划

## 概述

本文档描述 rucora 深度研究功能的 0.2 版本实现计划，参考 `temp/local-deep-research` Python 项目的核心功能，为 rucora 提供更强大、更灵活的深度研究能力。

## 参考项目分析

### temp/local-deep-research 核心功能

| 模块 | 功能描述 | 迁移优先级 |
|------|----------|------------|
| search_system | 多种搜索策略（20+种） | 高 |
| report_generator | 报告生成（Markdown/PDF） | 高 |
| research_library | 本地知识库存储 | 中 |
| web_search_engines | 多搜索引擎支持 | 高 |
| citation_handlers | 引用处理 | 中 |
| followup_research | 后续追问研究 | 中 |
| document_loaders | 文档加载（PDF/网页） | 中 |
| domain_classifier | 领域分类 | 低 |

### 现有 rucora-deep-research 能力

- ✅ 多阶段研究流程（搜索→精读→综合）
- ✅ Tavily/DuckDuckGo 搜索
- ✅ 网页内容抓取
- ✅ Markdown 报告生成
- ✅ 工具重试/超时/缓存
- ✅ quick_research 示例（快速模式）
- ✅ iterative_research 示例（迭代模式）

## 0.2 版本目标

### 核心目标

1. **多种研究策略实现** - 支持不同深度和速度的研究模式
2. **研究库系统** - 本地存储和检索研究结果
3. **增强的搜索能力** - 多引擎支持和搜索策略选择
4. **引用管理** - 完善引用和来源追踪

### 高级目标

5. **文档分析** - 支持本地文档研究
6. **后续研究** - 支持深度追问和迭代研究
7. **导出功能** - 多格式报告导出

## 实施方案

### 1. 多种研究策略示例

创建多个独立的 example，提供不同研究策略的选择：

```
examples/
├── rucora-deep-research/          # 现有 0.1 版本（多阶段基础版）
├── rucora-deep-research-fast/     # 快速研究模式（30秒-3分钟）
├── rucora-deep-research-agentic/  # Agentic 自主研究模式
├── rucora-deep-research-library/ # 研究库模式（知识积累）
└── rucora-deep-research-academic/ # 学术研究模式
```

### 2. 策略详细设计

#### 2.1 快速研究模式 (fast)

- **目标**: 30秒-3分钟内获取带引用的答案
- **策略**: 单次搜索 + 快速综合
- **适用场景**: 简单事实查询、快速了解

```rust
// 提示词设计
- 限制搜索轮次（最多 3 轮）
- 快速提取核心信息
- 简洁的引用格式
- 最大输出 1000 字
```

#### 2.2 Agentic 自主研究模式 (agentic)

- **目标**: 完全自主决策的研究流程
- **策略**: LangGraph 风格的自主 Agent
- **适用场景**: 复杂问题、深度分析

核心特点：
- LLM 自主决定搜索策略
- 自适应切换搜索引擎
- 基于发现动态调整研究路径
- 迭代式发现和验证

```rust
// Agentic 循环
loop {
    // 1. 分析当前状态
    // 2. 决定下一步行动（搜索/精读/综合）
    // 3. 执行行动
    // 4. 评估结果
    // 5. 决定是否继续或结束
}
```

#### 2.3 研究库模式 (library)

- **目标**: 持续积累知识，建立可检索的知识库
- **策略**: 研究 + 存储 + 检索
- **适用场景**: 长期研究主题、系列研究

功能：
- 研究结果持久化存储
- 向量相似性检索
- 历史研究查询
- 多轮研究关联

#### 2.4 学术研究模式 (academic)

- **目标**: 学术级研究报告
- **策略**: 多来源深度研究
- **适用场景**: 论文准备、深度分析

特点：
- arXiv/PubMed/Semantic Scholar 搜索
- 学术引用格式
- 同行评议来源优先
- 详细的引用列表

### 3. 核心模块设计

#### 3.1 搜索策略抽象

```rust
/// 搜索策略 trait
pub trait SearchStrategy: Send + Sync {
    /// 执行搜索
    async fn search(&self, context: &ResearchContext) -> Result<SearchResult, Error>;
    
    /// 判断是否继续搜索
    fn should_continue(&self, result: &SearchResult) -> bool;
    
    /// 策略名称
    fn name(&self) -> &'static str;
}
```

#### 3.2 研究上下文

```rust
/// 研究上下文
pub struct ResearchContext {
    pub topic: String,
    pub current_phase: ResearchPhase,
    pub collected_info: Vec<InfoPiece>,
    pub visited_urls: Vec<String>,
    pub search_history: Vec<SearchQuery>,
    pub citations: Vec<Citation>,
}
```

#### 3.3 研究引擎 trait

```rust
/// 深度研究引擎 trait
pub trait DeepResearchEngine: Send + Sync {
    /// 执行研究
    async fn research(&self, topic: &str) -> Result<ResearchReport, Error>;
    
    /// 获取研究进度
    fn progress(&self) -> ResearchProgress;
}
```

### 4. 技术实现细节

#### 4.1 工具集扩展

新增工具：

| 工具 | 功能 | 实现优先级 |
|------|------|------------|
| ArxivTool | 学术论文搜索 | 高 |
| PubMedTool | 医学文献搜索 | 中 |
| SemanticScholarTool | 学术搜索 | 中 |
| DuckDuckGoTool | 免费搜索 | 高 |
| SearXNGTool | 自托管搜索 | 中 |
| SerperTool | Google 搜索 API | 中 |

#### 4.2 引用处理

```rust
/// 引用条目
pub struct Citation {
    pub url: String,
    pub title: String,
    pub accessed_at: DateTime<Utc>,
    pub snippet: String,
    pub relevance_score: f32,
}

/// 引用处理器
pub trait CitationHandler: Send + Sync {
    fn extract_citations(&self, content: &str) -> Vec<Citation>;
    fn format_citation(&self, citation: &Citation) -> String;
    fn format_reference_list(&self, citations: &[Citation]) -> String;
}
```

#### 4.3 研究库存储

```rust
/// 研究库 trait
pub trait ResearchLibrary: Send + Sync {
    /// 保存研究结果
    async fn save_research(&self, report: &ResearchReport) -> Result<String, Error>;
    
    /// 搜索历史研究
    async fn search(&self, query: &str) -> Result<Vec<ResearchReport>, Error>;
    
    /// 获取特定研究
    async fn get(&self, id: &str) -> Result<Option<ResearchReport>, Error>;
}
```

### 5. 实现路线图

#### Phase 1: 基础架构（已完成部分）

- [x] quick_research 示例（快速模式）
- [x] iterative_research 示例（迭代模式）
- [ ] 定义核心 trait（SearchStrategy, ResearchEngine, ResearchLibrary）
- [ ] 完善现有报告生成

#### Phase 2: 高级搜索（2-3周）

- [ ] 实现 agentic 自主研究模式
- [ ] 添加多引擎支持（Serper, Arxiv 等）
- [ ] 完善引用处理

#### Phase 3: 研究库（2-3周）

- [ ] 实现本地存储（SQLite/文件）
- [ ] 实现向量检索（可选）
- [ ] 实现历史查询
- [ ] 创建 library 示例

#### Phase 4: 学术研究（1-2周）

- [ ] 添加学术搜索工具（Arxiv, PubMed, Semantic Scholar）
- [ ] 完善学术引用格式
- [ ] 创建 academic 示例

### 6. 示例文件规划

| 示例 | 文件 | 状态 | 描述 |
|------|------|------|------|
| 现有 0.1 | `rucora-deep-research/` | ✅ 完成 | 多阶段基础版 |
| 快速模式 | `quick_research/` | ✅ 完成 | 快速问答模式 |
| 迭代模式 | `iterative_research/` | ✅ 完成 | 迭代深化研究 |
| 自主模式 | `rucora-deep-research-agentic/` | 📋 计划 | 自主决策研究 |
| 研究库模式 | `rucora-deep-research-library/` | 📋 计划 | 知识积累模式 |
| 学术模式 | `rucora-deep-research-academic/` | 📋 计划 | 学术论文模式 |

> 注意：现有示例已实现基础功能，可直接使用。

每个完整示例包含：
- `Cargo.toml` - 依赖配置
- `src/main.rs` - 入口
- `src/engine.rs` - 研究引擎
- `README.md` - 使用说明

## 配置设计

### 策略配置

```rust
#[derive(Debug, Clone)]
pub struct ResearchConfig {
    /// 研究策略
    pub strategy: ResearchStrategy,
    
    /// 最大迭代次数
    pub max_iterations: u32,
    
    /// 最大并发搜索数
    pub max_concurrent_searches: u32,
    
    /// 搜索引擎配置
    pub search_engines: Vec<SearchEngineConfig>,
    
    /// 报告配置
    pub report_config: ReportConfig,
}
```

### 策略枚举

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
```

## 风险与挑战

1. **Agentic 模式复杂度** - 自主决策循环需要精心设计避免无限循环
2. **引用准确性** - 需要可靠的来源验证机制
3. **存储一致性** - 研究库需要考虑并发和数据一致性
4. **性能平衡** - 不同策略需要在深度和速度之间取得平衡

## 验收标准

### 功能验收

- [x] 至少 3 种可运行的研究模式（已完成：quick, iterative, standard）
- [x] 每种模式可独立运行并产出报告
- [ ] 报告包含正确的引用信息
- [x] 支持多提供商配置

### 性能验收

- [x] Fast 模式 30秒-3分钟内完成（quick_research）
- [x] 标准模式 5-15 分钟完成（rucora-deep-research）
- [x] 支持并发搜索加速

### 代码质量

- [x] 遵循 rucora 代码规范
- [x] 包含中文注释（根据 chinese-comments.mdc 规范）
- [ ] 通过 clippy 检查