//! AgentKit Prompt 模板示例
//!
//! 展示如何使用 Prompt 模板系统。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 09_prompt
//! ```

use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit Prompt 模板示例            ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("═══════════════════════════════════════");
    info!("Prompt 模板系统功能:");
    info!("═══════════════════════════════════════\n");

    // 示例 1：简单变量替换
    info!("1. 简单变量替换:");
    let template1 = "你好，{{name}}！你是{{role}}。";
    info!("   模板：{}", template1);

    let prompt1 = template1
        .replace("{{name}}", "张三")
        .replace("{{role}}", "工程师");
    info!("   渲染后：{}\n", prompt1);

    // 示例 2：系统提示词模板
    info!("2. 系统提示词模板:");
    let system_template = "你是{{company}}的{{role}}助手。
你的职责是：
- {{duty1}}
- {{duty2}}
- {{duty3}}

请专业、友好地回答用户问题。";

    info!("   模板：\n{}", system_template);

    let system_prompt = system_template
        .replace("{{company}}", "AgentKit")
        .replace("{{role}}", "技术")
        .replace("{{duty1}}", "解答技术问题")
        .replace("{{duty2}}", "提供代码示例")
        .replace("{{duty3}}", "帮助调试程序");

    info!("   渲染后：\n{}\n", system_prompt);

    // 示例 3：Few-Shot 模板
    info!("3. Few-Shot 模板:");
    let few_shot_template = "请将以下中文翻译成英文：

示例 1:
中文：你好
英文：Hello

示例 2:
中文：再见
英文：Goodbye

示例 3:
中文：{{input}}
英文：";

    info!("   模板：\n{}", few_shot_template);

    let prompt = few_shot_template.replace("{{input}}", "谢谢");
    info!("   渲染后：\n{}\n", prompt);

    info!("═══════════════════════════════════════");
    info!("Prompt 模板的优势:");
    info!("═══════════════════════════════════════");
    info!("1. 可复用 - 一次定义，多次使用");
    info!("2. 可维护 - 集中管理所有提示词");
    info!("3. 可测试 - 独立测试模板逻辑");
    info!("4. 可配置 - 运行时动态调整");
    info!("═══════════════════════════════════════\n");

    info!("示例完成！");

    Ok(())
}
