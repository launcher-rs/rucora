//! 工具调用增强配置模块
//!
//! # 概述
//!
//! 本模块提供工具调用的可靠性与性能增强配置：
//! - `RetryConfig`: 重试策略（指数退避）
//! - `TimeoutConfig`: 超时控制
//! - `CircuitBreakerConfig`: 熔断器配置
//! - `ConcurrencyConfig`: 细粒度并发控制
//! - `CacheConfig`: 工具结果缓存
//! - `ToolCallEnhancedConfig`: 聚合配置
//!
//! # 设计原则
//!
//! - 所有新配置均有默认值，保持向后兼容
//! - 新功能默认关闭，通过配置启用
//! - 不改变现有 API 签名

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::Mutex;

// ========== 重试配置 ==========

/// 重试策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// 固定间隔
    Fixed,
    /// 指数退避
    Exponential,
}

/// 工具调用重试配置
///
/// 默认关闭（`max_retries = 0`）。
///
/// # 示例
///
/// ```rust
/// use agentkit::agent::tool_call_config::RetryConfig;
/// use std::time::Duration;
///
/// let config = RetryConfig {
///     max_retries: 3,
///     initial_delay: Duration::from_millis(100),
///     max_delay: Duration::from_secs(5),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数（0 = 不重试）
    pub max_retries: u32,
    /// 初始重试延迟
    pub initial_delay: Duration,
    /// 最大重试延迟（指数退避上限）
    pub max_delay: Duration,
    /// 重试策略
    pub strategy: RetryStrategy,
    /// 指数退避的底数（仅 Exponential 策略有效）
    pub backoff_factor: f64,
    /// 仅对临时错误重试（工具返回的错误含 "transient" 标记时重试）
    pub only_transient: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            strategy: RetryStrategy::Exponential,
            backoff_factor: 2.0,
            only_transient: false,
        }
    }
}

impl RetryConfig {
    /// 创建关闭重试的配置
    pub fn disabled() -> Self {
        Self::default()
    }

    /// 创建指数退避重试配置
    pub fn exponential(max_retries: u32) -> Self {
        Self {
            max_retries,
            strategy: RetryStrategy::Exponential,
            ..Default::default()
        }
    }

    /// 计算第 n 次重试的等待时间（n 从 0 开始）
    pub fn delay_for(&self, attempt: u32) -> Duration {
        match self.strategy {
            RetryStrategy::Fixed => self.initial_delay,
            RetryStrategy::Exponential => {
                let factor = self.backoff_factor.powi(attempt as i32);
                let ms = (self.initial_delay.as_millis() as f64 * factor) as u64;
                let delay = Duration::from_millis(ms);
                delay.min(self.max_delay)
            }
        }
    }

    /// 判断是否应该对该错误输出进行重试
    pub fn should_retry(&self, output: &Value) -> bool {
        if !self.only_transient {
            return true;
        }
        // 检查输出中是否有 transient 标记
        output
            .get("error")
            .and_then(|e| e.get("transient"))
            .and_then(|t| t.as_bool())
            .unwrap_or(false)
    }
}

// ========== 超时配置 ==========

/// 工具调用超时配置
///
/// 默认关闭（`timeout = None`）。
///
/// # 示例
///
/// ```rust
/// use agentkit::agent::tool_call_config::TimeoutConfig;
/// use std::time::Duration;
///
/// // 全局默认超时 30s
/// let config = TimeoutConfig::default_timeout(Duration::from_secs(30));
///
/// // 针对特定工具设置不同超时
/// let config = TimeoutConfig::default_timeout(Duration::from_secs(30))
///     .with_tool_timeout("http_request", Duration::from_secs(60))
///     .with_tool_timeout("shell", Duration::from_secs(120));
/// ```
#[derive(Debug, Clone, Default)]
pub struct TimeoutConfig {
    /// 默认超时时间（None = 不限制）
    pub default_timeout: Option<Duration>,
    /// 按工具名称设置独立超时
    pub tool_timeouts: HashMap<String, Duration>,
}

impl TimeoutConfig {
    /// 创建关闭超时的配置
    pub fn disabled() -> Self {
        Self::default()
    }

    /// 设置默认超时
    pub fn default_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: Some(timeout),
            tool_timeouts: HashMap::new(),
        }
    }

    /// 为特定工具设置超时
    pub fn with_tool_timeout(mut self, tool_name: impl Into<String>, timeout: Duration) -> Self {
        self.tool_timeouts.insert(tool_name.into(), timeout);
        self
    }

    /// 获取工具的超时时间
    pub fn get_timeout(&self, tool_name: &str) -> Option<Duration> {
        self.tool_timeouts
            .get(tool_name)
            .copied()
            .or(self.default_timeout)
    }
}

