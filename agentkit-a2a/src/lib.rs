//! A2A（Agent-to-Agent）协议支持。
//!
//! 本 crate 是对第三方库 `ra2a`（A2A 协议 Rust SDK）的薄封装：
//! - 主要职责是统一在 `agentkit` workspace 中暴露 A2A 能力
//! - 尽量不在此处重复维护协议结构或传输层实现
//!
//! 使用方式：
//! - Client：`agentkit::a2a::client::Client`
//! - 类型：`agentkit::a2a::types::*`
//! - Server：由使用方自行选择 Web 框架并集成 `agentkit::a2a::server::*`
//!
pub use ra2a::*;

/// A2A 客户端相关 API（来自 `ra2a::client`）。
pub mod client {
    pub use ra2a::client::*;
}

/// A2A 服务端相关 API（来自 `ra2a::server`）。
///
/// 注意：服务端通常会依赖具体 Web 框架（例如 `axum`）。本 crate 仅转导出
/// `ra2a` 的 server 侧能力，具体监听端口、路由挂载、中间件等由使用方自行完成。
pub mod server {
    pub use ra2a::server::*;
}

/// A2A 协议核心数据结构（来自 `ra2a::types`），例如 `Message`、`Task` 等。
pub mod types {
    pub use ra2a::types::*;
}
