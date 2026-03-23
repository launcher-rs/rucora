# 变更日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [未发布]

### 架构调整

#### Crate 合并（破坏性变更）

**变更内容**：
- 将 `agentkit-mcp`、`agentkit-a2a`、`agentkit-skills` 三个独立 crate 合并到 `agentkit` 主库中
- 移除 `agentkit-cli` 和 `agentkit-server` 独立 crate
- Workspace 成员从 10 个减少到 4 个

**影响**：
- 用户只需依赖 `agentkit` 一个 crate 即可使用所有功能
- 导入路径发生变化

**迁移指南**：
```rust
// 旧导入方式
use agentkit_mcp::McpClient;
use agentkit_a2a::client::Client;
use agentkit_skills::load_skills_from_dir;

// 新导入方式
use agentkit::mcp::McpClient;
use agentkit::a2a::client::Client;
use agentkit::skills::load_skills_from_dir;
```

```toml
# 旧依赖配置
[dependencies]
agentkit = "0.1"
agentkit-mcp = "0.1"
agentkit-a2a = "0.1"
agentkit-skills = "0.1"

# 新依赖配置
[dependencies]
agentkit = { version = "0.1", features = ["mcp", "a2a", "skills"] }
```

#### DefaultAgent API 重构（破坏性变更）

**变更内容**：
- 移除了 `tools` 字段，`DefaultAgent` 现在只持有 `provider`，职责更单一

**迁移指南**：
```rust
// 旧代码
let agent = DefaultAgent::builder()
    .provider(provider)
    .tools(tools)  // 不再需要
    .build();

// 新代码
let agent = DefaultAgent::builder()
    .provider(provider)
    .build();
```

#### AgentInput API 改进（破坏性变更）

**变更内容**：
- 字段简化，移除 `messages` 和 `metadata` 字段
- 使用更简洁的构造方式

**迁移指南**：
```rust
// 旧代码
let input = AgentInput {
    messages: vec![ChatMessage::user("你好")],
    metadata: None,
};

// 新代码
let input = AgentInput::new("你好");
```

#### AgentOutput API 改进（破坏性变更）

**变更内容**：
- 简化输出结构，使用 `Value` 统一返回
- 新增辅助方法：`text()`, `message_count()`, `tool_call_count()`

**迁移指南**：
```rust
// 旧代码
println!("{}", output.message.content);

// 新代码
println!("{}", output.text().unwrap_or("无回复"));
```

### 新增功能

#### MCP 协议支持（可选）

**功能说明**：
- 支持连接 MCP（Model Context Protocol）服务器
- 将 MCP 工具转换为 agentkit 的 Tool trait
- 支持多种传输层（Stdio、HTTP）

**使用示例**：
```rust
use agentkit::mcp::{McpClient, StdioTransport};

// 创建传输层并连接
let transport = StdioTransport::new("mcp-server");
let client = McpClient::connect(transport).await?;

// 列出可用工具
let tools = client.list_tools().await?;
```

**启用方式**：
```toml
[dependencies]
agentkit = { version = "0.1", features = ["mcp"] }
```

#### A2A 协议支持（可选）

**功能说明**：
- 支持 Agent 之间的通信与协作
- 支持任务委托与结果返回
- 支持多 Agent 系统编排

**使用示例**：
```rust
use agentkit::a2a::client::Client;

// 连接远程 Agent
let client = Client::connect("http://agent-server:8080").await?;

// 发送任务
let task = client.send_task("process_data", "input data").await?;
```

**启用方式**：
```toml
[dependencies]
agentkit = { version = "0.1", features = ["a2a"] }
```

#### Skills 技能系统（默认启用）

**功能说明**：
- 支持 Rhai 脚本技能（可选）
- 支持命令模板技能（基于 SKILL.md）
- 支持从目录动态加载技能

**使用示例**：
```rust
use agentkit::skills::load_skills_from_dir;

// 从目录加载技能
let skills = load_skills_from_dir("skills").await?;

// 转换为工具列表
let tools = skills.as_tools();
```

**启用方式**：
```toml
[dependencies]
agentkit = { version = "0.1", features = ["skills", "rhai-skills"] }
```

### 改进优化

#### 项目结构优化
- 简化依赖管理，用户只需依赖一个 crate
- 统一版本号，所有功能使用同一版本
- 减少 crate 数量，降低维护成本
- 改善内部集成，子模块间可直接调用

#### 编译优化
- 避免多个 crate 间的重复编译
- 减少链接时间
- 统一文档，所有功能在一个 crate 的文档中

### 文档更新

#### 新增文档
- `docs/QUICK_REFERENCE.md` - 快速参考手册
- `docs/MERGE_COMPLETE.md` - Crate 合并完成报告

#### 更新文档
- `readme.md` - 更新项目结构和使用示例
- `docs/README.md` - 添加文档导航

### 删除内容

#### 移除的 Crate
- `agentkit-cli` - 命令行工具（可参考源码自行实现）
- `agentkit-server` - HTTP 服务器（可参考源码自行实现）

#### 移除的文档（开发过程文档）
- `AGENT_BUILDER_OPTIMIZATION.md`
- `AUTO_FIX_COMPLETE.md`
- `AUTO_FIX_PROGRESS.md`
- `FINAL_STATUS.md`
- `IMPROVEMENTS.md`
- `PROJECT_SUMMARY.md`
- `REFACTORING_COMPLETE.md`
- `REFACTORING.md`
- `RUNTIME_MIGRATION.md`
- `STATUS.md`
- `TODO_FIXES.md`
- `WARNINGS_FIXED.md`

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
