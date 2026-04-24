# 错误处理指南

AgentKit 提供了分层的错误处理系统，包括结构化错误枚举、错误分类、注入保护和重试/退避机制。

## 概述

错误处理系统构建在 `agentkit-core` crate 中，包含：

1. **结构化错误枚举** - 每个子系统都有特定的错误类型（Provider、Tool、Agent、Skill、Memory、Channel）
2. **诊断特征** (`DiagnosticError`) - 统一的错误内省 API
3. **错误分类** (`ErrorClassifier` 特征) - 智能故障决策
4. **注入保护** (`InjectionGuard` 特征) - 提示安全
5. **重试/退避机制** - 在提供者和工具执行层面
6. **熔断器、循环检测和上下文溢出恢复** - 在代理执行层面

## 核心错误类型

### ProviderError

LLM 提供者操作的主要错误类型：

```rust
pub enum ProviderError {
    /// 网络错误（可重试）
    Network {
        message: String,
        source: Option<Box<dyn Error>>,
        retriable: bool,
    },

    /// API 错误（带 HTTP 状态码）
    Api {
        status: u16,
        message: String,
        code: Option<String>,
    },

    /// 认证失败（不可重试）
    Authentication {
        message: String,
    },

    /// 速率限制（可重试，可选 retry-after 持续时间）
    RateLimit {
        message: String,
        retry_after: Option<Duration>,
    },

    /// 请求超时（可重试）
    Timeout {
        message: String,
        elapsed: Duration,
    },

    /// 模型错误（例如模型未找到）
    Model {
        message: String,
    },

    /// 通用后备
    Message(String),
}
```

**可重试逻辑：**

| 错误类型 | 可重试 | 说明 |
|----------|--------|------|
| `Network` | 取决于 `retriable` 标志 | 默认为 `true` |
| `RateLimit` | 总是可重试 | - |
| `Timeout` | 总是可重试 | - |
| `Model` | 可重试 | - |
| `Api` | 仅当 status >= 500 | 服务器错误可重试 |
| `Authentication` | 不可重试 | - |
| `Message` | 默认可重试 | - |

**辅助构造函数：**
- `ProviderError::network(message)` - 创建可重试的网络错误
- `ProviderError::authentication(message)` - 创建认证错误
- `ProviderError::rate_limit(message, retry_after)` - 创建速率限制错误

### ToolError

工具执行错误：

```rust
pub enum ToolError {
    /// 通用错误
    Message(String),

    /// 策略拒绝（带 rule_id 和 reason）
    PolicyDenied {
        rule_id: String,
        reason: String,
    },

    /// 工具未找到
    NotFound {
        name: String,
    },

    /// 输入验证失败
    ValidationError {
        message: String,
    },

    /// 执行超时
    Timeout {
        message: String,
    },
}
```

所有变体默认**不可重试**。`PolicyDenied` 映射到 `ErrorCategory::Policy`；`NotFound` 和 `ValidationError` 映射到 `ErrorCategory::Configuration`。

### AgentError

代理执行错误：

```rust
pub enum AgentError {
    /// 通用错误
    Message(String),

    /// 超过最大步数限制
    MaxStepsExceeded {
        max_steps: usize,
    },

    /// 包装 ProviderError
    ProviderError {
        source: ProviderError,
    },

    /// 代理返回需要 Runtime 模式的决策
    RequiresRuntime,
}
```

### 其他错误类型

**SkillError：**
```rust
pub enum SkillError {
    Message(String),
    NotFound { name: String },
    Timeout { message: String },  // retriable = true
}
```

**MemoryError：**
```rust
pub enum MemoryError {
    Message(String),
    NotFound { id: String },
}
```

**ChannelError：**
```rust
pub enum ChannelError {
    Message(String),
}
```

## 错误分类

### ErrorCategory 枚举

```rust
pub enum ErrorCategory {
    Network, Api, Authentication, Authorization,
    RateLimit, Timeout, Model, Tool,
    Policy, Configuration, Other,
}
```

**关键行为：**
- `is_retriable()` - `Network`、`Timeout`、`RateLimit` 为 true
- `is_authentication_error()` - `Authentication`、`Authorization` 为 true
- `is_client_error()` - `Authentication`、`Authorization`、`Configuration`、`Policy` 为 true

### DiagnosticError 特征

所有错误枚举都实现此特征，提供统一的内省 API：

