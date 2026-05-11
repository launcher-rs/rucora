//! Graceful Shutdown（优雅关闭）支持
//!
//! 提供优雅关闭机制，确保资源正确释放和任务有序终止。
//!
//! # 核心组件
//!
//! - `ShutdownHandle`: 关闭句柄，用于触发和控制关闭流程
//! - `ShutdownToken`: 关闭令牌，用于检查是否已收到关闭信号
//! - `GracefulShutdown`: 优雅关闭 trait，提供统一的关闭接口

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Notify};

/// 关闭令牌
///
/// 用于检查关闭信号是否已被触发。
#[derive(Debug, Clone)]
pub struct ShutdownToken {
    shutdown_tx: Arc<broadcast::Sender<()>>,
    is_shutdown: Arc<AtomicBool>,
}

impl ShutdownToken {
    /// 检查是否已收到关闭信号。
    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::SeqCst)
    }

    /// 订阅关闭信号。
    ///
    /// 返回一个接收器，当触发关闭时会收到通知。
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}

/// 优雅关闭句柄
///
/// 用于触发关闭信号并等待所有任务完成。
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    shutdown_tx: Arc<broadcast::Sender<()>>,
    notify: Arc<Notify>,
    is_shutdown: Arc<AtomicBool>,
}

impl GracefulShutdown for ShutdownHandle {
    fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(());
        self.notify.notify_waiters();
    }
}

impl ShutdownHandle {
    /// 创建新的关闭句柄。
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            shutdown_tx: Arc::new(shutdown_tx),
            notify: Arc::new(Notify::new()),
            is_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 获取关闭令牌。
    pub fn token(&self) -> ShutdownToken {
        ShutdownToken {
            shutdown_tx: Arc::clone(&self.shutdown_tx),
            is_shutdown: Arc::clone(&self.is_shutdown),
        }
    }

    /// 获取关闭通知器。
    pub fn notify(&self) -> Arc<Notify> {
        Arc::clone(&self.notify)
    }
}

impl Default for ShutdownHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// 优雅关闭 trait
///
/// 实现此 trait 的类型可以接收关闭信号并优雅地停止工作。
pub trait GracefulShutdown: Send + Sync {
    /// 触发关闭信号。
    fn shutdown(&self);

    /// 检查是否已关闭。
    fn is_shutdown(&self) -> bool {
        false
    }
}

/// 运行时关闭状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownState {
    /// 运行中
    Running,
    /// 正在关闭
    ShuttingDown,
    /// 已关闭
    Shutdown,
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_token() {
        let handle = ShutdownHandle::new();
        
        let token = handle.token();
        assert!(!token.is_shutdown(), "should not be shutdown before calling shutdown()");
        
        handle.shutdown();
        
        let token2 = handle.token();
        assert!(token2.is_shutdown(), "should be shutdown after calling shutdown()");
        
        let token3 = handle.token();
        assert!(token3.is_shutdown(), "should remain shutdown");
    }

    #[tokio::test]
    async fn test_multiple_tokens() {
        let handle = ShutdownHandle::new();
        
        let token1 = handle.token();
        let token2 = handle.token();
        
        assert!(!token1.is_shutdown());
        assert!(!token2.is_shutdown());
        
        handle.shutdown();
        
        assert!(token1.is_shutdown());
        assert!(token2.is_shutdown());
    }
}