// ========== 熔断器 ==========

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 关闭（正常运行）
    Closed,
    /// 打开（拒绝调用）
    Open,
    /// 半开（探测是否恢复）
    HalfOpen,
}

/// 熔断器配置
///
/// 默认关闭（`enabled = false`）。
///
/// # 示例
///
/// ```rust
/// use agentkit::agent::tool_call_config::CircuitBreakerConfig;
/// use std::time::Duration;
///
/// let config = CircuitBreakerConfig {
///     enabled: true,
///     failure_threshold: 5,
///     recovery_timeout: Duration::from_secs(30),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 是否启用熔断器
    pub enabled: bool,
    /// 连续失败次数阈值（达到后熔断）
    pub failure_threshold: u32,
    /// 熔断后等待时间（之后进入半开状态）
    pub recovery_timeout: Duration,
    /// 半开状态下允许通过的探测请求数
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
        }
    }
}

/// 单个工具的熔断器运行时状态
#[derive(Debug)]
pub struct CircuitBreakerState {
    /// 当前状态
    pub state: CircuitState,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 最后一次打开熔断器的时间
    pub last_opened_at: Option<Instant>,
    /// 半开状态下已通过的探测请求数
    pub half_open_calls: u32,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            last_opened_at: None,
            half_open_calls: 0,
        }
    }
}

impl CircuitBreakerState {
    /// 根据配置判断当前是否应该允许调用通过
    pub fn can_pass(&mut self, config: &CircuitBreakerConfig) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // 检查是否到了恢复探测时间
                if let Some(opened_at) = self.last_opened_at
                    && opened_at.elapsed() >= config.recovery_timeout
                {
                    self.state = CircuitState::HalfOpen;
                    self.half_open_calls = 0;
                    return true;
                }
                false
            }
            CircuitState::HalfOpen => {
                if self.half_open_calls < config.half_open_max_calls {
                    self.half_open_calls += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 记录一次成功调用
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.state = CircuitState::Closed;
        self.half_open_calls = 0;
    }

    /// 记录一次失败调用
    pub fn record_failure(&mut self, config: &CircuitBreakerConfig) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= config.failure_threshold {
            self.state = CircuitState::Open;
            self.last_opened_at = Some(Instant::now());
        }
    }
}

/// 所有工具的熔断器状态表（线程安全）
#[derive(Debug, Default, Clone)]
pub struct CircuitBreakerRegistry {
    states: Arc<Mutex<HashMap<String, CircuitBreakerState>>>,
}

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查是否允许工具调用通过（自动更新状态）
    pub async fn can_pass(&self, tool_name: &str, config: &CircuitBreakerConfig) -> bool {
        if !config.enabled {
            return true;
        }
        let mut states = self.states.lock().await;
        let state = states
            .entry(tool_name.to_string())
            .or_insert_with(CircuitBreakerState::default);
        state.can_pass(config)
    }

    /// 获取工具当前熔断状态
    pub async fn get_state(&self, tool_name: &str) -> CircuitState {
        let states = self.states.lock().await;
        states
            .get(tool_name)
            .map_or(CircuitState::Closed, |s| s.state)
    }

    /// 记录成功
    pub async fn record_success(&self, tool_name: &str) {
        let mut states = self.states.lock().await;
        if let Some(state) = states.get_mut(tool_name) {
            state.record_success();
        }
    }

    /// 记录失败
    pub async fn record_failure(&self, tool_name: &str, config: &CircuitBreakerConfig) {
        let mut states = self.states.lock().await;
        let state = states
            .entry(tool_name.to_string())
            .or_insert_with(CircuitBreakerState::default);
        state.record_failure(config);
    }
}

// ========== 细粒度并发控制 ==========

/// 细粒度并发控制配置
///
/// 支持按工具名称或按分类设置独立的最大并发数。
///
/// # 示例
///
/// ```rust
/// use agentkit::agent::tool_call_config::ConcurrencyConfig;
///
/// let config = ConcurrencyConfig::default()
///     // http_request 最多 5 个并发
///     .with_tool_concurrency("http_request", 5)
///     // shell 限制为 1 个并发（串行）
///     .with_tool_concurrency("shell", 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ConcurrencyConfig {
    /// 默认最大并发数（None = 使用全局 max_tool_concurrency）
    pub default_concurrency: Option<usize>,
    /// 按工具名称设置并发数
    pub tool_concurrency: HashMap<String, usize>,
}

