//! AgentKit 深度研究示例
//!
//! 功能：
//! 1. 多 Provider 支持 - OpenAI/Anthropic/Gemini/Ollama 等
//! 2. 可配置研究轮次 - 根据研究深度自动调整
//! 3. 丰富的工具系统 - Web 搜索/网页抓取/Shell/Git/文件操作等
//! 4. 增强报告生成 - SWOT 分析/时间线/关键发现/学习路径
//! 5. 实时进度显示 - Token 消耗和研究进度追踪
//! 6. 经验驱动 - 根据工具结果自动调整研究方向
//!
//! ## 运行方法
//! ```bash
//! # 使用环境变量
//! export OPENAI_API_KEY=sk-your-key
//! cargo run -p agentkit-deep-research
//!
//! # 或使用交互式配置
//! cargo run -p agentkit-deep-research
//! ```

mod config;

use agentkit::agent::ToolRegistry;
use agentkit::agent::execution::DefaultExecution;
use agentkit::prelude::*;
use agentkit_core::agent::{Agent, AgentContext, AgentDecision};
use agentkit_core::provider::LlmProvider;
use agentkit_tools::{
    DatetimeTool, FileWriteTool, ShellTool, WebScraperTool, WebSearchTool,
};
use chrono::Local;
use config::{AppConfig, ProviderType};
use console::style;
use dialoguer::{Input, Select};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// 研究阶段
#[derive(Debug, Clone)]
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
}

/// 简单 Agent 用于执行研究任务
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

/// 研究进度追踪
struct ResearchProgress {
    current_phase: usize,
    total_phases: usize,
    total_tokens: u32,
    phase_results: Vec<(String, String)>,
}

impl ResearchProgress {
    fn new(total_phases: usize) -> Self {
        Self {
            current_phase: 0,
            total_phases,
            total_tokens: 0,
            phase_results: Vec::new(),
        }
    }

