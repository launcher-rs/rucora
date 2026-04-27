//! 深度研究 Agent 实现
//!
//! 多阶段研究流程：
//! 1. 规划阶段：制定搜索策略，分解子问题
//! 2. 搜索阶段：并发搜索多个子问题
//! 3. 精读阶段：对关键 URL 进行深度阅读
//! 4. 综合阶段：汇总所有信息
//! 5. 报告阶段：生成结构化研究报告

use agentkit::agent::execution::DefaultExecution;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision, AgentInput};
use agentkit_core::error::AgentError;
use async_trait::async_trait;
use tracing::{info, warn};

// ============================================================
// 研究阶段定义
// ============================================================

/// 研究流程阶段
#[derive(Debug, Clone, PartialEq)]
pub enum ResearchPhase {
    /// 阶段 1：规划 + 搜索（让 LLM 自主调用搜索工具）
    SearchAndGather,
    /// 阶段 2：深度精读（让 LLM 对重要 URL 进行精读）
    DeepRead,
    /// 阶段 3：综合 + 生成完整报告
    Synthesize,
}

/// 阶段内部的 Agent 决策实现
struct PhaseAgent {
    name: String,
}

#[async_trait]
impl Agent for PhaseAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        // 使用 default_chat_request，DefaultExecution 会自动注入工具定义
        AgentDecision::Chat {
            request: Box::new(context.default_chat_request()),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================
// 多阶段研究引擎
// ============================================================

/// 多阶段深度研究引擎
///
/// 将深度研究分为三个独立阶段，每个阶段有专用的系统提示词和工具集，
/// 解决单次 run 无法生成完整报告的根本问题。
pub struct DeepResearchEngine {
    /// 搜索 + 收集阶段的执行器
    search_execution: DefaultExecution,
    /// 深度精读阶段的执行器
    read_execution: DefaultExecution,
    /// 综合报告阶段的执行器（不带工具，纯文本综合）
    synthesize_execution: DefaultExecution,
}

impl DeepResearchEngine {
    pub fn new(
        search_execution: DefaultExecution,
        read_execution: DefaultExecution,
        synthesize_execution: DefaultExecution,
    ) -> Self {
        Self {
            search_execution,
            read_execution,
            synthesize_execution,
        }
    }

    /// 执行完整的深度研究流程
    ///
    /// 返回 (最终报告文本, 每阶段输出摘要)
    pub async fn run(&self, topic: &str) -> Result<(String, Vec<PhaseResult>), AgentError> {
        let mut phase_results = Vec::new();

        // ── 阶段 1：搜索收集 ──────────────────────────────────────
        info!("🔍 阶段 1：搜索收集");
        let search_prompt = build_search_prompt(topic);
        let search_agent = PhaseAgent {
            name: "search_agent".to_string(),
        };

        let search_output = self
            .search_execution
            .run(&search_agent, AgentInput::new(search_prompt))
            .await?;

        let search_text = search_output.text().unwrap_or_default().to_string();
        let search_tokens = search_output.total_tokens();
        info!(
            "✅ 阶段 1 完成，收集到 {} 字符，消耗 {} tokens",
            search_text.len(),
            search_tokens
        );
        phase_results.push(PhaseResult {
            phase: ResearchPhase::SearchAndGather,
            content: search_text.clone(),
            tokens: search_tokens,
        });

        if search_text.trim().is_empty() {
            warn!("⚠️ 阶段 1 无输出，跳过后续阶段");
            return Err(AgentError::Message("搜索阶段无输出".to_string()));
        }

        // ── 阶段 2：深度精读 ──────────────────────────────────────
        info!("📖 阶段 2：深度精读");
        let read_prompt = build_deep_read_prompt(topic, &search_text);
        let read_agent = PhaseAgent {
            name: "read_agent".to_string(),
        };

        let read_output = self
            .read_execution
            .run(&read_agent, AgentInput::new(read_prompt))
            .await?;

        let read_text = read_output.text().unwrap_or_default().to_string();
        let read_tokens = read_output.total_tokens();
        info!(
            "✅ 阶段 2 完成，精读内容 {} 字符，消耗 {} tokens",
            read_text.len(),
            read_tokens
        );
        phase_results.push(PhaseResult {
            phase: ResearchPhase::DeepRead,
            content: read_text.clone(),
            tokens: read_tokens,
        });

        // ── 阶段 3：综合报告 ──────────────────────────────────────
        info!("📝 阶段 3：综合报告");
        let combined = format_combined_context(topic, &search_text, &read_text);
        let synthesize_prompt = build_synthesis_prompt(topic, &combined);
        let synthesize_agent = PhaseAgent {
            name: "synthesize_agent".to_string(),
        };

        let report_output = self
            .synthesize_execution
            .run(&synthesize_agent, AgentInput::new(synthesize_prompt))
            .await?;

        let report_text = report_output.text().unwrap_or_default().to_string();
        let report_tokens = report_output.total_tokens();
        info!(
            "✅ 阶段 3 完成，报告 {} 字符，消耗 {} tokens",
            report_text.len(),
            report_tokens
        );
        phase_results.push(PhaseResult {
            phase: ResearchPhase::Synthesize,
            content: report_text.clone(),
            tokens: report_tokens,
        });

        Ok((report_text, phase_results))
    }
}

/// 单阶段产出结果
pub struct PhaseResult {
    pub phase: ResearchPhase,
    pub content: String,
    pub tokens: u32,
}

// ============================================================
// 各阶段提示词构建
// ============================================================

/// 阶段 1：搜索收集提示词
///
/// 强制 LLM 先规划再搜索，并将搜索结果整理为结构化摘要
fn build_search_prompt(topic: &str) -> String {
    format!(
        r#"你是一名资深研究员，正在对以下主题进行深度调研：

**研究主题**: {topic}

## 任务要求

请严格按照以下步骤执行，**不得跳过任何步骤**：

### 步骤 1：获取当前时间
使用 datetime 工具获取当前日期，用于确认信息时效性。

### 步骤 2：制定搜索策略
将主题分解为 5-8 个搜索子问题，例如：
- 背景与历史
- 当前最新动态
- 各方观点与立场
- 数据与统计
- 未来趋势与影响

### 步骤 3：执行搜索
对每个子问题执行 **至少 1 次** 搜索（使用 tavily_search 或 web_fetch）。
搜索时注意：
- 使用多样化的关键词
- 搜索英文和中文信息
- 关注最新的新闻和分析

### 步骤 4：整理搜索结果
将所有搜索到的信息整理为结构化摘要，包含：
- 每个子问题的核心发现
- 关键数据和引用来源（URL）
- 重要的引用段落

**输出格式要求**：
输出一份完整的搜索摘要，每个发现都要注明来源 URL，方便后续精读。
格式如下：

## 搜索结果摘要

### [子问题 1 标题]
- **来源**: [URL]
- **核心发现**: [具体内容]

### [子问题 2 标题]
...

## 待精读 URL 列表
1. [URL 1] - [理由]
2. [URL 2] - [理由]
..."#
    )
}

/// 阶段 2：深度精读提示词
///
/// 基于阶段 1 的 URL 列表，深度阅读 3-5 个最重要的页面
fn build_deep_read_prompt(topic: &str, search_summary: &str) -> String {
    // 截取前 6000 字符，避免 prompt 过长
    let summary_preview = if search_summary.len() > 6000 {
        format!("{}...[已截断]", &search_summary[..6000])
    } else {
        search_summary.to_string()
    };

    format!(
        r#"你是一名资深研究员，正在对以下主题进行深度调研：**{topic}**

## 阶段 1 搜索摘要（已完成）

{summary_preview}

---

## 阶段 2 任务：深度精读

请从上方「待精读 URL 列表」中选择 **3-5 个最重要的 URL**，使用 `web_fetch` 或 `browse` 工具逐一精读。

精读标准：
- 优先选择权威来源（政府网站、主流媒体、学术机构）
- 选择信息量最丰富的页面
- 选择时效性最强的内容

对每个精读的页面，提取：
1. **核心论点**：该来源的主要观点是什么？
2. **关键数据**：有哪些具体数字、统计、事件日期？
3. **独特视角**：该来源提供了哪些其他来源没有的独特信息？
4. **引用段落**：直接引用最重要的 1-3 段文字（保留原文，标注来源 URL）

## 输出格式

### 精读报告 1：[页面标题]
- **URL**: [完整 URL]
- **来源类型**: [官方/媒体/学术/博客]
- **核心论点**: ...
- **关键数据**: ...
- **独特视角**: ...
- **关键引用**:
  > "..." — 来源：[URL]

### 精读报告 2：...
"#
    )
}

/// 将两阶段内容格式化为综合上下文
fn format_combined_context(topic: &str, search: &str, deep_read: &str) -> String {
    // 控制总长度，避免超过 LLM 上下文窗口
    let max_per_section = 8000usize;

    let search_trimmed = if search.len() > max_per_section {
        format!("{}...[内容过长已截断]", &search[..max_per_section])
    } else {
        search.to_string()
    };

    let read_trimmed = if deep_read.len() > max_per_section {
        format!("{}...[内容过长已截断]", &deep_read[..max_per_section])
    } else {
        deep_read.to_string()
    };

    format!(
        "## 研究主题\n{topic}\n\n## 阶段 1：搜索摘要\n\n{search_trimmed}\n\n## 阶段 2：深度精读\n\n{read_trimmed}"
    )
}

/// 阶段 3：综合报告提示词
///
/// 基于前两阶段的所有信息，生成完整的研究报告
fn build_synthesis_prompt(topic: &str, combined_context: &str) -> String {
    format!(
        r#"你是一名专业研究报告撰写专家。请基于以下研究材料，为主题「{topic}」撰写一份完整、深度的研究报告。

## 研究材料

{combined_context}

---

## 报告撰写要求

### 质量标准
1. **完整性**：必须覆盖主题的所有重要方面
2. **深度**：不能停留在表面，需要有深层分析
3. **客观性**：呈现多方观点，不偏向任何一方
4. **时效性**：基于最新信息，注明信息时间节点
5. **可读性**：结构清晰，语言专业但易懂

### 报告结构（必须完整包含以下所有章节）

```
# [报告标题]

## 执行摘要
（200-300字的核心发现总结）

## 一、背景与概述
（历史背景、基本情况介绍）

## 二、当前状况分析
（最新动态、关键事件时间线）

## 三、核心议题深度分析
（分 3-5 个子章节，每个子章节深度分析一个核心议题）

## 四、各方立场与观点
（不同利益相关方的立场分析）

## 五、数据与证据
（关键统计数据、图表说明、事实核查）

## 六、影响与意义
（短期影响、长期影响、潜在风险）

## 七、未来趋势与展望
（基于现有信息的趋势判断）

## 八、结论与建议
（综合结论，可行性建议）

## 参考来源
（列出所有引用的 URL 和来源名称）
```

### 格式要求
- 使用标准 Markdown 格式
- 重要数据用 **粗体** 标注
- 引用段落使用 > 引用格式，并注明来源
- 每个章节至少 200 字
- 总字数不少于 3000 字

请现在开始撰写完整报告，不要省略任何章节，不要用"（此处省略）"等替代实际内容。"#
    )
}
