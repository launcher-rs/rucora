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

#[derive(Clone)]
struct AppState {
    agent: Arc<DefaultRuntime>,
}

#[derive(Deserialize)]
struct ChatStreamRequest {
    messages: Vec<ChatMessage>,
    metadata: Option<serde_json::Value>,
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"ok": true}))
}

async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatStreamRequest>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, axum::Error>>> {
    let input = AgentInput {
        messages: req.messages,
        metadata: req.metadata,
    };

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
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let skill_dir = std::env::var("AGENTKIT_SKILL_DIR").unwrap_or_else(|_| "skills".to_string());
    let skill_dir = PathBuf::from(skill_dir);

    let profile = AgentkitConfig::load().await.expect("load config failed");
    let provider = AgentkitConfig::build_provider(&profile).expect("build provider failed");

    let skills = agentkit::skills::load_skills_from_dir(&skill_dir)
        .await
        .expect("load skills failed");

    let mut tools = ToolRegistry::new();
    for tool in skills.as_tools() {
        tools = tools.register_arc(tool);
    }
    tools = tools.register(agentkit::tools::CmdExecTool::new());

    let agent = DefaultRuntime::new(Arc::new(provider), tools);

    let state = AppState {
        agent: Arc::new(agent),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/chat/stream", post(chat_stream))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = std::env::var("AGENTKIT_SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .expect("invalid AGENTKIT_SERVER_ADDR");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind failed");

    axum::serve(listener, app).await.expect("serve failed");
}
