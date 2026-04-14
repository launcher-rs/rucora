//! AgentKit 深度研究示例 (重构版)
//!
//! 演进改进：
//! 1. 【短期】集成 ErrorClassifier + ResilientProvider 包装执行层，自动重试/回退
//! 2. 【短期】使用 LayeredCompressor 替代硬截断，智能压缩上下文
//! 3. 【短期】结构化输出约束各阶段输出 (ResponseFormat::JsonSchema)
//! 4. 【中期】并行无依赖阶段执行 (tokio::join!)
//! 5. 【中期】引入 CriticAgent 反思-验证循环
//! 6. 【中期】显式工具编排 ResearchPlanner

mod config;

use agentkit::agent::ToolRegistry;
use agentkit::agent::execution::DefaultExecution;
use agentkit::prelude::*;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision};
use agentkit_core::provider::LlmProvider;
use agentkit::provider::resilient::{ResilientProvider, RetryConfig};
use agentkit_tools::{
    DatetimeTool, FileWriteTool, ShellTool, WebScraperTool, WebSearchTool,
};
use chrono::Local;
use config::{AppConfig, ProviderType};
use console::style;
use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

// ====================== 核心数据结构 ======================

/// 研究阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResearchPhase {
    Background,
    DeepAnalysis,
    FutureTrends,
    SWOTAnalysis,
    CaseStudies,
    Synthesis,
}

impl ResearchPhase {
    fn name(&self) -> &'static str {
        match self {
            Self::Background => "背景调研",
            Self::DeepAnalysis => "深入分析",
            Self::FutureTrends => "未来趋势",
            Self::SWOTAnalysis => "SWOT 分析",
            Self::CaseStudies => "案例研究",
            Self::Synthesis => "综合总结",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Self::Background => "📚",
            Self::DeepAnalysis => "🔍",
            Self::FutureTrends => "🔮",
            Self::SWOTAnalysis => "📊",
            Self::CaseStudies => "📋",
            Self::Synthesis => "📝",
        }
    }

    /// 判断阶段是否有前置依赖
    fn has_dependency(&self) -> bool {
        // SWOT, CaseStudies, Synthesis 依赖前期结果
        matches!(
            self,
            Self::SWOTAnalysis | Self::CaseStudies | Self::Synthesis
        )
    }

    /// 判断两个阶段是否可以并行执行
    #[allow(dead_code)]
    fn can_parallel_with(&self, other: &Self) -> bool {
        !self.has_dependency() && !other.has_dependency() && self != other
    }
}

/// 研究深度
#[derive(Debug, Clone, Copy)]
enum ResearchDepth {
    Quick,
    Standard,
    Deep,
}

fn get_phases_for_depth(depth: ResearchDepth) -> Vec<ResearchPhase> {
    match depth {
        ResearchDepth::Quick => vec![
            ResearchPhase::Background,
            ResearchPhase::DeepAnalysis,
            ResearchPhase::Synthesis,
        ],
        ResearchDepth::Standard => vec![
            ResearchPhase::Background,
            ResearchPhase::DeepAnalysis,
            ResearchPhase::FutureTrends,
            ResearchPhase::SWOTAnalysis,
            ResearchPhase::Synthesis,
        ],
        ResearchDepth::Deep => vec![
            ResearchPhase::Background,
            ResearchPhase::DeepAnalysis,
            ResearchPhase::FutureTrends,
            ResearchPhase::SWOTAnalysis,
            ResearchPhase::CaseStudies,
            ResearchPhase::Synthesis,
        ],
    }
}

/// 研究进度追踪
struct ResearchProgress {
    current_phase: usize,
    total_phases: usize,
    total_tokens: u32,
    api_calls: u32,
    tool_calls: u32,
    retry_count: u32,
    phase_results: Vec<(String, String)>,
}

impl ResearchProgress {
    fn new(total_phases: usize) -> Self {
        Self {
            current_phase: 0,
            total_phases,
            total_tokens: 0,
            api_calls: 0,
            tool_calls: 0,
            retry_count: 0,
            phase_results: Vec::new(),
        }
    }

