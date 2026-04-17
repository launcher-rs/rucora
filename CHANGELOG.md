# 变更日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [未发布] - 2026-04-17

### Zeroclaw 架构 P1/P2 特性实现

#### P1 优先级：健壮性与可维护性

**1. Memory Namespace 高级内存系统 (`advanced_types.rs` / `advanced_trait.rs`)**
- 新增 `MemoryEntry` 结构，支持命名空间隔离和重要性评分
- 新增 `MemoryNamespace` 枚举，支持 Session/User/Agent/Team/Org/Global 六级命名空间
- 新增 `MemoryImportance` 重要性评分（1-10级）
- 新增 GDPR 合规支持：`gdpr_export` 导出用户数据，`gdpr_delete` 删除用户数据
- 新增程序记忆存储：`ProceduralMemory` 存储可复用技能
- **新增 API**:
  - `AdvancedMemory::store_in_namespace()` - 命名空间存储
  - `AdvancedMemory::query_with_importance()` - 按重要性查询
  - `AdvancedMemory::gdpr_export/gdpr_delete()` - GDPR 合规操作
  - `AdvancedMemory::store_procedure/recall_procedure()` - 程序记忆

**2. Tool Filter Groups 工具过滤组 (`filter.rs`)**
- 新增 `ToolFilter` 工具过滤器，支持 always/dynamic 可见性组
- 新增 `ToolGroup` 工具组管理
- 新增 `ToolFilterConfig` 过滤配置
- 新增 `ToolGroupManager` 组管理器
- 支持动态工具可见性控制，优化 LLM 工具选择
- **新增 API**:
  - `ToolFilter::new()` - 创建过滤器
  - `ToolFilter::with_always_tool/with_dynamic_tool()` - 添加工具
  - `ToolFilter::get_visible_tools()` - 获取可见工具
  - `ToolGroupManager::create_group()` - 创建工具组

**3. History Atomic Pruning 历史记录原子化裁剪**
- 已实现于 `emergency_history_trim`，确保 assistant + tool 消息组原子删除
- 保持消息配对完整性，防止孤儿消息

#### P2 优先级：架构与可扩展性

**4. Hook Priority System 钩子优先级系统 (`hooks.rs`)**
- 新增 `VoidHook` trait - 无返回值钩子（日志、监控等）
- 新增 `ModifyingHook` trait - 可修改数据钩子（转换、验证等）
- 新增 `HookPriority` 优先级枚举（Critical/High/Normal/Low/Background）
- 新增 `HookRegistry` 钩子注册表，支持优先级排序
- 支持 before/after 阶段钩子
- **新增 API**:
  - `HookRegistry::register_void()` - 注册无返回值钩子
  - `HookRegistry::register_modifying()` - 注册修改钩子
  - `HookRegistry::run_void()` - 执行无返回值钩子链
  - `HookRegistry::run_modifying()` - 执行修改钩子链

**5. RuntimeAdapter 跨平台运行时适配器 (`runtime_adapter.rs`)**
- 新增 `RuntimeAdapter` trait，抽象跨平台运行时能力
- 新增 `NativeRuntimeAdapter` 原生运行时（完整功能）
- 新增 `RestrictedRuntimeAdapter` 受限运行时（安全环境）
- 支持异步 shell 命令执行（tokio::process::Command）
- 支持文件系统操作和内存预算查询
- **新增 API**:
  - `RuntimeAdapter::execute_shell()` - 执行 shell 命令
  - `RuntimeAdapter::read_file/write_file()` - 文件操作
  - `RuntimeAdapter::get_memory_budget()` - 获取内存预算
  - `NativeRuntimeAdapter::new()` - 创建原生适配器
  - `RestrictedRuntimeAdapter::new()` - 创建受限适配器

**6. Pure Interface Layer 纯接口层分离 (`error_classifier_trait.rs` / `injection_guard_trait.rs`)**
- 将 `ErrorClassifier` 和 `InjectionGuard` 的 trait 定义与实现分离
- trait 层（`agentkit-core`）：纯接口，无重依赖
- 实现层（`agentkit`）：具体实现，可独立演进
- 符合接口隔离原则，提高模块可测试性
- **新增 Trait**:
  - `ErrorClassifier` - 错误分类接口
  - `InjectionGuard` - 注入防护接口

**7. Dual-track Metrics 双轨指标系统 (`metrics.rs`)**
- 新增 `ObserverEvent` 结构化事件枚举（LLM 调用开始/完成、工具调用等）
- 新增 `ObserverMetric` 数值指标枚举（Token 使用量、延迟等）
- 新增 `DualTrackObserver` trait，支持事件和指标双轨观察
- 分离结构化事件和数值指标，优化监控和告警
- **新增 API**:
  - `DualTrackObserver::on_event()` - 接收结构化事件
  - `DualTrackObserver::on_metric()` - 接收数值指标
  - `ObserverEvent::LlmCallStart/LlmCallComplete/ToolCall/Error` - 事件类型
  - `ObserverMetric::TokenUsage/Latency/QueueDepth` - 指标类型

