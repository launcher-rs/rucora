//! AgentKit 结构化数据提取示例
//!
//! 展示如何使用 Extractor 从非结构化文本中提取结构化数据。
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example 04_extractor
//! ```

use agentkit::agent::extractor::Extractor;
use agentkit::provider::OpenAiProvider;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// 代表一个人的记录
#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
struct Person {
    /// 姓名（如果提到）
    #[schemars(required)]
    pub name: Option<String>,
    /// 年龄（如果提到）
    #[schemars(required)]
    pub age: Option<u8>,
    /// 职业（如果提到）
    #[schemars(required)]
    pub profession: Option<String>,
    /// 所在城市（如果提到）
    #[schemars(required)]
    pub city: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit 结构化数据提取示例         ║");
    info!("╚════════════════════════════════════════╝\n");

    // 检查配置
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("OPENAI_BASE_URL").is_err() {
        info!("⚠ 未设置 API 配置");
        info!("   使用 OpenAI: export OPENAI_API_KEY=sk-your-key");
        info!("   使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }
    let model_name = std::env::var("MODEL_NAME").expect("没有设置环境变量MODEL_NAME");

    // 创建 Provider
    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 示例 1：提取人物信息
    info!("═══════════════════════════════════════");
    info!("示例 1: 提取人物信息");
    info!("═══════════════════════════════════════\n");

    let person_extractor = Extractor::<_, Person>::builder(provider, model_name)
        .preamble("只提取明确提到的信息，不要推测。")
        .retries(2)
        .build();

    let text1 = "John Doe 是 30 岁的软件工程师，住在纽约。";
    info!("输入文本：{}", text1);

    match person_extractor.extract(text1).await {
        Ok(person) => {
            info!("✓ 提取成功！");
            info!("  姓名：{:?}", person.name);
            info!("  年龄：{:?}", person.age);
            info!("  职业：{:?}", person.profession);
            info!("  城市：{:?}\n", person.city);
        }
        Err(e) => {
            info!("✗ 提取失败：{}\n", e);
        }
    }

    // 示例 2：带 Usage 追踪
    info!("═══════════════════════════════════════");
    info!("示例 2: 带 Usage 追踪");
    info!("═══════════════════════════════════════\n");

    let text2 = "Jane Smith 是 25 岁的数据科学家，在旧金山工作。";
    info!("输入文本：{}", text2);

    match person_extractor.extract_with_usage(text2).await {
        Ok(response) => {
            info!("✓ 提取成功！");
            info!("  姓名：{:?}", response.data.name);
            info!("  年龄：{:?}", response.data.age);
            info!("  职业：{:?}", response.data.profession);
            info!("  城市：{:?}", response.data.city);
            if let Some(usage) = response.usage {
                info!("\n  Token 使用统计:");
                info!("    输入 token: {}", usage.input_tokens);
                info!("    输出 token: {}", usage.output_tokens);
                info!("    总计 token: {}\n", usage.total_tokens);
            }
        }
        Err(e) => {
            info!("✗ 提取失败：{}\n", e);
        }
    }

    info!("═══════════════════════════════════════");
    info!("示例完成！");
    info!("═══════════════════════════════════════");

    Ok(())
}
