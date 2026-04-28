//! Dual-track Metrics（双轨指标系统）
//!
//! 本模块提供分离的事件（ObserverEvent）与指标（ObserverMetric）系统：
//! - ObserverEvent: 结构化事件，用于日志、追踪、审计
//! - ObserverMetric: 数值指标，用于监控、告警、统计
//!
//! 参考实现: zeroclaw `observability_traits.rs`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::provider::types::Usage;

/// 观测器事件（结构化事件）
///
/// 用于记录 Agent 执行过程中的关键事件，支持日志、追踪、审计等场景。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "payload")]
pub enum ObserverEvent {
    /// Agent 开始执行
    AgentStart {
        /// Agent 名称
        agent_name: String,
        /// Provider 名称
        provider: String,
        /// 模型名称
        model: String,
        /// 输入文本
        input_preview: String,
    },

    /// Agent 执行结束
    AgentEnd {
        /// Agent 名称
        agent_name: String,
        /// 执行步数
        steps: usize,
        /// Token 使用量
        usage: Option<Usage>,
        /// 估计成本（USD）
        cost_usd: Option<f64>,
    },

    /// LLM 请求开始
    LlmRequestStart {
        /// Provider 名称
        provider: String,
        /// 模型名称
        model: String,
        /// 消息数量
        messages_count: usize,
        /// 工具数量
        tools_count: usize,
    },

    /// LLM 响应完成
    LlmResponse {
        /// Provider 名称
        provider: String,
        /// 模型名称
        model: String,
        /// 请求耗时
        duration: Duration,
        /// 是否成功
        success: bool,
        /// Token 使用量
        usage: Option<Usage>,
    },

    /// LLM 流式响应块
    LlmStreamChunk {
        /// Provider 名称
        provider: String,
        /// 模型名称
        model: String,
        /// 块序号
        chunk_index: usize,
        /// 增量文本长度
        delta_len: usize,
    },

    /// 工具调用开始
    ToolCallStart {
        /// 工具名称
        tool: String,
        /// 调用 ID
        tool_call_id: String,
        /// 参数预览
        arguments_preview: String,
    },

    /// 工具调用完成
    ToolCallEnd {
        /// 工具名称
        tool: String,
        /// 调用 ID
        tool_call_id: String,
        /// 执行耗时
        duration: Duration,
        /// 是否成功
        success: bool,
        /// 输出预览
        output_preview: String,
    },

    /// 工具缓存命中
    ToolCacheHit {
        /// 工具名称
        tool: String,
        /// 节省的 Token 数（估算）
        tokens_saved: u64,
    },

    /// 工具缓存未命中
    ToolCacheMiss {
        /// 工具名称
        tool: String,
    },

    /// 熔断器开启
    CircuitBreakerOpen {
        /// 工具名称
        tool: String,
        /// 连续失败次数
        consecutive_failures: u32,
    },

    /// 熔断器关闭
    CircuitBreakerClose {
        /// 工具名称
        tool: String,
    },

    /// 重试事件
    ToolRetry {
        /// 工具名称
        tool: String,
        /// 重试次数
        attempt: u32,
        /// 最大重试次数
        max_retries: u32,
    },

    /// 超时事件
    ToolTimeout {
        /// 工具名称
        tool: String,
        /// 超时时间
        timeout: Duration,
    },

    /// 策略拒绝
    PolicyDenied {
        /// 工具名称
        tool: String,
        /// 规则 ID
        rule_id: String,
        /// 拒绝原因
        reason: String,
    },

    /// 步骤完成
    StepComplete {
        /// 步骤序号
        step: usize,
        /// 最大步骤数
        max_steps: usize,
        /// 本步决策类型
        decision_type: String,
    },

    /// 循环检测警告
    LoopDetected {
        /// 工具名称
        tool: String,
        /// 重复次数
        repeat_count: usize,
        /// 警告级别
        level: String,
    },

    /// Context Overflow 恢复
    ContextOverflowRecovered {
        /// 恢复策略
        strategy: String,
        /// 删除/截断的消息数
        messages_affected: usize,
    },

    /// 会话开始
    SessionStart {
        /// 会话 ID
        session_id: String,
        /// 渠道
        channel: String,
    },

    /// 会话结束
    SessionEnd {
        /// 会话 ID
        session_id: String,
        /// 持续时间
        duration: Duration,
    },

    /// 错误事件
    Error {
        /// 组件名称
        component: String,
        /// 错误类型
        error_type: String,
        /// 错误消息
        message: String,
    },

    /// 心跳事件
    Heartbeat {
        /// 活跃会话数
        active_sessions: u64,
        /// 队列深度
        queue_depth: u64,
    },

    /// 原始事件（用于自定义事件）
    Raw {
        /// 事件名称
        name: String,
        /// 事件数据
        data: serde_json::Value,
    },
}

/// 观测器指标（数值指标）
///
/// 用于监控、告警、统计等场景，可以被 Prometheus、OpenTelemetry 等系统收集。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "metric_type", content = "value")]
pub enum ObserverMetric {
    /// 请求延迟（毫秒）
    RequestLatencyMs(f64),