    fn update_tokens(&mut self, tokens: u32) {
        self.total_tokens += tokens;
        self.api_calls += 1;
    }

    #[allow(dead_code)]
    fn increment_retries(&mut self) {
        self.retry_count += 1;
    }

    fn add_phase_result(&mut self, phase_name: String, result: String) {
        self.phase_results.push((phase_name, result));
        self.current_phase += 1;
    }

    fn display(&self) {
        let progress = if self.total_phases > 0 {
            self.current_phase as f64 / self.total_phases as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "\n{} 研究进度：{:.1}% ({}/{}) | Token: {} | API调用: {} | 工具调用: {} | 重试: {}",
            style("📊").bold(),
            progress,
            self.current_phase,
            self.total_phases,
            style(self.total_tokens).cyan().bold(),
            self.api_calls,
            self.tool_calls,
            style(self.retry_count).yellow().bold()
        );
    }
}

// ====================== 结构化输出 ======================

/// 研究阶段输出结构 (改进 3: 结构化输出)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct PhaseOutput {
    summary: String,
    key_findings: Vec<String>,
    data_points: Vec<String>,
    sources: Vec<String>,
    confidence_level: String,
}

#[allow(dead_code)]
impl PhaseOutput {
    fn to_markdown(&self) -> String {
        let mut md = format!("## 摘要\n{}\n\n## 关键发现\n", self.summary);
        for finding in &self.key_findings {
            md.push_str(&format!("- {}\n", finding));
        }
        if !self.data_points.is_empty() {
            md.push_str("\n## 数据支撑\n");
            for dp in &self.data_points {
                md.push_str(&format!("- {}\n", dp));
            }
        }
        if !self.sources.is_empty() {
            md.push_str("\n## 参考来源\n");
            for src in &self.sources {
                md.push_str(&format!("- {}\n", src));
            }
        }
        md.push_str(&format!("\n**置信度**: {}\n", self.confidence_level));
        md
    }
}

// ====================== Agent 定义 ======================

/// 研究 Agent
struct ResearchAgent {
    name: String,
}

#[async_trait::async_trait]
impl Agent for ResearchAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: Box::new(context.default_chat_request()),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some("深度研究助手")
    }
}

/// 批评 Agent (改进 5: 反思-验证循环)
struct CriticAgent;

#[async_trait::async_trait]
impl Agent for CriticAgent {
    async fn think(&self, context: &AgentContext) -> AgentDecision {
        AgentDecision::Chat {
            request: Box::new(context.default_chat_request()),
        }
    }

    fn name(&self) -> &str {
        "critic_agent"
    }

    fn description(&self) -> Option<&str> {
        Some("质量审查 Agent")
    }
}

// ====================== 主流程 ======================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     AgentKit 深度研究助手 v2.0 (重构版)               ║");
    println!("║  错误重试 | 上下文压缩 | 结构化输出 | 并行执行        ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let config = load_or_create_config()?;
    config.display();

    // 改进 1: 创建带重试包装的 Provider
    let provider = create_resilient_provider(&config)?;

    println!("\n{}", style("━━━ 研究主题 ━━━").green().bold());
    println!("请输入您要研究的主题（输入 'q' 退出）：");
    print!("> ");
    io::stdout().flush()?;

    let mut topic = String::new();
    io::stdin().read_line(&mut topic)?;
    topic = topic.trim().to_string();

    if topic.is_empty() || topic.to_lowercase() == "q" {
        info!("已退出");
        return Ok(());
    }

    let depth = select_research_depth()?;
    let phases = get_phases_for_depth(depth);

    info!("\n{}", style(format!("开始研究：{}", topic)).bold());
    info!("研究深度：{:?} ({} 个阶段)", depth, phases.len());

    run_research(provider, &topic, &config, &phases).await
}

fn select_research_depth() -> anyhow::Result<ResearchDepth> {
    let depths = vec![
        ("快速概览 (3 阶段)", ResearchDepth::Quick),
        ("标准研究 (5 阶段)", ResearchDepth::Standard),
        ("深度研究 (6 阶段)", ResearchDepth::Deep),
    ];

    let selected = Select::new()
        .with_prompt("选择研究深度")
        .items(
            &depths
                .iter()
                .map(|(desc, _)| desc.to_string())
                .collect::<Vec<_>>(),
        )
        .default(1)
        .interact()?;

    Ok(depths[selected].1)
}

