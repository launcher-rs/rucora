//! 可中断信号 —— 用于工具执行、流式响应的取消控制

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// 可中断信号。可安全跨线程共享，用于通知异步任务提前退出。
///
/// # 基本用法
///
/// ```rust
/// use rucora_core::interrupt::InterruptSignal;
///
/// let signal = InterruptSignal::new();
/// let handle = signal.handle();
///
/// // 在另一个线程/任务中：
/// handle.interrupt();
///
/// // 在工作任务中：
/// assert!(signal.interrupted());
/// ```
///
/// # 与 tokio::select! 配合
///
/// ```rust,no_run
/// use rucora_core::interrupt::InterruptSignal;
/// use tokio::time::{sleep, Duration};
///
/// # async fn example() {
/// let signal = InterruptSignal::new();
/// let handle = signal.handle();
///
/// tokio::spawn(async move {
///     sleep(Duration::from_secs(1)).await;
///     handle.interrupt();
/// });
///
/// tokio::select! {
///     _ = signal.wait_for_interrupt() => { /* 被中断 */ }
///     _ = sleep(Duration::from_secs(10)) => { /* 正常完成 */ }
/// };
/// # }
/// ```
#[derive(Debug)]
pub struct InterruptSignal {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    flag: AtomicBool,
    notify: Notify,
}

impl InterruptSignal {
    /// 创建新的中断信号。
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                flag: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    /// 返回一个可用于触发的句柄。
    ///
    /// 持有句柄的代码可以在不持有 `InterruptSignal` 本身的情况下触发中断。
    pub fn handle(&self) -> InterruptHandle {
        InterruptHandle {
            inner: self.inner.clone(),
        }
    }

    /// 是否已被中断。
    pub fn interrupted(&self) -> bool {
        self.inner.flag.load(Ordering::Acquire)
    }

    /// 等待中断信号。
    pub async fn wait_for_interrupt(&self) {
        if self.interrupted() {
            return;
        }
        self.inner.notify.notified().await;
    }

    /// 重置中断状态（允许复用信号）。
    pub fn reset(&self) {
        self.inner.flag.store(false, Ordering::Release);
        // 不重置 Notify，因为 Notify 的一次性语义不影响后续使用
    }
}

impl Default for InterruptSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InterruptSignal {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// 中断触发句柄。
///
/// 持有此句柄的代码可触发中断，但不检查状态。
#[derive(Debug, Clone)]
pub struct InterruptHandle {
    inner: Arc<Inner>,
}

impl InterruptHandle {
    /// 触发中断。
    pub fn interrupt(&self) {
        self.inner.flag.store(true, Ordering::Release);
        self.inner.notify.notify_waiters();
    }

    /// 是否已被中断。
    pub fn interrupted(&self) -> bool {
        self.inner.flag.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, timeout, Duration};

    #[test]
    fn test_not_interrupted_by_default() {
        let signal = InterruptSignal::new();
        assert!(!signal.interrupted());
    }

    #[test]
    fn test_interrupt_via_handle() {
        let signal = InterruptSignal::new();
        let handle = signal.handle();
        assert!(!signal.interrupted());

        handle.interrupt();
        assert!(signal.interrupted());
    }

    #[test]
    fn test_interrupt_via_signal() {
        let signal = InterruptSignal::new();
        let handle = signal.handle();
        handle.interrupt();
        assert!(signal.interrupted());
    }

    #[test]
    fn test_reset() {
        let signal = InterruptSignal::new();
        signal.handle().interrupt();
        assert!(signal.interrupted());

        signal.reset();
        assert!(!signal.interrupted());
    }

    #[test]
    fn test_clone() {
        let signal = InterruptSignal::new();
        let cloned = signal.clone();
        signal.handle().interrupt();
        assert!(cloned.interrupted());
    }

    #[test]
    fn test_multiple_handles() {
        let signal = InterruptSignal::new();
        let h1 = signal.handle();
        let h2 = signal.handle();

        h1.interrupt();
        assert!(h2.interrupted());
    }

    #[tokio::test]
    async fn test_wait_for_interrupt() {
        let signal = InterruptSignal::new();
        let handle = signal.handle();

        tokio::spawn(async move {
            sleep(std::time::Duration::from_millis(10)).await;
            handle.interrupt();
        });

        // 应该在被中断时返回，而不是超时
        timeout(Duration::from_secs(5), signal.wait_for_interrupt())
            .await
            .expect("应该被中断，而不是超时");
    }

    #[tokio::test]
    async fn test_wait_for_interrupt_already_interrupted() {
        let signal = InterruptSignal::new();
        signal.handle().interrupt();

        // 如果已中断，wait_for_interrupt 应立即返回
        timeout(Duration::from_secs(1), signal.wait_for_interrupt())
            .await
            .expect("已中断时应立即返回");
    }
}