    /// Token 使用量
    TokensUsed {
        /// 提示词 Token 数
        prompt: u64,
        /// 输出 Token 数
        completion: u64,
        /// 总 Token 数
        total: u64,
    },

    /// 活跃会话数
    ActiveSessions(u64),

    /// 队列深度
    QueueDepth(u64),

    /// 工具调用次数
    ToolCallCount {
        /// 工具名称
        tool: String,
        /// 调用次数
        count: u64,
    },

    /// 工具调用延迟（毫秒）
    ToolLatencyMs {
        /// 工具名称
        tool: String,
        /// 平均延迟
        avg_latency: f64,
    },

    /// 错误率
    ErrorRate {
        /// 错误类型
        error_type: String,
        /// 错误率（0.0 - 1.0）
        rate: f64,
    },

    /// 缓存命中率
    CacheHitRate(f64),

    /// 熔断器状态（0=关闭, 1=半开, 2=开启）
    CircuitBreakerState {
        /// 工具名称
        tool: String,
        /// 状态值
        state: u8,
    },

    /// 内存使用量（字节）
    MemoryUsageBytes(u64),

    /// CPU 使用率（百分比）
    CpuUsagePercent(f64),

    /// 自定义计数器
    Counter {
        /// 指标名称
        name: String,
        /// 计数
        value: u64,
        /// 标签
        labels: Option<HashMap<String, String>>,
    },

    /// 自定义计量器
    Gauge {
        /// 指标名称
        name: String,
        /// 数值
        value: f64,
        /// 标签
        labels: Option<HashMap<String, String>>,
    },

    /// 直方图
    Histogram {
        /// 指标名称
        name: String,
        /// 观测值
        value: f64,
        /// 桶边界
        buckets: Vec<f64>,
    },
}

/// 指标标签
pub type MetricLabels = HashMap<String, String>;

/// 双轨观测器 trait
///
/// 同时支持事件和指标的观测器接口。
#[async_trait::async_trait]
pub trait DualTrackObserver: Send + Sync {
    /// 记录事件
    fn observe_event(&self, event: ObserverEvent);

    /// 记录指标
    fn record_metric(&self, metric: ObserverMetric);

    /// 记录带标签的指标
    fn record_metric_with_labels(&self, metric: ObserverMetric, labels: MetricLabels) {
        let _ = labels;
        self.record_metric(metric);
    }

    /// 刷新缓冲区（如果有）
    fn flush(&self) {}
}

/// 多路复用观测器
///
/// 将事件和指标分发到多个后端观测器。
pub struct MultiObserver {
    event_observers: Vec<Box<dyn DualTrackObserver>>,
}

impl MultiObserver {
    /// 创建新的多路复用观测器
    pub fn new() -> Self {
        Self {
            event_observers: Vec::new(),
        }
    }

    /// 添加观测器
    pub fn add_observer(mut self, observer: Box<dyn DualTrackObserver>) -> Self {
        self.event_observers.push(observer);
        self
    }
}

impl Default for MultiObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DualTrackObserver for MultiObserver {
    fn observe_event(&self, event: ObserverEvent) {
        for observer in &self.event_observers {
            observer.observe_event(event.clone());
        }
    }

    fn record_metric(&self, metric: ObserverMetric) {
        for observer in &self.event_observers {
            observer.record_metric(metric.clone());
        }
    }

    fn flush(&self) {
        for observer in &self.event_observers {
            observer.flush();
        }
    }
}

/// 日志观测器
///
/// 将事件记录为结构化日志。
pub struct LoggingObserver {
    level: LogLevel,
}

impl LoggingObserver {
    /// 创建新的日志观测器
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
        }
    }

    /// 设置日志级别
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
}