```rust
pub trait DiagnosticError {
    fn diagnostic(&self) -> ErrorDiagnostic;
    fn is_retriable(&self) -> bool { self.diagnostic().retriable }
    fn category(&self) -> ErrorCategory { self.diagnostic().category }
}
```

`ErrorDiagnostic` 结构体携带结构化字段：`kind`、`message`、`retriable`、`source`、`category`、`status_code`、`retry_after`。

**示例：**
```rust
use agentkit_core::error::{ProviderError, DiagnosticError, ErrorCategory};

let error = ProviderError::Api {
    status: 503,
    message: "Service unavailable".into(),
    code: None,
};

// 结构化检查
let diag = error.diagnostic();
assert_eq!(diag.kind, "provider");
assert_eq!(diag.status_code, Some(503));
assert!(diag.retriable);
assert_eq!(diag.category, ErrorCategory::Api);

// 便捷方法
assert!(error.is_retriable());
assert_eq!(error.category(), ErrorCategory::Api);
```

### FailoverReason 枚举

更细粒度的分类，用于故障转移/恢复决策：

```rust
pub enum FailoverReason {
    Auth, AuthPermanent, Billing, RateLimit, Overloaded,
    ServerError, Timeout, ContextOverflow, PayloadTooLarge,
    ModelNotFound, FormatError, ThinkingSignature, LongContextTier, Unknown,
}
```

**决策方法：**

| 方法 | 返回 true 的条件 |
|------|-----------------|
| `is_retryable()` | 除了 `AuthPermanent`、`Billing`、`ModelNotFound`、`FormatError` |
| `should_compress()` | `ContextOverflow`、`PayloadTooLarge`、`LongContextTier` |
| `should_fallback()` | `Billing`、`RateLimit`、`Overloaded`、`ModelNotFound`、`AuthPermanent` |
| `should_rotate_credential()` | 仅 `Auth` |
| `recommended_backoff_ms()` | RateLimit=5000ms, Overloaded=3000ms, Timeout=2000ms, ServerError=1000ms |

## 注入保护

### InjectionGuard 特征

```rust
pub enum ThreatType {
    PromptInjection, DisregardRules, ConcealInfo, BypassRestrictions,
    ReadSecrets, ExfilCurl, HiddenUnicode, RoleImpersonation,
}
```

**严重级别（1-5）：**

| 级别 | 威胁类型 |
|------|---------|
| 5（最严重） | `HiddenUnicode`、`ReadSecrets`、`ExfilCurl` |
| 4 | `PromptInjection`、`RoleImpersonation`、`BypassRestrictions` |
| 3 | `ConcealInfo`、`DisregardRules` |

```rust
pub trait InjectionGuard: Send + Sync {
    fn scan(&self, content: &str, source: &str) -> ScanResult;
    fn quick_scan(&self, content: &str, source: &str) -> ScanResult;
}
```

`ScanResult` 包含：`is_safe`、`threats: Vec<Threat>`、`cleaned_content`、`original_length`。

## 重试/退避机制

### 提供者级别：ResilientProvider

```rust
pub struct RetryConfig {
    pub max_retries: usize,           // 默认：2
    pub base_delay_ms: u64,           // 默认：200
    pub max_delay_ms: u64,            // 默认：2000
    pub timeout_ms: Option<u64>,
    pub retry_non_retriable_once: bool, // 默认：false
}
```

**退避算法：** 带抖动的指数退避：
```rust
fn backoff_delay_ms(&self, attempt: usize) -> u64 {
    let pow = 1u64.checked_shl(attempt.min(16) as u32).unwrap_or(u64::MAX);
    let delay = self.cfg.base_delay_ms.saturating_mul(pow);
    let jitter = (delay / 10).max(1);
    let jitter_offset = (jitter * (attempt as u64 * 6364136223846793005 + 1)) % jitter;
    delay.min(self.cfg.max_delay_ms) + jitter_offset
}
```

**错误分类**（通过字符串匹配）：
- 认证：包含 "auth"、"unauthorized"、"401"、"api key"、"permission"
- 无效请求：包含 "invalid"、"bad request"、"400"、"not found"、"404"
- 速率限制：包含 "rate limit"、"too many requests"、"429"
- 超时：包含 "timeout"、"timed out"
- 网络：包含 "network"、"connection"、"dns"、"socket"、"reset"、"unreachable"
- 不可用：包含 "unavailable"、"503"、"502"、"504"

### 工具级别：增强的工具执行