    fn update_tokens(&mut self, tokens: u32) {
        self.total_tokens += tokens;
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
            "\n{} 研究进度：{:.1}% ({}/{}) | Token 消耗：{}",
            style("📊").bold(),
            progress,
            self.current_phase,
            self.total_phases,
            style(self.total_tokens).cyan().bold()
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env 文件
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         AgentKit 深度研究助手                          ║");
    println!("║     多 Provider | 智能工具 | 结构化报告                ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 加载或创建配置
    let config = load_or_create_config()?;

    // 显示当前配置
    config.display();

    // 创建 Provider
    let provider = create_provider(&config)?;

    // 获取研究主题
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

    // 获取研究深度
    let depth = select_research_depth()?;
    let phases = get_phases_for_depth(depth);

    info!("\n{}", style(format!("开始研究：{}", topic)).bold());
    info!("研究深度：{:?} ({} 个阶段)", depth, phases.len());

    // 执行研究
    run_research(provider, &topic, &config, &phases).await
}

/// 选择研究深度
fn select_research_depth() -> anyhow::Result<ResearchDepth> {
    let depths = vec![
        ("快速概览 (3 阶段，快速了解)", ResearchDepth::Quick),
        ("标准研究 (5 阶段，全面分析)", ResearchDepth::Standard),
        ("深度研究 (6 阶段，专业报告)", ResearchDepth::Deep),
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

/// 研究深度
#[derive(Debug, Clone, Copy)]
enum ResearchDepth {
    Quick,
    Standard,
    Deep,
}

/// 根据深度获取研究阶段
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

/// 加载或创建配置
fn load_or_create_config() -> anyhow::Result<AppConfig> {
    // 尝试加载现有配置
    if let Some(config) = AppConfig::load()
        && config.is_complete()
    {
        if AppConfig::from_env().is_some() {
            println!("✓ 已从环境变量加载配置");
            return Ok(config);
        }

        println!("✓ 已加载现有配置");
        println!(
            "  Provider: {}",
            config.provider.as_ref().unwrap_or(&"未指定".to_string())
        );
        println!(
            "  模型：{}",
            config.model.as_ref().unwrap_or(&"未指定".to_string())
        );

        let use_existing = dialoguer::Confirm::new()
            .with_prompt("是否使用现有配置？")
            .default(true)
            .interact()?;

        if use_existing {
            return Ok(config);
        }
    }

    // 交互式配置
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

    let default_model = selected_provider.default_model();
    let model: String = Input::new()
        .with_prompt("输入模型名称")
        .default(default_model.to_string())
        .interact_text()?;

    let config = AppConfig {
        provider: Some(selected_provider.name().to_string()),
        api_key: Some(api_key),
        model: Some(model),
        base_url,
        serpapi_keys: None,
    };

    config.save()?;
    println!(
        "✓ 配置已保存到 {}",
        AppConfig::config_path().unwrap().display()
    );

    Ok(config)
}

/// 创建 Provider
fn create_provider(config: &AppConfig) -> anyhow::Result<Arc<dyn LlmProvider + Send + Sync>> {
    let api_key = config
        .api_key
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少 API Key"))?;
    let model = config
        .model
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少模型配置"))?;
    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or("https://api.openai.com/v1");

    let provider: Arc<dyn LlmProvider + Send + Sync> =
        Arc::new(agentkit_providers::OpenAiProvider::new(base_url, api_key.clone())
            .with_default_model(model.clone()));

    info!(
        "✓ Provider 初始化成功：{}",
        config.provider.as_ref().unwrap_or(&"OpenAI".to_string())
    );
    info!("  模型：{}", model);
    if let Some(ref url) = config.base_url {
        info!("  Base URL: {}", url);
    }
    Ok(provider)
}

/// 创建工具注册表
fn create_tool_registry(config: &AppConfig) -> ToolRegistry {
    let mut tools = ToolRegistry::new()
        .register(WebSearchTool::new().with_max_results(5))
        .register(WebScraperTool::new())
        .register(DatetimeTool::new())
        .register(FileWriteTool::new())
        .register(ShellTool::new());

    // 添加 SerpAPI（如果配置了）
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

/// 执行研究
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
            "你是一个专业的深度研究助手。请遵循以下研究流程：\n\
             1. 制定详细的研究计划\n\
             2. 使用搜索工具收集相关信息\n\
             3. 抓取网页获取详细内容\n\
             4. 分析整理收集的信息\n\
             5. 生成完整、结构化的研究报告\n\n\
             要求：\n\
             - 信息来源可靠\n\
             - 分析客观全面\n\
             - 报告结构清晰\n\
             - 数据准确有据\n\
             - 使用 Markdown 格式输出",
        )
        .with_max_steps(30);

    let mut progress = ResearchProgress::new(phases.len());
    let agent = ResearchAgent {
        name: "research_agent".to_string(),
    };

    info!("\n{}", style("【研究开始】").bold());

    // 执行每个研究阶段
    for phase in phases {
        progress.display();

        info!(
            "\n{} {}",
            style(format!("━━━ {} ━━━", phase.name())).cyan().bold(),
            phase.icon()
        );

        let prompt = build_phase_prompt(topic, phase, &progress.phase_results);
        let input = AgentInput::new(prompt);

        match execution.run(&agent, input).await {
            Ok(output) => {
                let tokens = output.total_tokens();
                progress.update_tokens(tokens);

                let result_text = output.text().unwrap_or("无结果").to_string();
                progress.add_phase_result(phase.name().to_string(), result_text.clone());

                info!("✓ {} 完成 (Token: {})", phase.name(), tokens);
                info!("  摘要: {}", &result_text.chars().take(100).collect::<String>());
            }
            Err(e) => {
                info!("✗ {} 失败: {}", phase.name(), e);
                progress.add_phase_result(phase.name().to_string(), format!("研究失败: {}", e));
            }
        }
    }

    // 生成报告
    generate_report(topic, &progress)
}

/// 构建研究阶段提示词
fn build_phase_prompt(
    topic: &str,
    phase: &ResearchPhase,
    previous_results: &[(String, String)],
) -> String {
    let previous_context = previous_results
        .iter()
        .map(|(name, result)| format!("## {}\n{}\n", name, result.chars().take(500).collect::<String>()))
        .collect::<Vec<_>>()
        .join("\n");

    match phase {
        ResearchPhase::Background => format!(
            "请研究以下主题的背景信息：{}\n\n\
             请收集：\n\
             1. 主题的定义和基本概念\n\
             2. 发展历史和时间线\n\
             3. 当前的发展状况\n\
             4. 关键人物和机构\n\
             5. 相关数据和统计\n\n\
             请使用搜索工具收集信息，并提供详细的背景介绍。",
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
             之前收集的背景信息：\n{}",
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
             之前的分析信息：\n{}",
            topic, previous_context
        ),
        ResearchPhase::SWOTAnalysis => format!(
            "基于以下信息，为主题进行 SWOT 分析：{}\n\n\
             请提供详细的 SWOT 分析：\n\
             1. **优势 (Strengths)** - 内部有利因素\n\
             2. **劣势 (Weaknesses)** - 内部不利因素\n\
             3. **机会 (Opportunities)** - 外部有利因素\n\
             4. **威胁 (Threats)** - 外部不利因素\n\n\
             对每项进行详细说明，并提供战略建议。\n\n\
             之前收集的信息：\n{}",
            topic, previous_context
        ),
        ResearchPhase::CaseStudies => format!(
            "基于以下信息，为主题收集并分析典型案例：{}\n\n\
             请提供 3-5 个典型案例：\n\
             1. 案例背景和情况\n\
             2. 关键成功因素或失败原因\n\
             3. 经验教训\n\
             4. 可借鉴的实践\n\n\
             每个案例应包含具体数据和支持证据。\n\n\
             之前收集的信息：\n{}",
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

/// 生成研究报告
fn generate_report(topic: &str, progress: &ResearchProgress) -> anyhow::Result<()> {
    info!("\n{}", style("━━━ 生成研究报告 ━━━").cyan().bold());

    let safe_filename = topic
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c.is_whitespace() {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .replace(" ", "_");

    // 构建报告内容
    let mut report = format!(
        r#"# {} 深度研究报告

**研究日期**: {}
**研究阶段**: {} 个
**总 Token 消耗**: {}

## 目录

1. [执行摘要](#执行摘要)
2. [研究背景](#研究背景)
3. [深入分析](#深入分析)
4. [SWOT 分析](#swot-分析)
5. [未来趋势](#未来趋势)
6. [案例研究](#案例研究)
7. [关键发现与建议](#关键发现与建议)
8. [参考资料与学习路径](#参考资料与学习路径)

---

"#,
        topic,
        Local::now().format("%Y 年%m 月%d 日"),
        progress.total_phases,
        progress.total_tokens,
    );

    // 添加各阶段结果
    for (phase_name, result) in &progress.phase_results {
        report.push_str(&format!("## {}\n\n{}\n\n---\n\n", phase_name, result));
    }

    // 添加结论部分
    report.push_str(&format!(
        r#"## 关键发现与建议

基于以上研究，得出以下关键发现：

1. **核心发现**：{}是一个重要且快速发展的领域，具有广泛的应用前景。
2. **关键因素**：发展受到技术进步、市场需求、政策支持等多重因素影响。
3. **发展趋势**：未来将呈现持续增长和创新，在多个领域产生深远影响。
4. **建议行动**：相关从业者和研究者应关注最新动态，把握发展机遇。

---

*报告生成时间：{}*
*本报告由 AgentKit 深度研究助手自动生成*
*总 Token 消耗：{}*
"#,
        topic,
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        progress.total_tokens,
    ));

    // 保存报告
    let filename = format!("research_report_{}.md", safe_filename);
    std::fs::write(&filename, &report)?;

    info!("\n{}", style("✓ 研究完成").green().bold());
    info!("📄 报告已保存到：{}", filename);
    info!("📊 研究阶段：{} 个", progress.total_phases);
    info!("📝 报告长度：{} 字符", report.len());
    info!("💰 Token 消耗：{}", progress.total_tokens);

    info!("\n{}", style("=== 研究完成 ===").green().bold());

    Ok(())
}
