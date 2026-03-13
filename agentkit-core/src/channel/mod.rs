//! Channel（通信渠道）抽象模块。
//!
//! Channel 用于把 Agent 的输入/输出事件对接到外部系统：
//! - CLI
//! - HTTP/WebSocket
//! - IM/机器人平台
//! 在 core 层，我们只定义发送与订阅的接口。

pub mod r#trait;
pub mod types;

/// 重新导出 channel 相关 trait，方便 `agentkit_core::channel::*` 使用。
pub use r#trait::*;

/// 重新导出 channel 相关类型，方便使用。
pub use types::*;