**重试配置：**
```rust
pub struct RetryConfig {
    pub max_retries: u32,        // 默认：0（禁用）
    pub initial_delay: Duration, // 默认：100ms
    pub max_delay: Duration,     // 默认：10s
    pub strategy: RetryStrategy, // Fixed 或 Exponential
    pub backoff_factor: f64,     // 默认：2.0
    pub only_transient: bool,    // 默认：false
}
```

**熔断器：**
- 状态：`Closed`（正常）、`Open`（拒绝）、`HalfOpen`（探测）
- 配置：`failure_threshold`（默认：5）、`recovery_timeout`（默认：30s）、`half_open_max_calls`（默认：1）
- 每个工具的状态跟踪通过 `CircuitBreakerRegistry`

## 错误传播模式

错误在系统中的流动：

```
Provider (HTTP/API)
  |
  v
ProviderError (Network/Api/Auth/RateLimit/Timeout/Model/Message)
  |
  v
ResilientProvider（带指数退避的重试循环）
  |
  v
Agent Execution (execution.rs)
  |
  +-- 上下文溢出恢复：fast_trim_tool_results -> emergency_history_trim
  +-- 循环检测：LoopDetector (Warning -> Block -> Break)
  |
  v
Tool Execution (tool_execution.rs)
  |
  +-- 策略检查 (ToolPolicy -> ToolError::PolicyDenied)
  +-- 超时控制
  +-- 重试循环（增强配置）
  +-- 熔断器
  +-- 凭证清理（scrub_credentials）
  +-- 输出大小限制 (DEFAULT_TOOL_OUTPUT_MAX_BYTES = 64KB)
  |
  v
AgentError (Message/MaxStepsExceeded/ProviderError/RequiresRuntime)
  |
  v
Middleware 错误钩子 (process_error)
  |
  v
ChannelEvent::Error -> ChannelObserver
```

### 关键传播模式

#### 1. 诊断提取 + 恢复尝试

```rust
// 来自 execution.rs
let response = match self.provider.chat(*request).await {
    Ok(r) => r,
    Err(e) => {
        let diag = e.diagnostic();
        if is_context_overflow_error(&diag.message) {
            // 尝试恢复：裁剪工具结果，然后紧急裁剪
            let saved = fast_trim_tool_results(&mut messages, 4);
            if saved > 0 { continue; }
            let dropped = emergency_history_trim(&mut messages, 4);
            if dropped > 0 { continue; }
        }
        return Err(AgentError::Message(format!(
            "provider error ({}): {}", diag.kind, diag.message
        )));
    }
};
```

#### 2. 错误包装为 JSON 响应（工具错误）

```rust
// 来自 tool_execution.rs
Err(e) => {
    json!({
        "ok": false,
        "error": {"kind": "tool_error", "message": e.to_string()}
    })
}
```

#### 3. 策略检查与结构化拒绝

```rust
// 来自 tool_execution.rs
if let Err(e) = policy.check(&ctx).await {
    match &e {
        ToolError::PolicyDenied { rule_id, reason } => {
            // 返回结构化拒绝给 LLM
            return Ok(ToolResult { tool_call_id, output: denial_json });
        }
        _ => { /* 通用错误处理 */ }
    }
}
```

#### 4. 凭证清理

```rust
// 来自 tool_execution.rs
pub(crate) fn scrub_credentials(input: &str) -> String {
    // 匹配：token/api_key/password/secret/user_key/bearer/credential
    // 保留前 4 个字符，其余替换为 [REDACTED]
}
```

## 使用示例

### 示例 A：创建 Resilient Provider

```rust
use agentkit::provider::OpenAiProvider;
use agentkit::provider::resilient::{ResilientProvider, RetryConfig};
use std::sync::Arc;

let openai = OpenAiProvider::from_env()?;
let resilient = ResilientProvider::new(Arc::new(openai))
    .with_config(RetryConfig {
        max_retries: 3,
        base_delay_ms: 500,
        max_delay_ms: 10_000,
        timeout_ms: None,
        retry_non_retriable_once: false,
    });

// 像普通 LlmProvider 一样使用
let response = resilient.chat(request).await?;
```

### 示例 B：实现带适当错误处理的工具

