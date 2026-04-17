//! AgentKit 深度研究示例
//!
//! ## 功能特性
//!
//! - **多阶段研究流程**：搜索收集 → 深度精读 → 综合报告
//! - **工具自动注入**：DefaultExecution 自动向 LLM 注入工具定义
//! - **搜索工具降级**：优先 Tavily，无则用 BrowseTool 抓取 DuckDuckGo
//! - **增强配置集成**：重试 + 超时控制，提升研究稳定性
//! - **完整报告生成**：自动保存 Markdown 格式的研究报告
//!
//! ## 发现的 agentkit 缺陷及修复说明
//!
//! ### 缺陷 1：单次 run 无法生成完整深度报告
//! - **问题**：`DefaultExecution::run` 是单轮工具调用循环，LLM 往往完成搜索后就停止，
//!            不会继续精读和生成完整报告。
//! - **修复**：引入 `DeepResearchEngine`，将研究分为三个独立阶段，每阶段有专属
//!            系统提示词和工具集，前一阶段输出作为下一阶段的输入上下文。
//!
//! ### 缺陷 2：工具调用无重试和超时
//! - **问题**：网络工具失败直接返回错误，研究过程脆弱。
//! - **修复**：使用本次 PR 新增的 `ToolCallEnhancedConfig`，为搜索工具配置
//!            指数退避重试（3次）+ 60s 超时。
//!
//! ### 缺陷 3：系统提示词缺乏强制约束
//! - **问题**：原示例的系统提示词过于宽泛，LLM 容易偷懒跳过搜索。
//! - **修复**：为每个阶段设计精确的系统提示词，强制步骤执行顺序和输出格式。
//!
//! ### 缺陷 4：缺乏搜索工具降级策略
//! - **问题**：无 Tavily Key 时只有 WebFetchTool，但 LLM 不知道如何用它搜索。
//! - **修复**：无 Tavily 时，自动注入 BrowseTool，并在提示词中给出
//!            DuckDuckGo HTML 搜索 URL 格式，引导 LLM 主动搜索。

mod config;
mod reporter;
mod research_agent;

use agentkit::agent::execution::DefaultExecution;
use agentkit::agent::{
    CacheConfig, RetryConfig, TimeoutConfig, ToolCallEnhancedConfig, ToolRegistry,
};
use agentkit::provider::resilient::{ResilientProvider, RetryConfig as ProviderRetryConfig};
use agentkit_core::provider::LlmProvider;
use agentkit_tools::{BrowseTool, DatetimeTool, TavilyTool, WebFetchTool};
use config::{AppConfig, ProviderType};
use console::style;
use dialoguer::{Input, Select};
use reporter::Reporter;
use research_agent::DeepResearchEngine;
use std::sync::Arc;
use std::time::Duration;
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

// ====================== 研究主题（固定，不需要修改） ======================

/// 测试用研究主题
const RESEARCH_TOPIC: &str = "分析中东局势";

// ====================== 主流程 ======================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志（只显示 INFO 级别，减少干扰）
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    print_banner();

    // 1. 加载配置
    let config = load_or_create_config()?;
    config.display();

    // 2. 创建带重试的 Provider
    let provider = create_resilient_provider(&config)?;

    // 3. 研究主题（固定，不需修改）
    let topic = RESEARCH_TOPIC;
    println!("\n{}", style("━━━ 研究主题 ━━━").green().bold());
    println!("  {}", style(topic).cyan().bold());

    // 4. 构建多阶段研究引擎
    let engine = build_research_engine(&config, &provider)?;

    // 5. 执行深度研究
    println!("\n{}", style("━━━ 开始深度研究 ━━━").blue().bold());
    info!("开始研究主题：{}", topic);

    match engine.run(topic).await {
        Ok((report, phases)) => {
            let reporter = Reporter::default();

            // 保存完整报告
            match reporter.save_report(topic, &report) {
                Ok(path) => {
                    reporter.print_summary(topic, &phases, &path);

                    // 同时保存阶段性记录（调试用）
                    if let Err(e) = reporter.save_phase_results(topic, &phases) {
                        warn!("保存阶段记录失败（非致命）：{}", e);
                    }
                }
                Err(e) => {
                    warn!("保存报告失败：{}，直接打印到标准输出", e);
                    println!("\n{}", report);
                }
            }
        }
        Err(e) => {
            eprintln!("\n{} 研究失败：{}", style("✗").red().bold(), e);
            eprintln!("提示：请检查 API Key 配置和网络连接。");
            std::process::exit(1);
        }
    }

    Ok(())
}

