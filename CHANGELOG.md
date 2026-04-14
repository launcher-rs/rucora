# 变更日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [未发布] - 2026-04-09

### 代码质量与安全改进

#### P0 严重问题修复

**1. HTTP 超时配置**
- 为所有 8 个 Provider 添加 HTTP 超时配置
- 默认请求超时：120 秒
- 默认连接超时：15 秒
- 防止请求无限挂起，提高系统可用性
- 新增 `http_config.rs` 模块统一管理超时配置

**2. Gemini API Key 安全修复**
- 修复 API Key 暴露在 URL 查询参数中的安全问题
- 改用 `x-goog-api-key` 请求头传递 API Key
- 消除 API Key 泄露到日志和代理服务器的风险

**3. ResilientProvider 退避算法修复**
- 修复抖动计算始终返回 0 的数学 bug
- 使用基于 attempt 的伪随机算法生成有效抖动
- 重试策略现在能有效分散请求，避免重试风暴

**4. AgentError 统一定义**
- 消除 `agentkit-core` 中两个重复的 `AgentError` 定义
- 添加 `RequiresRuntime` 变体到统一错误类型
- 更新所有使用旧变体（`MaxStepsReached`）的代码为新变体（`MaxStepsExceeded`）
- 接口统一，减少使用困惑

**5. ShellTool 安全策略增强**
- 新增命令白名单/黑名单机制
- 增强危险操作符检测（管道、重定向、命令替换等）
- 添加路径遍历攻击防护
- 添加环境变量泄露检测
- 支持配置化安全策略
- 支持工作目录设置
- 修复 args 参数传递问题

**6. Agent 默认实现改进**
- 将默认最大步骤数从 10 增加到 20
- 改进 Chat/ToolCall 决策的错误提示
- 完善文档注释，明确说明默认实现的局限性

#### P1 高优先级问题修复

**7. 错误可重试性修正**
- 从 `is_retriable()` 中移除 `ErrorCategory::Model`（模型错误通常是永久性错误）
- 将 `ToolError::Timeout` 的 `retriable` 标记为 `false`（工具超时不应重试）
- 将 `ProviderError::Model` 的 `retriable` 标记为 `false`
- 避免无效重试，提高错误处理效率

**8. ProviderError::Timeout 的 retry_after 映射修复**
- 修复 `elapsed`（已消耗时间）被错误映射到 `retry_after`（建议等待时间）的问题
- 将 `retry_after` 设置为 `None`
- 提供准确的重试建议

#### P2 中优先级问题修复

**9. tokio features 优化**
- `agentkit-core` 将 `tokio` features 从 `["full"]` 改为 `["sync", "time", "macros", "rt"]`
- 减少编译时间和二进制大小

**10. AgentInput 初始值统一**
- 统一 `AgentInput::new()` 和 `AgentInputBuilder::new()` 的初始值
- 都使用 `Value::Object(serde_json::Map::new())` 作为 context 初始值
- 消除行为不一致问题

### Hermes Agent 高优先级特性集成

**参考项目**: Hermes Agent v0.9.0 (Nous Research)  
**研究报告**: `docs/HERMES_AGENT_RESEARCH.md`

#### 1. 结构化错误分类器 (`error_classifier.rs`)
- 新增 `FailoverReason` 枚举，14 种精细错误原因分类
- 新增 `ClassifiedError` 结构，包含恢复策略的分类结果
- 新增 `ErrorClassifier` 分类器，优先级排序的分类管线
- 支持错误类型：认证/计费/速率限制/上下文溢出/模型不存在等
- 自动判断：是否可重试、是否应压缩、是否应回退、是否应轮换凭证
- 为每种错误类型推荐退避时间
- **新增 API**:
  - `ErrorClassifier::classify(error, context)` - 分类错误
  - `ProviderError::classify(context)` - 便捷方法
  - `FailoverReason::is_retryable()` - 判断是否可重试
  - `FailoverReason::should_compress()` - 判断是否应压缩
  - `FailoverReason::should_fallback()` - 判断是否应回退
  - `FailoverReason::recommended_backoff_ms()` - 推荐退避时间

#### 2. Prompt 注入防护扫描器 (`injection_guard.rs`)
- 新增 `ThreatType` 枚举，8 种威胁类型
- 新增 `InjectionGuard` 扫描器，基于正则的多模式检测
- 新增 `ScanResult` 结果，包含威胁详情和清理后内容
- 检测模式：指令忽略/规则规避/信息隐藏/权限绕过/秘密读取/数据外泄/隐藏 Unicode/角色伪装
- 威胁等级评估（1-5 级）
- 支持内容清理（移除或标记危险片段）
- 提供便捷的扩展方法 `scan_for_injection()`
- **新增 API**:
  - `InjectionGuard::scan(content, source)` - 扫描内容
  - `InjectionGuard::quick_scan(content, source)` - 快速扫描
  - `ContentScannable::scan_for_injection(source)` - 扩展方法

