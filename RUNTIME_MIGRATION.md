# AgentKit Runtime 合并说明

## 变更概述

将 `agentkit-runtime` crate 合并到 `agentkit` crate 中，作为可选的 `runtime` feature。

## 变更原因

1. **简化依赖**：用户只需添加 `agentkit` 一个依赖即可使用所有功能
2. **避免循环依赖**：解决 `agentkit` 和 `agentkit-runtime` 之间的潜在循环依赖问题
3. **统一管理**：所有功能在一个 crate 中，便于维护和版本管理

## 迁移指南

### 之前（多依赖）

```toml
[dependencies]
agentkit = "0.1"
agentkit-runtime = "0.1"
```

```rust
use agentkit::provider::OpenAiProvider;
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
```

### 现在（单依赖）

```toml
[dependencies]
# 完整功能（默认包含 runtime 和 skills）
agentkit = "0.1"

# 或最小化（无 runtime）
agentkit = { version = "0.1", default-features = false }

# 或只启用 runtime
agentkit = { version = "0.1", features = ["runtime"] }
```

```rust
// 方式 1: 使用 runtime 模块
use agentkit::runtime::{DefaultRuntime, ToolRegistry};

// 方式 2: 使用 prelude（推荐）
use agentkit::prelude::*;
```

## Feature 说明

| Feature | 默认 | 说明 |
|---------|------|------|
| `runtime` | ✅ | 运行时支持（DefaultRuntime, ToolRegistry） |
| `skills` | ✅ | Skills 系统支持，包含内置工具加载 |
| `mcp` | ❌ | MCP 协议支持 |
| `a2a` | ❌ | A2A 协议支持 |
| `rhai-skills` | ❌ | Rhai 脚本技能支持 |

## 代码迁移

### 导入语句更新

**之前：**
```rust
use agentkit_runtime::{DefaultRuntime, ToolRegistry};
use agentkit_runtime::loader::ToolLoader;
```

**现在：**
```rust
use agentkit::runtime::{DefaultRuntime, ToolRegistry};
use agentkit::runtime::loader::ToolLoader;

// 或使用 prelude
use agentkit::prelude::*;  // 包含 DefaultRuntime, ToolRegistry
```

### Cargo.toml 更新

**之前：**
```toml
[dependencies]
agentkit = { path = "../../agentkit" }
agentkit-runtime = { path = "../../agentkit-runtime" }
```

**现在：**
```toml
[dependencies]
agentkit = { path = "../../agentkit", features = ["runtime"] }
```

## 项目结构

```
agentkit/
├── src/
│   ├── runtime/           # ← 新增（原 agentkit-runtime）
│   │   ├── mod.rs
│   │   ├── default_runtime.rs
│   │   ├── tool_registry.rs
│   │   ├── loader.rs
│   │   ├── policy.rs
│   │   ├── tool_execution.rs
│   │   ├── trace.rs
│   │   └── utils.rs
│   ├── agent/
│   ├── provider/
│   ├── tools/
│   └── ...
└── Cargo.toml
```

## 优势

1. ✅ **简化依赖管理**：用户只需管理一个 crate
2. ✅ **避免循环依赖**：彻底解决架构问题
3. ✅ **更好的 Feature 控制**：可以按需启用功能
4. ✅ **统一的版本管理**：所有功能使用同一版本号
5. ✅ **减少编译时间**：减少 crate 间切换

## 向后兼容性

此变更为**破坏性变更**，需要用户更新导入语句和 Cargo.toml。

## 示例代码

### 基本使用

```rust
use agentkit::prelude::*;
use agentkit::provider::OpenAiProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建 Provider
    let provider = OpenAiProvider::from_env()?;
    
    // 创建运行时（使用 prelude）
    let runtime = DefaultRuntime::new(
        Arc::new(provider),
        ToolRegistry::new()
    ).with_system_prompt("你是有用的助手");
    
    // 执行对话
    let input = AgentInput::new("你好");
    let output = runtime.run(input).await?;
    
    println!("{}", output.text().unwrap_or("无回复"));
    Ok(())
}
```

### 使用内置工具

```toml
[dependencies]
agentkit = { version = "0.1", features = ["skills"] }
```

```rust
use agentkit::runtime::loader::ToolLoader;

let loader = ToolLoader::new()
    .load_builtin_tools()  // 需要 skills feature
    .load_skills_from_dir("skills")
    .await?;

let registry = loader.build();
```

## 状态

- ✅ 代码迁移完成
- ✅ 所有编译通过
- ✅ 示例代码更新
- ✅ 文档更新