// ====================== 引擎构建 ======================

/// 构建多阶段深度研究引擎
///
/// 三个阶段使用不同的工具集和系统提示词：
/// - 阶段 1（搜索）：Tavily/Browse + WebFetch + Datetime，多搜索工具
/// - 阶段 2（精读）：Browse + WebFetch，专注内容精读
/// - 阶段 3（综合）：无工具，纯文本综合输出
fn build_research_engine(
    config: &AppConfig,
    provider: &Arc<dyn LlmProvider + Send + Sync>,
) -> anyhow::Result<DeepResearchEngine> {
    let model = config.model.as_deref().unwrap_or("gpt-4o-mini");
    let has_tavily = config.tavily_keys.is_some();

    // ── 工具调用增强配置 ───────────────────────────────────
    // 网络工具容易超时，配置重试和超时控制（使用本次 PR 新增功能）
    let enhanced_config = ToolCallEnhancedConfig::new()
        // 网络请求失败时最多重试 3 次，指数退避
        .with_retry(RetryConfig::exponential(3))
        // 搜索工具超时 60s，精读工具超时 45s
        .with_timeout(
            TimeoutConfig::default_timeout(Duration::from_secs(60))
                .with_tool_timeout("web_fetch", Duration::from_secs(45))
                .with_tool_timeout("browse", Duration::from_secs(45)),
        )
        // 对相同输入的搜索结果缓存 5 分钟，节省 API 调用
        .with_cache(CacheConfig {
            enabled: true,
            default_ttl: Duration::from_secs(300),
            max_entries: 100,
            ..Default::default()
        });

    // ── 阶段 1：搜索收集执行器 ────────────────────────────
    let mut search_registry = ToolRegistry::new()
        .register(DatetimeTool::new())
        .register(WebFetchTool::new())
        .register(BrowseTool::new());

    if let Some(tavily_keys) = &config.tavily_keys {
        search_registry = search_registry.register(TavilyTool::with_keys(tavily_keys.clone()));
        info!("✓ Tavily 搜索工具已启用");
    } else {
        info!("⚠ 未配置 Tavily Key，使用 Browse + WebFetch 作为搜索替代");
    }

    let search_system_prompt = build_search_system_prompt(has_tavily);

    let search_execution = DefaultExecution::new(provider.clone(), model, search_registry)
        .with_system_prompt(search_system_prompt)
        .with_max_steps(30)   // 允许足够多的工具调用步数
        .with_max_tool_concurrency(3) // 并发执行搜索以加速
        .with_enhanced_config(enhanced_config.clone());

    // ── 阶段 2：深度精读执行器 ────────────────────────────
    let read_registry = ToolRegistry::new()
        .register(WebFetchTool::new())
        .register(BrowseTool::new());

    let read_execution = DefaultExecution::new(provider.clone(), model, read_registry)
        .with_system_prompt(
            "你是一名专业内容分析师。你的唯一任务是：\n\
             1. 使用 web_fetch 或 browse 工具读取指定的网页内容\n\
             2. 对每个页面提取核心观点、关键数据和重要引用\n\
             3. 绝不虚构内容，所有信息必须来自实际抓取的页面\n\
             4. 如果页面无法访问，直接说明并跳过",
        )
        .with_max_steps(20)
        .with_max_tool_concurrency(2)
        .with_enhanced_config(enhanced_config.clone());

    // ── 阶段 3：综合报告执行器（无工具）──────────────────
    let synthesize_registry = ToolRegistry::new(); // 报告阶段不需要工具
    let synthesize_execution =
        DefaultExecution::new(provider.clone(), model, synthesize_registry)
            .with_system_prompt(
                "你是一名顶级研究报告撰写专家，擅长将复杂的研究材料整理为\
                 结构清晰、深度充分的专业报告。\n\
                 要求：\n\
                 - 报告必须完整，不得省略任何章节\n\
                 - 所有观点必须有来源支撑\n\
                 - 数据和引用必须准确，注明出处\n\
                 - 保持客观中立，呈现多方观点",
            )
            .with_max_steps(5) // 报告阶段只需要少量步骤
            .with_enhanced_config(enhanced_config);

    info!(
        "✓ 研究引擎构建完成 | 模型: {} | Tavily: {}",
        model,
        if has_tavily { "启用" } else { "禁用" }
    );

    Ok(DeepResearchEngine::new(
        search_execution,
        read_execution,
        synthesize_execution,
    ))
}