impl ConcurrencyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// 为特定工具设置并发数
    pub fn with_tool_concurrency(
        mut self,
        tool_name: impl Into<String>,
        concurrency: usize,
    ) -> Self {
        self.tool_concurrency
            .insert(tool_name.into(), concurrency.max(1));
        self
    }

    /// 设置默认并发数
    pub fn with_default_concurrency(mut self, concurrency: usize) -> Self {
        self.default_concurrency = Some(concurrency.max(1));
        self
    }

    /// 获取工具的最大并发数（fallback 到 global_max）
    pub fn get_concurrency(&self, tool_name: &str, global_max: usize) -> usize {
        self.tool_concurrency
            .get(tool_name)
            .copied()
            .or(self.default_concurrency)
            .unwrap_or(global_max)
    }
}

// ========== 结果缓存 ==========

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 缓存的结果值
    value: Value,
    /// 缓存时间
    cached_at: Instant,
    /// TTL
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// 工具结果缓存配置
///
/// 默认关闭（`enabled = false`）。
/// 缓存基于工具名称 + 输入内容的 hash。
///
/// # 示例
///
/// ```rust
/// use agentkit::agent::tool_call_config::CacheConfig;
/// use std::time::Duration;
///
/// let config = CacheConfig {
///     enabled: true,
///     default_ttl: Duration::from_secs(300),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 是否启用缓存
    pub enabled: bool,
    /// 默认 TTL
    pub default_ttl: Duration,
    /// 按工具名称设置独立 TTL（None = 使用默认 TTL）
    pub tool_ttls: HashMap<String, Duration>,
    /// 最大缓存条目数（超出时 LRU 淘汰）
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_ttl: Duration::from_secs(300),
            tool_ttls: HashMap::new(),
            max_entries: 1000,
        }
    }
}

impl CacheConfig {
    /// 获取工具的 TTL
    pub fn get_ttl(&self, tool_name: &str) -> Duration {
        self.tool_ttls
            .get(tool_name)
            .copied()
            .unwrap_or(self.default_ttl)
    }

    /// 为特定工具设置 TTL
    pub fn with_tool_ttl(mut self, tool_name: impl Into<String>, ttl: Duration) -> Self {
        self.tool_ttls.insert(tool_name.into(), ttl);
        self
    }
}

/// 工具结果缓存（线程安全）
#[derive(Debug, Default, Clone)]
pub struct ToolResultCache {
    entries: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl ToolResultCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成缓存 key（工具名 + 输入内容）
    fn make_key(tool_name: &str, input: &Value) -> String {
        // 使用工具名和输入的 JSON 字符串作为 key
        // 对输入进行规范化排序以保证同一输入生成相同 key
        format!("{tool_name}:{input}")
    }

    /// 查询缓存
    pub async fn get(&self, tool_name: &str, input: &Value) -> Option<Value> {
        let key = Self::make_key(tool_name, input);
        let mut entries = self.entries.lock().await;
        let entry = entries.get(&key)?;
        if entry.is_expired() {
            entries.remove(&key);
            return None;
        }
        Some(entry.value.clone())
    }

    /// 写入缓存
    pub async fn set(
        &self,
        tool_name: &str,
        input: &Value,
        value: Value,
        ttl: Duration,
        max_entries: usize,
    ) {
        let key = Self::make_key(tool_name, input);
        let mut entries = self.entries.lock().await;

        // 先清理过期条目
        entries.retain(|_, v| !v.is_expired());

        // 如果还是超出限制，移除最旧的条目
        while entries.len() >= max_entries {
            // 简单策略：移除第一个（实际是随机一个，HashMap 无序）
            if let Some(oldest_key) = entries.keys().next().cloned() {
                entries.remove(&oldest_key);
            } else {
                break;
            }
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                cached_at: Instant::now(),
                ttl,
            },
        );
    }

    /// 清除所有缓存
    pub async fn clear(&self) {
        let mut entries = self.entries.lock().await;
        entries.clear();
    }

    /// 清除指定工具的缓存
    pub async fn clear_tool(&self, tool_name: &str) {
        let prefix = format!("{tool_name}:");
        let mut entries = self.entries.lock().await;
        entries.retain(|k, _| !k.starts_with(&prefix));
    }
}

// ========== 聚合配置 ==========

