//! RetryPolicy（重试策略）接口
//!
//! 提供通用的重试逻辑抽象，支持指数退避、抖动等策略。
//!
//! # 设计原则
//!
//! - **可组合**: 支持装饰器模式增强策略
//! - **可观测**: 提供详细的重试日志
//! - **灵活**: 支持自定义延迟计算和终止条件
//!
//! # 使用示例
//!
//! ## 使用内置指数退避策略
//!
//! ```rust
//! use rucora_core::retry::{RetryPolicy, ExponentialBackoff};
//!
//! let policy = ExponentialBackoff::new(3, std::time::Duration::from_millis(100));
//! for attempt in 0..5 {
//!     if let Some(delay) = policy.should_retry(attempt) {
//!         println!("重试 {attempt}，等待 {delay:?}");
//!     } else {
//!         println!("不应重试");
//!         break;
//!     }
//! }
//! ```
//!
//! ## 实现自定义策略
//!
//! ```rust
//! use rucora_core::retry::{RetryPolicy, RetryAction};
//! use std::time::Duration;
//!
//! struct MyPolicy;
//!
//! impl RetryPolicy for MyPolicy {
//!     fn should_retry(&self, attempt: u32) -> Option<Duration> {
//!         if attempt < 3 {
//!             Some(Duration::from_millis(100 * 2u64.pow(attempt)))
//!         } else {
//!             None
//!         }
//!     }
//! }
//! ```

use std::time::Duration;

/// 重试动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAction {
    /// 应该重试
    ShouldRetry(Duration),
    /// 不应该重试
    NoRetry,
    /// 永久失败
    PermanentFailure,
}

/// 重试策略 trait
///
/// 所有重试策略必须实现此 trait。
pub trait RetryPolicy: Send + Sync {
    /// 判断是否应该重试
    ///
    /// # 参数
    ///
    /// * `attempt`: 当前重试次数（从 0 开始）
    ///
    /// # 返回
    ///
    /// - `Some(Duration)`: 应该重试，返回等待时间
    /// - `None`: 不应该重试
    fn should_retry(&self, attempt: u32) -> Option<Duration>;

    /// 判断是否应该重试（带错误信息）
    ///
    /// 默认实现调用 `should_retry`，忽略错误信息。
    /// 子类可以重写此方法根据错误类型决定是否重试。
    fn should_retry_with_error(&self, attempt: u32, _error: &str) -> Option<Duration> {
        self.should_retry(attempt)
    }

    /// 获取最大重试次数
    fn max_retries(&self) -> u32 {
        u32::MAX
    }
}

/// 指数退避策略
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

impl ExponentialBackoff {
    /// 创建新的指数退避策略
    ///
    /// # 参数
    ///
    /// * `max_retries`: 最大重试次数
    /// * `initial_delay`: 初始延迟
    pub fn new(max_retries: u32, initial_delay: Duration) -> Self {
        Self {
            max_retries,
            initial_delay,
            max_delay: Duration::from_secs(30),
            jitter: false,
        }
    }

    /// 设置最大延迟
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// 启用抖动（Jitter）
    ///
    /// 抖动可以避免多客户端同时重试造成的雷鸣羊群效应（Thundering Herd）。
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }

    fn calculate_delay(&self, attempt: u32) -> f64 {
        let base_delay_ms = self.initial_delay.as_millis() as f64;
        let max_delay_ms = self.max_delay.as_millis() as f64;
        let delay = base_delay_ms * 2f64.powi(attempt as i32);

        let delay = if delay > max_delay_ms {
            max_delay_ms
        } else {
            delay
        };

        if self.jitter {
            use std::time::Instant;
            let now = Instant::now();
            let nanos = now.elapsed().as_nanos() as f64;
            let jitter_range = delay * 0.2;
            let jitter = nanos % jitter_range;
            delay - jitter_range / 2.0 + jitter
        } else {
            delay
        }
    }
}

impl RetryPolicy for ExponentialBackoff {
    fn should_retry(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_retries {
            return None;
        }
        let delay_ms = self.calculate_delay(attempt);
        Some(Duration::from_millis(delay_ms as u64))
    }

    fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

/// 固定间隔策略
#[derive(Debug, Clone)]
pub struct FixedDelay {
    max_retries: u32,
    delay: Duration,
}

impl FixedDelay {
    /// 创建新的固定间隔策略
    pub fn new(max_retries: u32, delay: Duration) -> Self {
        Self { max_retries, delay }
    }
}

impl RetryPolicy for FixedDelay {
    fn should_retry(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_retries {
            return None;
        }
        Some(self.delay)
    }

    fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

/// 空策略（不重试）
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRetry;

impl RetryPolicy for NoRetry {
    fn should_retry(&self, _attempt: u32) -> Option<Duration> {
        None
    }

    fn max_retries(&self) -> u32 {
        0
    }
}

/// 装饰器：添加临时错误过滤
///
/// 只有满足条件的错误才会触发重试。
pub struct TransientFilter<P> {
    inner: P,
    predicate: Box<dyn Fn(&str) -> bool + Send + Sync>,
}

impl<P: RetryPolicy> TransientFilter<P> {
    /// 创建新的临时错误过滤器
    pub fn new(policy: P, predicate: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        Self {
            inner: policy,
            predicate: Box::new(predicate),
        }
    }
}

impl<P: RetryPolicy> RetryPolicy for TransientFilter<P> {
    fn should_retry(&self, attempt: u32) -> Option<Duration> {
        self.inner.should_retry(attempt)
    }

    fn should_retry_with_error(&self, attempt: u32, error: &str) -> Option<Duration> {
        if (self.predicate)(error) {
            self.inner.should_retry_with_error(attempt, error)
        } else {
            None
        }
    }

    fn max_retries(&self) -> u32 {
        self.inner.max_retries()
    }
}

/// RetryPolicy 扩展方法
pub trait RetryPolicyExt: RetryPolicy + Sized {
    /// 获取重试延迟的迭代器
    fn delays(&self) -> DelayIterator<'_, Self> {
        DelayIterator {
            policy: self,
            attempt: 0,
        }
    }
}

impl<T: RetryPolicy + Sized> RetryPolicyExt for T {}

/// 重试延迟迭代器
#[derive(Debug)]
pub struct DelayIterator<'a, P: RetryPolicy> {
    policy: &'a P,
    attempt: u32,
}

impl<P: RetryPolicy> Iterator for DelayIterator<'_, P> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let delay = self.policy.should_retry(self.attempt);
        self.attempt += 1;
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = ExponentialBackoff::new(3, Duration::from_millis(100));

        assert_eq!(policy.should_retry(0), Some(Duration::from_millis(100)));
        assert_eq!(policy.should_retry(1), Some(Duration::from_millis(200)));
        assert_eq!(policy.should_retry(2), Some(Duration::from_millis(400)));
        assert_eq!(policy.should_retry(3), None);
    }

    #[test]
    fn test_exponential_backoff_with_max_delay() {
        let policy = ExponentialBackoff::new(10, Duration::from_millis(100))
            .with_max_delay(Duration::from_millis(500));

        assert_eq!(policy.should_retry(0), Some(Duration::from_millis(100)));
        assert_eq!(policy.should_retry(1), Some(Duration::from_millis(200)));
        assert_eq!(policy.should_retry(2), Some(Duration::from_millis(400)));
        assert_eq!(policy.should_retry(3), Some(Duration::from_millis(500)));
        assert_eq!(policy.should_retry(4), Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_fixed_delay() {
        let policy = FixedDelay::new(3, Duration::from_secs(1));

        assert_eq!(policy.should_retry(0), Some(Duration::from_secs(1)));
        assert_eq!(policy.should_retry(1), Some(Duration::from_secs(1)));
        assert_eq!(policy.should_retry(2), Some(Duration::from_secs(1)));
        assert_eq!(policy.should_retry(3), None);
    }

    #[test]
    fn test_no_retry() {
        let policy = NoRetry;

        assert_eq!(policy.should_retry(0), None);
        assert_eq!(policy.should_retry(1), None);
    }

    #[test]
    fn test_delay_iterator() {
        let policy = ExponentialBackoff::new(3, Duration::from_millis(100));

        let delays: Vec<_> = policy.delays().collect();
        assert_eq!(delays.len(), 3);
        assert_eq!(delays[0], Duration::from_millis(100));
        assert_eq!(delays[1], Duration::from_millis(200));
        assert_eq!(delays[2], Duration::from_millis(400));
    }
}