impl Default for LoggingObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DualTrackObserver for LoggingObserver {
    #[allow(clippy::cognitive_complexity)]
    fn observe_event(&self, event: ObserverEvent) {
        match event {
            ObserverEvent::AgentStart { ref agent_name, .. } => {
                tracing::info!(agent_name, "agent.start");
            }
            ObserverEvent::AgentEnd {
                ref agent_name,
                steps,
                ..
            } => {
                tracing::info!(agent_name, steps, "agent.end");
            }
            ObserverEvent::LlmRequestStart {
                ref provider,
                ref model,
                ..
            } => {
                tracing::debug!(provider, model, "llm.request.start");
            }
            ObserverEvent::LlmResponse {
                ref provider,
                success,
                ..
            } => {
                if success {
                    tracing::debug!(provider, "llm.response.success");
                } else {
                    tracing::warn!(provider, "llm.response.failure");
                }
            }
            ObserverEvent::ToolCallStart { ref tool, .. } => {
                tracing::debug!(tool, "tool.call.start");
            }
            ObserverEvent::ToolCallEnd {
                ref tool, success, ..
            } => {
                if success {
                    tracing::debug!(tool, "tool.call.end");
                } else {
                    tracing::warn!(tool, "tool.call.failure");
                }
            }
            ObserverEvent::ToolCacheHit { ref tool, .. } => {
                tracing::debug!(tool, "tool.cache.hit");
            }
            ObserverEvent::ToolCacheMiss { ref tool } => {
                tracing::debug!(tool, "tool.cache.miss");
            }
            ObserverEvent::Error {
                ref component,
                ref message,
                ..
            } => {
                tracing::error!(component, message, "observer.error");
            }
            ObserverEvent::LoopDetected {
                ref tool,
                repeat_count,
                ..
            } => {
                tracing::warn!(tool, repeat_count, "loop.detected");
            }
            _ => {
                if self.level <= LogLevel::Debug {
                    tracing::debug!(event = ?event, "observer.event");
                }
            }
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn record_metric(&self, metric: ObserverMetric) {
        match metric {
            ObserverMetric::RequestLatencyMs(latency) => {
                tracing::debug!(latency_ms = latency, "metric.request_latency");
            }
            ObserverMetric::TokensUsed {
                prompt,
                completion,
                total,
            } => {
                tracing::debug!(prompt, completion, total, "metric.tokens_used");
            }
            ObserverMetric::ActiveSessions(count) => {
                tracing::debug!(count, "metric.active_sessions");
            }
            ObserverMetric::ToolCallCount { ref tool, count } => {
                tracing::debug!(tool, count, "metric.tool_call_count");
            }
            ObserverMetric::CacheHitRate(rate) => {
                tracing::debug!(rate, "metric.cache_hit_rate");
            }
            _ => {
                tracing::debug!(metric = ?metric, "metric.recorded");
            }
        }
    }
}

/// 详细日志观测器
///
/// 记录所有事件的详细信息，用于调试。
pub struct VerboseObserver;

impl VerboseObserver {
    /// 创建新的详细观测器
    pub fn new() -> Self {
        Self
    }
}

impl Default for VerboseObserver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DualTrackObserver for VerboseObserver {
    fn observe_event(&self, event: ObserverEvent) {
        tracing::trace!(event = ?event, "verbose.event");
    }

    fn record_metric(&self, metric: ObserverMetric) {
        tracing::trace!(metric = ?metric, "verbose.metric");
    }
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// 指标聚合器
///
/// 用于聚合多个指标值，计算统计信息。
pub struct MetricAggregator {
    values: Vec<f64>,
}

impl MetricAggregator {
    /// 创建新的聚合器
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// 添加值
    pub fn add(&mut self, value: f64) {
        self.values.push(value);
    }

    /// 获取平均值
    pub fn average(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.values.iter().sum::<f64>() / self.values.len() as f64
        }
    }

    /// 获取最小值
    pub fn min(&self) -> f64 {
        self.values.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    /// 获取最大值
    pub fn max(&self) -> f64 {
        self.values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// 获取百分位数
    pub fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    /// 获取样本数
    pub fn count(&self) -> usize {
        self.values.len()
    }

    /// 清空
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl Default for MetricAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// 事件构建器
pub struct EventBuilder {
    event: Option<ObserverEvent>,
}

impl EventBuilder {
    /// 创建新的事件构建器
    pub fn new() -> Self {
        Self { event: None }
    }

    /// 设置 Agent 开始事件
    pub fn agent_start(
        mut self,
        agent_name: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.event = Some(ObserverEvent::AgentStart {
            agent_name: agent_name.into(),
            provider: provider.into(),
            model: model.into(),
            input_preview: String::new(),
        });
        self
    }

    /// 设置工具调用开始事件
    pub fn tool_call_start(
        mut self,
        tool: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Self {
        self.event = Some(ObserverEvent::ToolCallStart {
            tool: tool.into(),
            tool_call_id: tool_call_id.into(),
            arguments_preview: String::new(),
        });
        self
    }

    /// 构建事件
    pub fn build(self) -> Option<ObserverEvent> {
        self.event
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_event_serialization() {
        let event = ObserverEvent::AgentStart {
            agent_name: "test".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            input_preview: "hello".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("AgentStart"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_observer_metric_serialization() {
        let metric = ObserverMetric::TokensUsed {
            prompt: 100,
            completion: 50,
            total: 150,
        };

        let json = serde_json::to_string(&metric).unwrap();
        assert!(json.contains("TokensUsed"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_metric_aggregator() {
        let mut agg = MetricAggregator::new();
        agg.add(1.0);
        agg.add(2.0);
        agg.add(3.0);
        agg.add(4.0);
        agg.add(5.0);

        assert_eq!(agg.average(), 3.0);
        assert_eq!(agg.min(), 1.0);
        assert_eq!(agg.max(), 5.0);
        assert_eq!(agg.count(), 5);
        assert_eq!(agg.percentile(50.0), 3.0);
    }

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new()
            .agent_start("my_agent", "openai", "gpt-4")
            .build();

        assert!(matches!(event, Some(ObserverEvent::AgentStart { .. })));
    }
}