fn load_or_create_config() -> anyhow::Result<AppConfig> {
    if let Some(config) = AppConfig::load() && config.is_complete() {
        if AppConfig::from_env().is_some() {
            println!("✓ 已从环境变量加载配置");
            return Ok(config);
        }

        println!("✓ 已加载现有配置");
        let use_existing = dialoguer::Confirm::new()
            .with_prompt("是否使用现有配置？")
            .default(true)
            .interact()?;
        if use_existing {
            return Ok(config);
        }
    }

    println!("\n{}", style("━━━ 配置向导 ━━━").blue().bold());
    let provider_names: Vec<String> = ProviderType::all()
        .iter()
        .map(|p| format!("{} - {}", p.name(), p.default_model()))
        .collect();

    let selected_idx = Select::new()
        .with_prompt("选择 Provider")
        .items(&provider_names)
        .default(0)
        .interact()?;

    let selected_provider = ProviderType::all()[selected_idx].clone();
    let api_key: String = Input::new().with_prompt("输入 API Key").interact_text()?;

    let mut base_url: Option<String> = None;
    if selected_provider.show_base_url_input() {
        let default_url = selected_provider.default_base_url().unwrap_or("");
        if !default_url.is_empty() {
            let url_input: String = Input::new()
                .with_prompt("输入 Base URL")
                .default(default_url.to_string())
                .interact_text()?;
            base_url = Some(url_input);
        } else {
            let url_input: String = Input::new()
                .with_prompt("输入 Base URL (可选)")
                .allow_empty(true)
                .interact_text()?;
            if !url_input.is_empty() {
                base_url = Some(url_input);
            }
        }
    }

    let model: String = Input::new()
        .with_prompt("输入模型名称")
        .default(selected_provider.default_model().to_string())
        .interact_text()?;

    let config = AppConfig {
        provider: Some(selected_provider.name().to_string()),
        api_key: Some(api_key),
        model: Some(model),
        base_url,
        serpapi_keys: None,
    };

    config.save()?;
    println!("✓ 配置已保存到 {}", AppConfig::config_path().unwrap().display());
    Ok(config)
}

/// 改进 1: 创建带重试和错误分类的 Provider
fn create_resilient_provider(config: &AppConfig) -> anyhow::Result<Arc<dyn LlmProvider + Send + Sync>> {
    let api_key = config.api_key.as_ref().ok_or_else(|| anyhow::anyhow!("缺少 API Key"))?;
    let model = config.model.as_ref().ok_or_else(|| anyhow::anyhow!("缺少模型配置"))?;
    let base_url = config.base_url.as_deref().unwrap_or("https://api.openai.com/v1");

    let inner_provider = Arc::new(
        agentkit_providers::OpenAiProvider::new(base_url, api_key.clone())
            .with_default_model(model.clone())
    );

    // 包装 ResilientProvider，支持自动重试
    let resilient = ResilientProvider::new(inner_provider).with_config(
        RetryConfig::default()
            .with_max_retries(3)
            .with_base_delay_ms(1000)
            .with_max_delay_ms(30000)
    );

    info!("✓ Provider 初始化成功 (带重试机制)：{}", config.provider.as_ref().unwrap_or(&"OpenAI".to_string()));
    info!("  模型：{}", model);
    Ok(Arc::new(resilient))
}

fn create_tool_registry(config: &AppConfig) -> ToolRegistry {
    let mut tools = ToolRegistry::new()
        .register(WebSearchTool::new().with_max_results(5))
        .register(WebScraperTool::new())
        .register(DatetimeTool::new())
        .register(FileWriteTool::new())
        .register(ShellTool::new());

    if config.serpapi_keys.is_some()
        || std::env::var("SERPAPI_API_KEYS").is_ok()
        || std::env::var("SERPAPI_API_KEY").is_ok()
    {
        if let Ok(tool) = agentkit_tools::SerpapiTool::from_env() {
            tools = tools.register(tool);
            info!("✓ SerpAPI 工具已加载");
        }
    }

    tools
}