#### 修改文件清单

**新建文件 (8)**:
- `agentkit-core/src/memory/advanced_types.rs` - 高级内存类型
- `agentkit-core/src/memory/advanced_trait.rs` - 高级内存 trait
- `agentkit-core/src/tool/filter.rs` - 工具过滤组
- `agentkit-core/src/channel/hooks.rs` - 钩子优先级系统
- `agentkit-core/src/channel/metrics.rs` - 双轨指标系统
- `agentkit-core/src/agent/runtime_adapter.rs` - 运行时适配器
- `agentkit-core/src/error_classifier_trait.rs` - 错误分类纯接口
- `agentkit-core/src/injection_guard_trait.rs` - 注入防护纯接口

**修改文件 (2)**:
- `agentkit-core/Cargo.toml` - 添加 tokio fs/process features
- `agentkit-core/src/lib.rs` - 导出新模块

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

### Agent 执行能力增强（参考 Zeroclaw 架构）

**参考项目**: Zeroclaw 最新代码  
**研究报告**: `docs/ZEROCLAW_ARCHITECTURE_ANALYSIS.md`

#### P0 优先级：安全性与稳定性

**1. Credential 清洗 (`scrub_credentials`)**
- 新增 `scrub_credentials()` 函数，基于正则表达式检测并脱敏敏感 KV 对
- 匹配模式：token、api_key、password、secret、user_key、bearer、credential 等
- 在工具输出返回给 LLM 前自动执行清洗
- 防止 API Key / Token 等敏感信息泄露到 LLM 上下文
- 新增模块：`agentkit/src/agent/tool_execution.rs` 新增 `SENSITIVE_KV_REGEX`

**2. 孤儿 Tool 消息清理 (`remove_orphaned_tool_messages`)**
- 新增 `remove_orphaned_tool_messages()` 函数，修复 context 截断后的孤儿 tool 消息问题
- 两阶段算法：第一遍删除连续 assistant+tool_calls 之间的非法对，第二遍清理剩余孤儿
- 每次 Agent Loop 迭代前自动执行，防止 Anthropic / MiniMax 等 Provider 返回 400 错误
- 新增模块：`agentkit/src/agent/execution.rs` 新增 `remove_orphaned_tool_messages`、`extract_tool_call_id_from_content`

#### P1 优先级：健壮性与可维护性

**3. Loop Detector 循环检测 (`loop_detector.rs`)**
- 新增 `loop_detector.rs` 模块，防止 Agent 陷入无限循环
- 使用滑动窗口 + 哈希签名检测重复工具调用
- 四级响应：Ok（正常）→ Warning（注入系统消息）→ Block（替换输出）→ Break（终止循环）
- 支持 `LoopDetectorConfig` 配置（enabled、window_size、max_repeats）
- Builder 方法 `with_loop_detector_config()` 支持自定义配置
- 串行路径和并发路径均集成检测
- 新增 4 个单元测试
- **新增 API**:
  - `LoopDetectorConfig` - 循环检测器配置
  - `LoopDetector::new(config)` - 创建检测器
  - `LoopDetector::record(tool_name, args, output)` - 记录调用并返回检测结果
  - `LoopDetectionResult` - 枚举：Ok / Warning / Block / Break

**4. Context Overflow 内联恢复 (`fast_trim_tool_results` / `emergency_history_trim`)**
- LLM 调用失败时自动检测 context window 溢出并尝试恢复
- 两阶段恢复策略：
  - Stage 1：快速裁剪旧 tool 消息到 2000 字符（保留首 2/3 + 尾 1/3）
  - Stage 2：紧急删除最旧 1/3 消息（assistant + tool 组原子删除，保持配对完整）
- 恢复成功后 `continue` 重试 LLM 调用，避免任务直接失败
- 支持多种 Provider 错误消息模式检测（context length、context window、token limit 等）
- 新增工具函数：`truncate_tool_content`、`floor_char_boundary`
- **新增 API**:
  - `fast_trim_tool_results(messages, protect_last_n)` - 快速裁剪 tool 消息
  - `emergency_history_trim(messages, keep_recent)` - 紧急删除历史消息
  - `is_context_overflow_error(message)` - 检测 context 溢出错误

#### 修改文件
- `agentkit/src/agent/mod.rs` - 注册 `loop_detector` 模块
- `agentkit/src/agent/loop_detector.rs` - 新增循环检测器模块
- `agentkit/src/agent/execution.rs` - 集成 loop detector、孤儿清理、context 恢复
- `agentkit/src/agent/tool_execution.rs` - 新增 credential 清洗
- `docs/ZEROCLAW_ARCHITECTURE_ANALYSIS.md` - 新增架构分析报告

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