/// 根据是否有 Tavily，构建不同的搜索阶段系统提示词
fn build_search_system_prompt(has_tavily: bool) -> String {
    let search_tool_guide = if has_tavily {
        r#"## 可用搜索工具

### tavily_search（推荐，AI 增强搜索）
```json
{"query": "搜索关键词", "max_results": 8, "search_depth": "advanced"}
```
- 适合：快速获取高质量摘要和相关链接
- 每个子问题至少搜索一次

### web_fetch（网页内容抓取）
```json
{"url": "https://example.com"}
```
- 适合：获取特定页面的完整内容

### browse（高级浏览器工具）
```json
{"action": "navigate", "url": "https://example.com"}
```
然后：
```json
{"action": "get_content", "session": "default"}
```
- 适合：处理动态页面或需要会话的网站"#
    } else {
        r#"## 可用搜索工具（无 Tavily，使用免费替代方案）

### browse + DuckDuckGo（免费搜索）
步骤 1：先用 browse 工具打开 DuckDuckGo 搜索结果页：
```json
{"action": "navigate", "url": "https://html.duckduckgo.com/html/?q=YOUR_QUERY"}
```
将 YOUR_QUERY 替换为 URL 编码的搜索词（空格用 + 替换）

步骤 2：获取搜索结果列表：
```json
{"action": "get_content", "session": "default"}
```

步骤 3：选择其中 3-5 个链接，用 web_fetch 或 browse 逐一访问：
```json
{"url": "https://选中的链接"}
```

### web_fetch（直接抓取已知 URL）
```json
{"url": "https://example.com"}
```

**注意**：没有 Tavily 时，你需要主动使用 DuckDuckGo 搜索来找到相关页面，不要直接回答问题！"#
    };

    format!(
        r#"你是一名资深研究分析师，专注于信息收集和事实核查。

**核心原则**：
- 严禁使用训练数据中的知识直接回答，必须搜索最新信息
- 每个子问题至少搜索一次，重要问题多角度搜索
- 所有信息必须标注来源 URL

{search_tool_guide}

### datetime（获取当前时间）
```json
{{}}
```
- 第一步必须调用，确认研究时间节点"#
    )
}

// ====================== 辅助函数 ======================

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         AgentKit 深度研究示例 v2.0                      ║");
    println!("║  多阶段流程: 搜索 → 精读 → 综合报告                    ║");
    println!("║  新特性: 重试/超时/缓存 (ToolCallEnhancedConfig)        ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
}

fn load_or_create_config() -> anyhow::Result<AppConfig> {
    if let Some(config) = AppConfig::load()
        && config.is_complete()
    {
        println!("✓ 已加载现有配置");
        return Ok(config);
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
    let model: String = Input::new()
        .with_prompt("输入模型名称")
        .default(selected_provider.default_model().to_string())
        .interact_text()?;

    // 可选：Tavily API Key
    let tavily_key: String = Input::new()
        .with_prompt("输入 Tavily API Key（可选，直接回车跳过）")
        .allow_empty(true)
        .interact_text()?;

    let tavily_keys = if tavily_key.trim().is_empty() {
        None
    } else {
        Some(vec![tavily_key.trim().to_string()])
    };

    let config = AppConfig {
        provider: Some(selected_provider.name().to_string()),
        api_key: Some(api_key),
        model: Some(model),
        base_url: selected_provider.default_base_url().map(String::from),
        tavily_keys,
        // 向后兼容保留的字段
        serpapi_keys: None,
    };
    config.save()?;
    Ok(config)
}

fn create_resilient_provider(
    config: &AppConfig,
) -> anyhow::Result<Arc<dyn LlmProvider + Send + Sync>> {
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

    let inner = Arc::new(
        agentkit_providers::OpenAiProvider::new(base_url, api_key.clone())
            .with_default_model(model.clone()),
    );

    Ok(Arc::new(
        ResilientProvider::new(inner).with_config(ProviderRetryConfig::default()),
    ))
}