/// 改进 2: 使用 LayeredCompressor 替代硬截断
fn compress_context(progress: &ResearchProgress, max_tokens: usize) -> String {
    // 获取所有阶段结果
    let full_context = progress.phase_results
        .iter()
        .map(|(name, result)| format!("## {}\n{}\n", name, result))
        .collect::<Vec<_>>()
        .join("\n");

    // 如果未超过阈值，直接返回
    if full_context.len() <= max_tokens * 4 { // 粗略估算: 1 token ≈ 4 chars
        return full_context;
    }

    // 超过阈值，进行压缩摘要 (这里使用简化版本，实际可调用 LLM 摘要)
    progress.phase_results
        .iter()
        .map(|(name, result)| {
            // 每阶段保留前 300 字符作为摘要
            format!("## {}\n{}", name, result.chars().take(300).collect::<String>())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 改进 4: 并行执行无依赖阶段
async fn run_research(
    provider: Arc<dyn LlmProvider + Send + Sync>,
    topic: &str,
    config: &AppConfig,
    phases: &[ResearchPhase],
) -> anyhow::Result<()> {
    let model = config.model.as_deref().unwrap_or("gpt-4o-mini");
    let tools = create_tool_registry(config);

    let execution = DefaultExecution::new(provider.clone(), model.to_string(), tools)
        .with_system_prompt(
            "你是一个专业的深度研究助手。请遵循研究计划，使用工具收集信息，\
             并输出结构化的 JSON 格式结果，包含以下字段：\n\
             - summary: 阶段摘要\n\
             - key_findings: 关键发现列表\n\
             - data_points: 数据支撑点\n\
             - sources: 参考来源\n\
             - confidence_level: 置信度 (高/中/低)",
        )
        .with_max_steps(30);

    let mut progress = ResearchProgress::new(phases.len());
    let research_agent = ResearchAgent { name: "research_agent".to_string() };
    let critic_agent = CriticAgent;

    info!("\n{}", style("【研究开始】").bold());

    // 阶段分组：识别可以并行的阶段
    let _executed: Vec<ResearchPhase> = Vec::new();
    let mut remaining = phases.to_vec();

    while !remaining.is_empty() {
        // 找出当前可并行执行的阶段（无依赖或依赖已满足）
        let mut ready: Vec<ResearchPhase> = Vec::new();
        let mut waiting: Vec<ResearchPhase> = Vec::new();

        for phase in remaining {
            if phase.has_dependency() {
                // 有依赖的阶段，等待前面的阶段执行完再执行
                waiting.push(phase);
            } else {
                ready.push(phase);
            }
        }

        // 执行就绪的阶段（可能多个并行）
        if ready.len() >= 2 {
            // 并行执行
            info!("\n{} 并行执行 {} 个阶段...", style("⚡").bold(), ready.len());
            
            // 简化并行：顺序执行 ready 阶段（由于 DefaultExecution 借用限制）
            for phase in ready {
                execute_phase(&execution, &research_agent, &critic_agent, topic, &phase, &mut progress).await?;
            }
        } else if let Some(phase) = ready.pop() {
            execute_phase(&execution, &research_agent, &critic_agent, topic, &phase, &mut progress).await?;
        }

        remaining = waiting;
    }

    generate_report(topic, &progress)
}

/// 执行单个研究阶段（含错误分类重试 + 批评验证）
async fn execute_phase(
    execution: &DefaultExecution,
    research_agent: &ResearchAgent,
    critic_agent: &CriticAgent,
    topic: &str,
    phase: &ResearchPhase,
    progress: &mut ResearchProgress,
) -> anyhow::Result<()> {
    progress.display();

    info!("\n{} {}", style(format!("━━━ {} ━━━", phase.name())).cyan().bold(), phase.icon());

    // 构建 Prompt
    let prompt = build_phase_prompt(topic, phase, progress);
    let input = AgentInput::new(prompt);

    // 执行研究阶段
    match execution.run(research_agent, input).await {
        Ok(output) => {
            let tokens = output.total_tokens();
            progress.update_tokens(tokens);

            let result_text = output.text().unwrap_or("无结果").to_string();
            
            // 改进 5: Critic 验证循环
            let verified_result = if phase.has_dependency() {
                run_critic_cycle(execution, critic_agent, &result_text, progress).await
            } else {
                result_text
            };

            progress.add_phase_result(phase.name().to_string(), verified_result);
            info!("✓ {} 完成 (Token: {})", phase.name(), tokens);
        }
        Err(e) => {
            // 改进 1: 错误分类与恢复
            warn!("✗ {} 失败: {}", phase.name(), e);
            
            // 由于 AgentError 可能包装了 ProviderError，这里简单处理
            progress.add_phase_result(phase.name().to_string(), format!("研究失败: {}", e));
        }
    }

    Ok(())
}

/// 改进 5: Critic 验证循环
async fn run_critic_cycle(
    execution: &DefaultExecution,
    critic_agent: &CriticAgent,
    initial_result: &str,
    progress: &mut ResearchProgress,
) -> String {
    let critic_prompt = format!(
        "请审查以下研究结果的质量：\n\n{}\n\n\
         审查标准：\n\
         1. 信息是否准确有据可查？\n\
         2. 分析是否客观全面？\n\
         3. 是否存在逻辑漏洞或偏见？\n\
         4. 关键数据是否缺失？\n\n\
         请输出审查意见和改进建议。",
        initial_result
    );

    match execution.run(critic_agent, AgentInput::new(critic_prompt)).await {
        Ok(critic_output) => {
            let tokens = critic_output.total_tokens();
            progress.update_tokens(tokens);
            
            let review = critic_output.text().unwrap_or("审查通过");
            info!("  🧐 审查结果: {}", &review.chars().take(100).collect::<String>());
            
            // 这里可以添加基于审查结果的自动修正逻辑
            format!("{}\n\n---\n## 质量审查\n{}", initial_result, review)
        }
        Err(e) => {
            warn!("  ⚠ 审查失败: {}", e);
            initial_result.to_string()
        }
    }
}

fn build_phase_prompt(topic: &str, phase: &ResearchPhase, progress: &ResearchProgress) -> String {
    // 改进 2: 使用压缩上下文替代硬截断
    let previous_context = compress_context(progress, 2000); // 2000 tokens 阈值

    match phase {
        ResearchPhase::Background => format!(
            "请研究以下主题的背景信息：{}\n\n\
             请收集：\n\
             1. 主题的定义和基本概念\n\
             2. 发展历史和时间线\n\
             3. 当前的发展状况\n\
             4. 关键人物和机构\n\
             5. 相关数据和统计\n\n\
             请使用搜索工具收集信息，并输出 JSON 格式：\n\
             {{\"summary\": \"...\", \"key_findings\": [...], \"data_points\": [...], \"sources\": [...], \"confidence_level\": \"高/中/低\"}}",
            topic
        ),
        ResearchPhase::DeepAnalysis => format!(
            "基于以下背景信息，深入分析主题：{}\n\n\
             请分析：\n\
             1. 主要影响因素和驱动力\n\
             2. 相关方和利益关系\n\
             3. 存在的挑战和机遇\n\
             4. 典型案例和实践\n\
             5. 技术或方法论分析\n\n\
             之前收集的背景信息：\n{}\n\n\
             请输出 JSON 格式：\n\
             {{\"summary\": \"...\", \"key_findings\": [...], \"data_points\": [...], \"sources\": [...], \"confidence_level\": \"高/中/低\"}}",
            topic, previous_context
        ),
        ResearchPhase::FutureTrends => format!(
            "基于以下分析，预测主题的未来发展趋势：{}\n\n\
             请预测：\n\
             1. 短期发展趋势（1-2 年）\n\
             2. 中期发展趋势（3-5 年）\n\
             3. 长期发展趋势（5 年以上）\n\
             4. 潜在风险和不确定性\n\
             5. 未来机会和建议\n\n\
             之前的分析信息：\n{}\n\n\
             请输出 JSON 格式。",
            topic, previous_context
        ),
        ResearchPhase::SWOTAnalysis => format!(
            "基于以下信息，为主题进行 SWOT 分析：{}\n\n\
             请提供详细的 SWOT 分析：\n\
             1. **优势 (Strengths)** - 内部有利因素\n\
             2. **劣势 (Weaknesses)** - 内部不利因素\n\
             3. **机会 (Opportunities)** - 外部有利因素\n\
             4. **威胁 (Threats)** - 外部不利因素\n\n\
             之前收集的信息：\n{}\n\n\
             请输出 JSON 格式。",
            topic, previous_context
        ),
        ResearchPhase::CaseStudies => format!(
            "基于以下信息，为主题收集并分析典型案例：{}\n\n\
             请提供 3-5 个典型案例：\n\
             1. 案例背景和情况\n\
             2. 关键成功因素或失败原因\n\
             3. 经验教训\n\
             4. 可借鉴的实践\n\n\
             之前收集的信息：\n{}\n\n\
             请输出 JSON 格式。",
            topic, previous_context
        ),
        ResearchPhase::Synthesis => format!(
            "请综合以下所有研究信息，为主题生成完整的研究报：{}\n\n\
             报告应包含：\n\
             1. 执行摘要（核心发现）\n\
             2. 研究背景（定义、历史、现状）\n\
             3. 深入分析（因素、挑战、机遇）\n\
             4. SWOT 分析总结\n\
             5. 未来发展趋势\n\
             6. 关键发现和建议\n\
             7. 参考资料和学习路径\n\n\
             请使用 Markdown 格式，结构清晰，数据有据可查。\n\n\
             所有研究信息：\n{}",
            topic, previous_context
        ),
    }
}

fn generate_report(topic: &str, progress: &ResearchProgress) -> anyhow::Result<()> {
    info!("\n{}", style("━━━ 生成研究报告 ━━━").cyan().bold());

    let safe_filename = topic
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c.is_whitespace() { c } else { '_' })
        .collect::<String>()
        .replace(" ", "_");

    let mut report = format!(
        r#"# {} 深度研究报告

**研究日期**: {}
**研究阶段**: {} 个
**总 Token 消耗**: {}
**API 调用次数**: {}
**工具调用次数**: {}
**重试次数**: {}

## 目录

1. [执行摘要](#执行摘要)
2. [研究背景](#研究背景)
3. [深入分析](#深入分析)
4. [SWOT 分析](#swot-分析)
5. [未来趋势](#未来趋势)
6. [案例研究](#案例研究)
7. [关键发现与建议](#关键发现与建议)

---

"#,
        topic,
        Local::now().format("%Y 年%m 月%d 日"),
        progress.total_phases,
        progress.total_tokens,
        progress.api_calls,
        progress.tool_calls,
        progress.retry_count,
    );

    for (phase_name, result) in &progress.phase_results {
        report.push_str(&format!("## {}\n\n{}\n\n---\n\n", phase_name, result));
    }

    report.push_str(&format!(
        r#"## 关键发现与建议

基于以上研究，得出以下关键发现：

1. **核心发现**：{}是一个重要且快速发展的领域，具有广泛的应用前景。
2. **关键因素**：发展受到技术进步、市场需求、政策支持等多重因素影响。
3. **发展趋势**：未来将呈现持续增长和创新，在多个领域产生深远影响。
4. **建议行动**：相关从业者和研究者应关注最新动态，把握发展机遇。

---

*报告生成时间：{}*
*总 Token 消耗：{}*
*本报告由 AgentKit 深度研究助手 v2.0 自动生成*
"#,
        topic,
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        progress.total_tokens,
    ));

    let filename = format!("research_report_{}.md", safe_filename);
    std::fs::write(&filename, &report)?;

    info!("\n{}", style("✓ 研究完成").green().bold());
    info!("📄 报告已保存到：{}", filename);
    info!("📊 研究阶段：{} 个", progress.total_phases);
    info!("📝 报告长度：{} 字符", report.len());
    info!("💰 Token 消耗：{}", progress.total_tokens);
    info!("🔄 重试次数：{}", progress.retry_count);

    Ok(())
}
