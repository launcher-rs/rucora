//! AgentKit 中间件示例
//!
//! 展示中间件系统的概念和使用方法。
//!
//! ## 运行方法
//! ```bash
//! cargo run --example 08_middleware
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
    info!("║   AgentKit 中间件示例                 ║");
    info!("╚════════════════════════════════════════╝\n");

    info!("中间件系统允许你在请求处理过程中插入自定义逻辑。\n");

    info!("═══════════════════════════════════════");
    info!("常见的中间件类型:");
    info!("═══════════════════════════════════════");
    info!("1. 日志中间件 - 记录请求和响应");
    info!("2. 指标收集中间件 - 收集性能指标");
    info!("3. 限流中间件 - 限制请求频率");
    info!("4. 认证中间件 - 验证用户身份");
    info!("5. 缓存中间件 - 缓存响应结果");
    info!("6. 重试中间件 - 自动重试失败请求");
    info!("═══════════════════════════════════════\n");

    info!("中间件的使用场景:");
    info!("• 横切关注点 - 日志、监控、认证等");
    info!("• 请求预处理 - 参数验证、数据转换");
    info!("• 响应后处理 - 格式化、压缩");
    info!("• 错误处理 - 统一错误格式\n");

    info!("示例完成！");
    info!("提示：中间件系统正在开发中，敬请期待");

    Ok(())
}