```rust
use agentkit_core::{error::ToolError, tool::{Tool, ToolCategory}};

#[async_trait]
impl Tool for ShellTool {
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 验证必填字段
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("Missing 'command' field".into()))?;

        // 安全验证
        self.validate_command(command)?; // 违规时返回 ToolError

        // 带超时执行
        let output = timeout(timeout_duration, cmd.output())
            .await
            .map_err(|_| ToolError::Message(format!("Command timed out after {}s", timeout_secs)))?
            .map_err(|e| ToolError::Message(format!("Execution failed: {}", e)))?;

        Ok(json!({ "stdout": stdout, "exit_code": exit_code }))
    }
}
```

### 示例 C：带重试 + 熔断器的增强工具执行

```rust
use agentkit::agent::tool_call_config::*;
use std::time::Duration;

let config = ToolCallEnhancedConfig::new()
    .with_retry(RetryConfig::exponential(3))
    .with_timeout(TimeoutConfig::default_timeout(Duration::from_secs(30))
        .with_tool_timeout("http_request", Duration::from_secs(60)))
    .with_circuit_breaker(CircuitBreakerConfig {
        enabled: true,
        failure_threshold: 5,
        recovery_timeout: Duration::from_secs(30),
        ..Default::default()
    })
    .with_cache(CacheConfig {
        enabled: true,
        default_ttl: Duration::from_secs(300),
        max_entries: 1000,
        ..Default::default()
    });
```

### 示例 D：诊断错误检查

```rust
use agentkit_core::error::{ProviderError, DiagnosticError, ErrorCategory};

let error = ProviderError::Api {
    status: 503,
    message: "Service unavailable".into(),
    code: None,
};

// 结构化检查
let diag = error.diagnostic();
assert_eq!(diag.kind, "provider");
assert_eq!(diag.status_code, Some(503));
assert!(diag.retriable);
assert_eq!(diag.category, ErrorCategory::Api);

// 便捷方法
assert!(error.is_retriable());
assert_eq!(error.category(), ErrorCategory::Api);
```

## 最佳实践

1. **始终使用 `DiagnosticError` 特征** 进行错误检查，而不是字符串匹配。`diagnostic()` 方法提供结构化字段。

2. **使用特定的错误变体** 而非 `Message()` 后备。`ProviderError::RateLimit` 携带 `retry_after`；`ProviderError::Api` 携带 `status`。

3. **提供者级别重试** 使用 `ResilientProvider` 包装。工具级别重试使用 `ToolCallEnhancedConfig` 带 `RetryConfig`。

4. **工具错误应转换为 JSON 响应**，而不是作为异常传播。这允许代理循环继续，LLM 可以看到错误。

5. **PolicyDenied 错误** 应始终返回结构化的 `ToolResult` 并附带 `rule_id` 和 `reason`，以便 LLM 理解调用被阻止的原因。

6. **上下文溢出恢复** 是两阶段过程：首先廉价裁剪旧工具结果，然后紧急删除最旧的非系统消息。

7. **循环检测** 使用滑动窗口（默认 20 个条目）和基于哈希的签名匹配 `(tool_name, args_hash, output_hash)`。

8. **熔断器** 按工具操作，具有可配置的阈值。状态转换：Closed -> Open（N 次失败后）-> HalfOpen（恢复超时后）-> Closed（成功后）。

9. **凭证清理** 在所有工具输出上自动执行。正则表达式匹配常见模式（API 密钥、令牌、密码、机密）并除前 4 个字符外全部编辑。

10. **输出大小限制** 使用 `DEFAULT_TOOL_OUTPUT_MAX_BYTES`（64KB）防止大型工具输出导致上下文溢出。

## 相关文件

- `agentkit-core/src/error.rs` - 所有错误枚举、`ErrorCategory`、`DiagnosticError` 特征
- `agentkit-core/src/error_classifier_trait.rs` - `FailoverReason`、`ClassifiedError`、`ErrorClassifier` 特征
- `agentkit-core/src/injection_guard_trait.rs` - `ThreatType`、`ScanResult`、`InjectionGuard` 特征
- `agentkit-providers/src/resilient.rs` - `ResilientProvider`、`RetryConfig`
- `agentkit/src/agent/execution.rs` - 带上下文溢出恢复的代理执行
- `agentkit/src/agent/tool_execution.rs` - 带策略检查、凭证清理的工具执行
- `agentkit/src/agent/tool_call_config.rs` - `RetryConfig`、`TimeoutConfig`、`CircuitBreakerConfig`
- `agentkit/src/agent/loop_detector.rs` - `LoopDetector`、`LoopDetectorConfig`
- `agentkit/src/agent/policy.rs` - `ToolPolicy` 特征、`DefaultToolPolicy`