#### 3. 分层上下文压缩引擎 (`compact/engine.rs`)
- 新增 `LayeredCompressor` 分层压缩引擎
- 新增 `CompressionConfig` 压缩配置
- 新增 `CompressionStrategy` 压缩策略（Aggressive/Balanced/Conservative）
- 实现智能分层压缩算法：
  1. 修剪旧工具结果（廉价预压缩）
  2. 保护头部消息（系统提示 + 首次交互）
  3. 按 Token 预算保护尾部消息
  4. 用结构化 LLM 提示摘要中间回合
  5. 后续压缩时迭代更新先前摘要
- 结构化摘要模板：Goal/Progress/Decisions/Questions/Files 等
- 支持冷却期机制，防止频繁压缩
- **新增 API**:
  - `LayeredCompressor::should_compress(tokens, window)` - 判断是否需要压缩
  - `LayeredCompressor::compress(provider, messages, window)` - 执行压缩
  - `CompressionConfig::aggressive()` - 激进压缩配置
  - `CompressionConfig::conservative()` - 保守压缩配置

### 新增示例

#### 22. 结构化错误分类器示例 (`22_error_classification.rs`)
- 展示错误分类器的基础使用
- 演示 6 种常见错误场景的分类
- 展示便捷方法和 FailoverReason 判断方法
- 演示实际 Provider 调用中的错误分类
- 提供恢复策略决策树说明

#### 23. Prompt 注入防护扫描器示例 (`23_prompt_injection_guard.rs`)
- 展示 8 种威胁类型的检测
- 演示各种威胁场景的检测效果
- 展示内容清理功能
- 演示安全内容不会被误报
- 展示便捷扩展方法
- 提供实际应用场景示例

#### 24. 分层上下文压缩引擎示例 (`24_context_compression.rs`)
- 展示压缩配置说明
- 演示三种压缩策略（Aggressive/Balanced/Conservative）
- 展示判断是否需要压缩
- 解释消息分层概念
- 展示结构化摘要模板
- 提供迭代压缩机制说明
- 给出实际应用建议

### 新增依赖

- `regex = "1"` - 用于错误分类和注入检测
- `tracing = "0.1"` - 用于安全扫描日志（agentkit-core）

### 新增导出

**agentkit-core**:
- `ErrorClassifier`, `ErrorContext`, `ClassifiedError`, `FailoverReason`
- `InjectionGuard`, `ScanResult`, `Threat`, `ThreatType`, `ContentScannable`

**agentkit**:
- `CompressionConfig`, `CompressionStrategy`, `LayeredCompressor`

### 修改文件清单

#### 新建文件 (6)
- `agentkit-core/src/error_classifier.rs` - 结构化错误分类器
- `agentkit-core/src/injection_guard.rs` - Prompt 注入防护扫描器
- `agentkit/src/compact/engine.rs` - 分层上下文压缩引擎
- `agentkit/examples/22_error_classification.rs` - 错误分类器示例
- `agentkit/examples/23_prompt_injection_guard.rs` - 注入防护示例
- `agentkit/examples/24_context_compression.rs` - 上下文压缩示例

#### 修改文件 (5)
- `agentkit-core/src/lib.rs` - 导出新模块和类型
- `agentkit-core/Cargo.toml` - 添加 regex 和 tracing 依赖
- `agentkit/src/lib.rs` - 重新导出压缩引擎类型
- `agentkit/Cargo.toml` - 添加新 example 配置
- `docs/HERMES_AGENT_RESEARCH.md` - Hermes Agent 研究报告

---

### 测试修复

#### Compact 模块测试修复

**修复 3 个失败的测试**：

1. **test_group_messages**
   - 修复消息分组算法逻辑错误
   - 正确识别用户消息开始的 API 轮次
   - 支持连续 assistant 消息的分组

2. **test_generate_partial_compact_prompt**
   - 更新 `PARTIAL_COMPACT_PROMPT` 常量
   - 确保提示词包含"最近的消息"关键字

3. **test_should_compact**
   - 调整测试参数（使用较小的 buffer 和 gpt-4 模型）
   - 增加消息长度和数量以确保触发压缩阈值

**测试结果**：
- ✅ 81 个测试全部通过（72 agentkit + 9 agentkit-core）
- ✅ 0 个测试失败

### 修改文件清单

#### 新建文件 (5)
- `agentkit/src/provider/http_config.rs` - HTTP 客户端配置模块
- `docs/CODE_AUDIT_REPORT.md` - 完整代码审计报告
- `docs/CODE_IMPROVEMENT_REPORT.md` - 改进实施报告
- `docs/P0_FIXES_COMPLETE.md` - P0 修复完成报告
- `docs/COMPACT_TESTS_FIX_REPORT.md` - 测试修复报告