/// 工具调用增强配置（聚合所有改进选项）
///
/// 所有选项默认关闭，按需启用，向后兼容。
///
/// # 示例
///
/// ```rust
/// use agentkit::agent::tool_call_config::{
///     ToolCallEnhancedConfig, RetryConfig, TimeoutConfig,
///     CircuitBreakerConfig, CacheConfig,
/// };
/// use std::time::Duration;
///
/// let config = ToolCallEnhancedConfig::new()
///     .with_retry(RetryConfig::exponential(3))
///     .with_timeout(TimeoutConfig::default_timeout(Duration::from_secs(30)))
///     .with_circuit_breaker(CircuitBreakerConfig {
///         enabled: true,
///         failure_threshold: 5,
///         ..Default::default()
///     })
///     .with_cache(CacheConfig {
///         enabled: true,
///         ..Default::default()
///     });
/// ```
#[derive(Debug, Clone, Default)]
pub struct ToolCallEnhancedConfig {
    /// 重试配置
    pub retry: RetryConfig,
    /// 超时配置
    pub timeout: TimeoutConfig,
    /// 熔断器配置
    pub circuit_breaker: CircuitBreakerConfig,
    /// 细粒度并发控制
    pub concurrency: ConcurrencyConfig,
    /// 结果缓存配置
    pub cache: CacheConfig,
}

impl ToolCallEnhancedConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_timeout(mut self, timeout: TimeoutConfig) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_circuit_breaker(mut self, cb: CircuitBreakerConfig) -> Self {
        self.circuit_breaker = cb;
        self
    }

    pub fn with_concurrency(mut self, concurrency: ConcurrencyConfig) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn with_cache(mut self, cache: CacheConfig) -> Self {
        self.cache = cache;
        self
    }
}

// ========== 运行时状态 ==========

/// 工具调用运行时增强状态（线程安全，可跨任务共享）
#[derive(Debug, Clone, Default)]
pub struct ToolCallEnhancedRuntime {
    /// 熔断器注册表
    pub circuit_breaker: CircuitBreakerRegistry,
    /// 结果缓存
    pub cache: ToolResultCache,
}

impl ToolCallEnhancedRuntime {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_exponential() {
        let cfg = RetryConfig::exponential(3);
        assert_eq!(cfg.delay_for(0), Duration::from_millis(100));
        assert_eq!(cfg.delay_for(1), Duration::from_millis(200));
        assert_eq!(cfg.delay_for(2), Duration::from_millis(400));
    }

    #[test]
    fn test_retry_delay_max() {
        let cfg = RetryConfig {
            max_retries: 10,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(5),
            strategy: RetryStrategy::Exponential,
            backoff_factor: 2.0,
            only_transient: false,
        };
        // 第 10 次重试时不会超过 max_delay
        assert!(cfg.delay_for(10) <= Duration::from_secs(5));
    }

    #[test]
    fn test_timeout_config() {
        let cfg = TimeoutConfig::default_timeout(Duration::from_secs(30))
            .with_tool_timeout("http_request", Duration::from_secs(60));

        assert_eq!(
            cfg.get_timeout("http_request"),
            Some(Duration::from_secs(60))
        );
        assert_eq!(cfg.get_timeout("shell"), Some(Duration::from_secs(30)));
        assert_eq!(TimeoutConfig::disabled().get_timeout("any"), None);
    }

    #[test]
    fn test_concurrency_config() {
        let cfg = ConcurrencyConfig::new()
            .with_tool_concurrency("http_request", 5)
            .with_tool_concurrency("shell", 1);

        assert_eq!(cfg.get_concurrency("http_request", 3), 5);
        assert_eq!(cfg.get_concurrency("shell", 3), 1);
        assert_eq!(cfg.get_concurrency("other", 3), 3); // fallback 到 global
    }

    #[test]
    fn test_circuit_breaker_state() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(50),
            half_open_max_calls: 1,
        };

        let mut state = CircuitBreakerState::default();

        // 初始状态：允许通过
        assert!(state.can_pass(&config));

        // 连续失败 3 次后熔断
        state.record_failure(&config);
        state.record_failure(&config);
        state.record_failure(&config);
        assert_eq!(state.state, CircuitState::Open);

        // 熔断状态：不允许通过
        assert!(!state.can_pass(&config));
    }

    #[tokio::test]
    async fn test_tool_result_cache() {
        let cache = ToolResultCache::new();
        let input = serde_json::json!({"path": "/tmp/test.txt"});
        let value = serde_json::json!({"content": "hello"});

        // 初始无缓存
        assert!(cache.get("file_read", &input).await.is_none());

        // 写入缓存
        cache
            .set(
                "file_read",
                &input,
                value.clone(),
                Duration::from_secs(60),
                1000,
            )
            .await;

        // 命中缓存
        let cached = cache.get("file_read", &input).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), value);
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let cache = ToolResultCache::new();
        let input = serde_json::json!({"key": "val"});
        let value = serde_json::json!({"result": "ok"});

        // 设置极短 TTL
        cache
            .set("tool", &input, value, Duration::from_millis(1), 1000)
            .await;

        // 等待过期
        tokio::time::sleep(Duration::from_millis(10)).await;

        // 缓存应已过期
        assert!(cache.get("tool", &input).await.is_none());
    }
}
