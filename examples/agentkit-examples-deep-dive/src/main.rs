//! AgentKit 深入研究示例
//!
//! 本示例深入展示 AgentKit 特定功能的高级用法：
//! - 自定义 Provider 实现
//! - 自定义 Tool 实现
//! - 自定义 Runtime 实现
//! - 高级错误处理
//! - 性能优化技巧
//!
//! # 运行方式
//!
//! ```bash
//! # 运行所有示例
//! cargo run -p agentkit-examples-deep-dive
//!
//! # 运行特定示例
//! cargo run -p agentkit-examples-deep-dive -- custom_tool
//! cargo run -p agentkit-examples-deep-dive -- custom_provider
//! cargo run -p agentkit-examples-deep-dive -- custom_runtime
//! cargo run -p agentkit-examples-deep-dive -- error_handling
//! cargo run -p agentkit-examples-deep-dive -- performance
//! ```

mod custom_provider;
mod custom_runtime;
mod custom_tool;
mod error_handling;
mod performance;

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// 示例类型枚举
#[derive(Debug, Clone)]
enum Example {
    All,
    CustomProvider,
    CustomTool,
    CustomRuntime,
    ErrorHandling,
    Performance,
}

impl Example {
    fn from_str(s: &str) -> Self {
        match s {
            "custom_provider" => Example::CustomProvider,
            "custom_tool" => Example::CustomTool,
            "custom_runtime" => Example::CustomRuntime,
            "error_handling" => Example::ErrorHandling,
            "performance" => Example::Performance,
            _ => Example::All,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    // 解析命令行参数
    let example = std::env::args()
        .nth(1)
        .map(|s| Example::from_str(&s))
        .unwrap_or(Example::All);

    info!("=== AgentKit 深入研究示例 ===");
    info!("运行示例：{:?}", example);

    match example {
        Example::All => {
            run_all_examples().await?;
        }
        Example::CustomProvider => {
            custom_provider::run().await?;
        }
        Example::CustomTool => {
            custom_tool::run().await?;
        }
        Example::CustomRuntime => {
            custom_runtime::run().await?;
        }
        Example::ErrorHandling => {
            error_handling::run().await?;
        }
        Example::Performance => {
            performance::run().await?;
        }
    }

    info!("\n=== 示例运行完成 ===");

    Ok(())
}

/// 运行所有示例
async fn run_all_examples() -> Result<()> {
    info!("\n=== 运行所有深入研究示例 ===\n");

    // 1. 自定义 Provider
    info!("\n--- 1. 自定义 Provider ---");
    custom_provider::run().await?;

    // 2. 自定义 Tool
    info!("\n--- 2. 自定义 Tool ---");
    custom_tool::run().await?;

    // 3. 自定义 Runtime
    info!("\n--- 3. 自定义 Runtime ---");
    custom_runtime::run().await?;

    // 4. 错误处理
    info!("\n--- 4. 错误处理 ---");
    error_handling::run().await?;

    // 5. 性能优化
    info!("\n--- 5. 性能优化 ---");
    performance::run().await?;

    Ok(())
}