#### 修改文件 (20)
- `agentkit-core/Cargo.toml` - 优化 tokio features
- `agentkit-core/src/agent/mod.rs` - 统一 AgentError，改进默认实现
- `agentkit-core/src/error.rs` - 添加 RequiresRuntime，修正可重试性
- `agentkit/src/agent/execution.rs` - 更新错误变体
- `agentkit/src/compact/grouping.rs` - 修复消息分组算法
- `agentkit/src/compact/mod.rs` - 修复测试
- `agentkit/src/compact/prompt.rs` - 更新提示词常量
- `agentkit/src/provider/anthropic.rs` - 添加 HTTP 超时
- `agentkit/src/provider/azure_openai.rs` - 添加 HTTP 超时
- `agentkit/src/provider/deepseek.rs` - 添加 HTTP 超时
- `agentkit/src/provider/gemini.rs` - 添加 HTTP 超时，修复 API Key 泄露
- `agentkit/src/provider/mod.rs` - 添加 http_config 模块导出
- `agentkit/src/provider/moonshot.rs` - 添加 HTTP 超时
- `agentkit/src/provider/ollama.rs` - 添加 HTTP 超时
- `agentkit/src/provider/openai.rs` - 添加 HTTP 超时
- `agentkit/src/provider/openrouter.rs` - 添加 HTTP 超时
- `agentkit/src/provider/resilient.rs` - 修复退避算法抖动 bug
- `agentkit/src/tools/cmd_exec.rs` - 更新函数调用
- `agentkit/src/tools/shell.rs` - 增强安全策略
- `examples/agentkit-skills-example/src/main.rs` - 更新 ShellTool 使用

---

## [0.1.0] - 2024-01-XX

### 新增功能

#### 核心框架
- **agentkit-core** - 核心抽象层（traits/types）
- **agentkit** - 主库（实现聚合）
- **agentkit-runtime** - 运行时编排层

#### Provider 支持
- **OpenAiProvider** - OpenAI GPT 系列模型
- **OllamaProvider** - Ollama 本地模型

#### 工具系统
- **ShellTool** - 执行系统命令
- **FileReadTool** - 读取本地文件
- **FileWriteTool** - 写入文件
- **HttpRequestTool** - HTTP 请求
- **GitTool** - Git 操作
- 等 12+ 内置工具

#### 技能系统
- **RhaiSkill** - Rhai 脚本技能
- **CommandSkill** - 命令模板技能
- **FileReadSkill** - 文件读取技能

#### 协议支持
- **MCP 协议** - Model Context Protocol
- **A2A 协议** - Agent-to-Agent Protocol

#### 应用
- **agentkit-cli** - 命令行工具
- **agentkit-server** - HTTP 服务器

### 项目结构

```
agentkit/
├── agentkit-core       # 核心抽象层
├── agentkit            # 主库（实现聚合）
├── agentkit-runtime    # 运行时编排
├── agentkit-skills     # 技能系统
├── agentkit-cli        # 命令行工具
├── agentkit-server     # HTTP 服务器
├── agentkit-mcp        # MCP 协议支持
└── agentkit-a2a        # A2A 协议支持
```

---

## 迁移指南

### 从 0.1.0 迁移到当前版本

#### 1. 更新依赖配置

```toml
# 旧配置（0.1.0）
[dependencies]
agentkit = "0.1"
agentkit-runtime = "0.1"
agentkit-mcp = "0.1"
agentkit-a2a = "0.1"
agentkit-skills = "0.1"

# 新配置（当前版本）
[dependencies]
agentkit = { version = "0.1", features = ["runtime", "mcp", "a2a", "skills"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
anyhow = "1"
```

#### 2. 更新导入语句

```rust
// 旧导入方式（0.1.0）
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use agentkit_core::agent::AgentInput;
use agentkit_mcp::McpClient;
use agentkit_skills::load_skills_from_dir;

// 新导入方式（当前版本）
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::prelude::AgentInput;
use agentkit::mcp::McpClient;
use agentkit::skills::load_skills_from_dir;
```

#### 3. 更新 AgentInput 使用

```rust
// 旧用法（0.1.0）
let input = AgentInput {
    messages: vec![ChatMessage::user("你好")],
    metadata: None,
};

// 新用法（当前版本）
let input = AgentInput::new("你好");
```

#### 4. 更新 AgentOutput 访问

```rust
// 旧用法（0.1.0）
println!("{}", output.message.content);

// 新用法（当前版本）
println!("{}", output.text().unwrap_or("无回复"));
```

#### 5. 更新运行时使用

```rust
// 旧用法（0.1.0）
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use std::sync::Arc;

let provider = OpenAiProvider::from_env()?;
let runtime = DefaultRuntime::new(
    Arc::new(provider),
    ToolRegistry::new()
).with_system_prompt("你是有用的助手");

let input = AgentInput::new("用一句话介绍 Rust");
let output = runtime.run(input).await?;

// 新用法（当前版本）
use agentkit::provider::OpenAiProvider;
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::prelude::AgentInput;
use std::sync::Arc;

let provider = OpenAiProvider::from_env()?;
let runtime = DefaultRuntime::new(
    Arc::new(provider),
    ToolRegistry::new()
).with_system_prompt("你是有用的助手");

let input = AgentInput::new("用一句话介绍 Rust");
let output = runtime.run(input).await?;
println!("{}", output.text().unwrap_or("无回复"));
```

---

## 贡献

欢迎贡献代码、文档或反馈问题！

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。
