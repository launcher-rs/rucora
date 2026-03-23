//! A2A Server 示例 - 使用 ra2a 库实现 A2A 协议服务器
//!
//! 此服务器提供一个简单的 Agent，可以：
//! - 回答当前时间
//! - 回应用户消息
//!
//! 运行方式：
//! ```bash
//! cargo run --example a2a-server
//! ```

use chrono::Local;
use ra2a::{
    error::Result,
    server::{AgentExecutor, Event, EventQueue, RequestContext, ServerState, a2a_router},
    types::{AgentCard, Message, Part, Task, TaskState, TaskStatus},
};
use std::{future::Future, pin::Pin};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

/// 时间 Agent - 可以回答当前时间
struct TimeAgent;

impl TimeAgent {
    /// 获取当前时间的字符串表示
    fn get_current_time() -> String {
        Local::now().format("%Y 年%m 月%d 日 %H:%M:%S").to_string()
    }

    /// 处理用户消息，判断是否需要返回时间
    fn process_message(input: &str) -> String {
        let lower = input.to_lowercase();

        // 检查是否询问时间
        if lower.contains("时间")
            || lower.contains("几点")
            || lower.contains("几点了")
            || lower.contains("time")
            || lower.contains("clock")
        {
            format!("现在的时间是：{}", Self::get_current_time())
        } else {
            // 普通回复
            format!(
                "收到您的消息：{}\n\n我是一个简单的时间助手，可以告诉您当前时间。\n请问：现在几点了？",
                input
            )
        }
    }
}

impl AgentExecutor for TimeAgent {
    /// 执行任务
    fn execute<'a>(
        &'a self,
        ctx: &'a RequestContext,
        queue: &'a EventQueue,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // 提取输入消息
            let input = ctx
                .message
                .as_ref()
                .and_then(ra2a::Message::text_content)
                .unwrap_or_default();

            info!("收到消息：{}", input);

            // 处理消息
            let response = Self::process_message(&input);

            info!("回复：{}", response);

            // 创建完成的任务
            let mut task = Task::new(&ctx.task_id, &ctx.context_id);
            task.status = TaskStatus::with_message(
                TaskState::Completed,
                Message::agent(vec![Part::text(response)]),
            );

            // 发送事件
            queue.send(Event::Task(task))?;
            Ok(())
        })
    }

    /// 取消任务
    fn cancel<'a>(
        &'a self,
        ctx: &'a RequestContext,
        queue: &'a EventQueue,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            info!("取消任务：{}", ctx.task_id);

            let mut task = Task::new(&ctx.task_id, &ctx.context_id);
            task.status = TaskStatus::new(TaskState::Canceled);
            queue.send(Event::Task(task))?;
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         AgentKit A2A Server - 时间助手                 ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 创建 AgentCard
    let card = AgentCard::new("时间助手 Agent", "http://localhost:8080");

    // 创建 ServerState
    let state = ServerState::from_executor(TimeAgent, card);

    // 构建 Axum 路由
    let app = axum::Router::new().merge(a2a_router(state));

    // 启动服务器
    let addr = "0.0.0.0:8080";
    info!("A2A Server 启动在：{}", addr);
    println!("✓ A2A Server 已启动");
    println!("  地址：http://{}", addr);
    println!("  AgentCard: http://{}/.well-known/agent.json", addr);
    println!("\n等待客户端连接...\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
