//! AgentKit HTTP Server
//!
//! # 概述
//!
//! 本服务器提供 AgentKit 的 HTTP API 接口，支持：
//! - SSE（Server-Sent Events）流式输出
//! - CORS 跨域支持
//! - 健康检查端点
//!
//! # API 端点
//!
//! | 端点 | 方法 | 说明 |
//! |------|------|------|
//! | `/health` | GET | 健康检查 |
//! | `/v1/chat/stream` | POST | 流式聊天（SSE） |
//!
//! # 启动方式
//!
//! ## 使用默认配置
//!
//! ```bash
//! cargo run -p agentkit-server
//! ```
//!
//! 默认监听 `127.0.0.1:8080`
//!
//! ## 自定义配置
//!
//! ```bash
//! # 设置监听地址
//! export AGENTKIT_SERVER_ADDR=0.0.0.0:3000
//!
//! # 设置 Skills 目录
//! export AGENTKIT_SKILL_DIR=skills
//!
//! # 设置配置文件
//! export AGENTKIT_CONFIG=config.yaml
//!
//! # 启动服务器
//! cargo run -p agentkit-server
//! ```
//!
//! # 使用示例
//!
//! ## 健康检查
//!
//! ```bash
//! curl http://127.0.0.1:8080/health
//! ```
//!
//! 响应：
//! ```json
//! {"ok": true}
//! ```
//!
//! ## 流式聊天
//!
//! ```bash
//! curl -X POST http://127.0.0.1:8080/v1/chat/stream \
//!   -H "Content-Type: application/json" \
//!   -d '{"messages": [{"role": "user", "content": "你好"}]}'
//! ```
//!
//! SSE 响应格式：
//! ```text
//! event: event
//! data: {"type":"TokenDelta","payload":{"delta":"你"}}
//!
//! event: event
//! data: {"type":"TokenDelta","payload":{"delta":"好"}}
//!
//! event: event
//! data: {"type":"Message","payload":{...}}
//! ```
//!
//! # 环境变量
//!
//! | 变量名 | 说明 | 默认值 |
//! |--------|------|--------|
//! | `AGENTKIT_SERVER_ADDR` | 服务器监听地址 | `127.0.0.1:8080` |
//! | `AGENTKIT_SKILL_DIR` | Skills 目录 | `skills` |
//! | `AGENTKIT_CONFIG` | 配置文件路径 | - |
//! | `AGENTKIT_PROFILE` | Profile 名称 | `default` |
//!
//! # 错误处理
//!
//! 服务器启动失败时会：
//! 1. 打印错误信息到 stderr
//! 2. 返回非零退出码
//!
//! 运行时错误会通过 SSE 错误事件返回：
//! ```text
//! event: error
//! data: {"type":"Error","payload":{"kind":"runtime","message":"..."}}
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use agentkit::config::AgentkitConfig;
use agentkit_core::agent::types::AgentInput;
use agentkit_core::provider::types::ChatMessage;
use agentkit_runtime::{ChannelEvent, DefaultRuntime, ToolRegistry};
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

/// 应用状态
#[derive(Clone)]
struct AppState {
    /// Agent 运行时
    agent: Arc<DefaultRuntime>,
}

/// 流式聊天请求
#[derive(Deserialize)]
struct ChatStreamRequest {
    /// 消息列表
    messages: Vec<ChatMessage>,
    /// 元数据（可选，保留以兼容）
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

/// 健康检查处理器
///
/// 返回简单的 JSON 响应表示服务器正常运行。
async fn health() -> Json<serde_json::Value> {
    Json(json!({"ok": true}))
}

/// 流式聊天处理器
///
/// 接收聊天请求，通过 Agent 运行时处理，并以 SSE 格式返回事件流。
///
/// # 事件格式
///
/// - `event`: 事件类型（event/error）
/// - `data`: JSON 格式的事件数据
///
/// # 错误处理
///
/// 运行时错误会被转换为错误事件返回：
/// ```json
/// {"type":"Error","payload":{"kind":"runtime","message":"..."}}
/// ```
async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatStreamRequest>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, axum::Error>>> {
    // 将消息转换为文本输入
    let text = req
        .messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let input = AgentInput::new(text);

    let s = state.agent.run_stream(input).map(|item| match item {
        Ok(ev) => {
            let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
            Ok::<Event, axum::Error>(Event::default().event("event").data(data))
        }
        Err(e) => {
            let err = ChannelEvent::Error(agentkit_runtime::ErrorEvent {
                kind: "runtime".to_string(),
                message: e.to_string(),
                data: None,
            });
            let data = serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string());
            Ok::<Event, axum::Error>(Event::default().event("error").data(data))
        }
    });

    Sse::new(s).keep_alive(KeepAlive::default())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 加载 Skills 目录
    let skill_dir = std::env::var("AGENTKIT_SKILL_DIR").unwrap_or_else(|_| "skills".to_string());
    let skill_dir = PathBuf::from(skill_dir);

    // 加载配置
    let profile = match AgentkitConfig::load().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("错误：加载配置失败 - {}", e);
            std::process::exit(1);
        }
    };

    // 构建 provider
    let provider = match AgentkitConfig::build_provider(&profile) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("错误：构建 provider 失败 - {}", e);
            std::process::exit(1);
        }
    };

    // 加载 skills
    let skills = match agentkit::skills::load_skills_from_dir(&skill_dir).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("错误：加载 skills 失败 - {}", e);
            std::process::exit(1);
        }
    };

    // 构建工具注册表
    let mut tools = ToolRegistry::new();
    for tool in skills.as_tools() {
        tools = tools.register_arc(tool);
    }
    tools = tools.register(agentkit::tools::CmdExecTool::new());

    // 创建 Agent 运行时
    let agent = DefaultRuntime::new(Arc::new(provider), tools);

    let state = AppState {
        agent: Arc::new(agent),
    };

    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 构建路由
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/chat/stream", post(chat_stream))
        .layer(cors)
        .with_state(state);

    // 解析监听地址
    let addr_str =
        std::env::var("AGENTKIT_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("错误：无效的 AGENTKIT_SERVER_ADDR '{}' - {}", addr_str, e);
            std::process::exit(1);
        }
    };

    // 绑定地址
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("错误：绑定地址失败 {:?} - {}", addr, e);
            std::process::exit(1);
        }
    };

    eprintln!("AgentKit Server 运行在 http://{}", addr);
    eprintln!("API 端点:");
    eprintln!("  - GET  http://{}/health", addr);
    eprintln!("  - POST http://{}/v1/chat/stream", addr);

    // 启动服务器
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("错误：服务器运行失败 - {}", e);
        std::process::exit(1);
    }

    Ok(())
}
