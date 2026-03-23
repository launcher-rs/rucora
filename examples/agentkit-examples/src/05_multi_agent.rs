//! 多 Agent 协作示例
//!
//! 展示多个 Agent 之间如何协作完成复杂任务
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --bin multi-agent
//! ```

mod utils;

use agentkit::agent::DefaultAgent;
use agentkit::prelude::*;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use utils::MockProvider;

/// 专用 Agent：翻译助手
struct TranslatorAgent {
    agent: DefaultAgent<MockProvider>,
}

impl TranslatorAgent {
    fn new() -> Self {
        let provider =
            MockProvider::with_response("Translation: Hello, World! (翻译：你好，世界！)");
        let agent = DefaultAgent::builder()
            .provider(provider)
            .system_prompt("你是一个专业的翻译助手，擅长中英文翻译")
            .build();

        Self { agent }
    }

    async fn translate(&self, text: &str) -> anyhow::Result<String> {
        let input = AgentInput::new(format!("请将以下内容翻译成中文：{}", text));
        let output = self.agent.run(input).await?;
        Ok(output.text().unwrap_or("翻译失败").to_string())
    }
}

/// 专用 Agent：总结助手
struct SummarizerAgent {
    agent: DefaultAgent<MockProvider>,
}

impl SummarizerAgent {
    fn new() -> Self {
        let provider =
            MockProvider::with_response("总结：这是一个关于 AgentKit 多 Agent 协作的演示。");
        let agent = DefaultAgent::builder()
            .provider(provider)
            .system_prompt("你是一个专业的总结助手，擅长提取关键信息并生成简洁的总结")
            .build();

        Self { agent }
    }

    async fn summarize(&self, text: &str) -> anyhow::Result<String> {
        let input = AgentInput::new(format!("请总结以下内容：{}", text));
        let output = self.agent.run(input).await?;
        Ok(output.text().unwrap_or("总结失败").to_string())
    }
}

/// 专用 Agent：质量检查助手
struct QualityCheckerAgent {
    agent: DefaultAgent<MockProvider>,
}

impl QualityCheckerAgent {
    fn new() -> Self {
        let provider = MockProvider::with_response("✓ 质量检查通过：内容清晰准确，格式规范。");
        let agent = DefaultAgent::builder()
            .provider(provider)
            .system_prompt("你是一个质量检查助手，负责检查内容的准确性和规范性")
            .build();

        Self { agent }
    }

    async fn check(&self, text: &str) -> anyhow::Result<String> {
        let input = AgentInput::new(format!("请检查以下内容的质量：{}", text));
        let output = self.agent.run(input).await?;
        Ok(output.text().unwrap_or("检查失败").to_string())
    }
}

/// 多 Agent 协作管理器
struct MultiAgentCoordinator {
    translator: TranslatorAgent,
    summarizer: SummarizerAgent,
    checker: QualityCheckerAgent,
}

impl MultiAgentCoordinator {
    fn new() -> Self {
        Self {
            translator: TranslatorAgent::new(),
            summarizer: SummarizerAgent::new(),
            checker: QualityCheckerAgent::new(),
        }
    }

    /// 协作流程：翻译 -> 总结 -> 质量检查
    async fn process_document(&self, original_text: &str) -> anyhow::Result<ProcessResult> {
        info!("开始多 Agent 协作流程...\n");

        // 步骤 1：翻译
        info!("步骤 1/3: 翻译助手处理中...");
        let translated = self.translator.translate(original_text).await?;
        info!("✓ 翻译完成：{}\n", translated);

        // 步骤 2：总结
        info!("步骤 2/3: 总结助手处理中...");
        let summarized = self.summarizer.summarize(&translated).await?;
        info!("✓ 总结完成：{}\n", summarized);

        // 步骤 3：质量检查
        info!("步骤 3/3: 质量检查助手处理中...");
        let quality_report = self.checker.check(&summarized).await?;
        info!("✓ 质量检查完成：{}\n", quality_report);

        Ok(ProcessResult {
            original: original_text.to_string(),
            translated,
            summarized,
            quality_report,
        })
    }
}

/// 处理结果
struct ProcessResult {
    original: String,
    translated: String,
    summarized: String,
    quality_report: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════════════════════╗");
    info!("║         多 Agent 协作示例                                ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    // 创建协作管理器
    info!("创建多 Agent 协作系统:");
    let coordinator = MultiAgentCoordinator::new();
    info!("✓ 已创建 3 个专用 Agent:");
    info!("  - TranslatorAgent (翻译助手)");
    info!("  - SummarizerAgent (总结助手)");
    info!("  - QualityCheckerAgent (质量检查助手)\n");

    // 测试用例
    let test_document =
        "Hello, this is a demonstration of multi-agent collaboration using AgentKit. 
This system allows multiple specialized agents to work together on complex tasks.";

    info!("原始文档:\n{}\n", test_document);

    // 执行协作流程
    let result = coordinator.process_document(test_document).await?;

    // 显示最终结果
    info!("╔════════════════════════════════════════════════════════╗");
    info!("║                  协作流程完成                           ║");
    info!("╚════════════════════════════════════════════════════════╝\n");

    info!("最终结果:");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("【原文】\n{}\n", result.original);
    info!("【翻译】\n{}\n", result.translated);
    info!("【总结】\n{}\n", result.summarized);
    info!("【质量报告】\n{}\n", result.quality_report);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    info!("=== 示例完成 ===");
    info!("提示：这是一个演示，使用 Mock Provider。");
    info!("      真实场景中，每个 Agent 可以调用真实的 LLM API。");

    Ok(())
}
