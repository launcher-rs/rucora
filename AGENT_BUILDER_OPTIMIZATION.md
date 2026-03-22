# Agent Builder API 优化说明

## 优化背景

原有的 Agent Builder API 将所有外部能力（Tools、MCP、A2A、Skills）都通过 `.tool()` 方法添加，存在以下问题：

1. **语义不清晰**：无法从代码上区分工具、MCP 服务、A2A 代理和 Skills
2. **配置能力弱**：无法为不同来源的能力设置特定配置
3. **类型信息丢失**：所有能力都被转换为 `Arc<dyn Tool>`，丢失了原始类型信息

## 新的 API 设计

### 基本使用

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::EchoTool;

let provider = OpenAiProvider::from_env()?;

let agent = DefaultAgent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .default_model("gpt-4o-mini")
    .tool(EchoTool)  // 添加单个工具
    .tools([ShellTool::new(), GitTool::new()])  // 添加多个工具
    .max_steps(10)
    .build();
```

### 使用 Skills

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;

let agent = DefaultAgent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .with_skills("skills")  // 加载 skills 目录
    .build();
```

### 使用 MCP 服务

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;

let agent = DefaultAgent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .with_mcp("http://localhost:8080")  // MCP 服务器地址
    .build();
```

### 使用 A2A 代理

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;

let agent = DefaultAgent::builder()
    .provider(provider)
    .system_prompt("你是有用的助手")
    .with_a2a("http://agent.example.com")  // A2A 代理 URL
    .build();
```

### 完整示例

```rust
use agentkit::agent::DefaultAgent;
use agentkit::provider::OpenAiProvider;
use agentkit::tools::{EchoTool, ShellTool, GitTool};

let provider = OpenAiProvider::from_env()?;

let agent = DefaultAgent::builder()
    // 基础配置
    .provider(provider)
    .system_prompt("你是有用的助手，可以帮助用户执行命令和管理文件")
    .default_model("gpt-4o-mini")
    .max_steps(15)
    
    // 添加工具
    .tool(EchoTool)
    .tools([ShellTool::new(), GitTool::new()])
    
    // 配置 Skills（需要 skills feature）
    .with_skills("skills")
    
    // 配置 MCP 服务（需要 mcp feature）
    .with_mcp("http://localhost:8080")
    
    // 配置 A2A 代理（需要 a2a feature）
    .with_a2a("http://agent.example.com")
    
    .build();
```

## API 对比

### 原有 API

```rust
// 问题：所有能力都通过 tool() 添加，语义不清晰
let agent = DefaultAgent::builder()
    .provider(provider)
    .tool(EchoTool)
    .tool(mcp_tool)      // MCP 工具？
    .tool(a2a_tool)      // A2A 工具？
    .tool(skill_tool)    // Skill 工具？
    .build();
```

### 新的 API

```rust
// 解决：语义清晰，配置独立
let agent = DefaultAgent::builder()
    .provider(provider)
    .tool(EchoTool)           // 明确的工具
    .with_skills("skills")    // 明确的 Skills 配置
    .with_mcp("http://...")   // 明确的 MCP 配置
    .with_a2a("http://...")   // 明确的 A2A 配置
    .build();
```

## Feature 标志

新的 API 使用 feature 标志来控制编译：

| 方法 | 需要的 Feature | 说明 |
|------|---------------|------|
| `.tool()` / `.tools()` | - | 始终可用 |
| `.with_skills()` | `skills` | 需要启用 skills feature |
| `.with_mcp()` | `mcp` | 需要启用 mcp feature |
| `.with_a2a()` | `a2a` | 需要启用 a2a feature |

## 迁移指南

### 最小改动

如果你的代码只使用工具，不需要修改：

```rust
// 原有代码 - 仍然有效
let agent = DefaultAgent::builder()
    .provider(provider)
    .tool(EchoTool)
    .build();
```

### 推荐迁移

使用新的 API 可以获得更好的语义和配置能力：

```rust
// 推荐方式
let agent = DefaultAgent::builder()
    .provider(provider)
    .tools([EchoTool, ShellTool::new()])
    .with_skills("skills")  // 如果有 skills
    .build();
```

## 优势

1. ✅ **语义清晰**：从代码可以直接看出使用了哪些外部能力
2. ✅ **配置灵活**：可以为不同能力设置独立配置
3. ✅ **类型安全**：编译时检查 feature 标志
4. ✅ **向后兼容**：原有代码无需修改
5. ✅ **易于扩展**：未来可以轻松添加更多配置选项

## 未来扩展

基于新的设计，未来可以轻松添加更多配置选项：

```rust
// 未来可能的 API
let agent = DefaultAgent::builder()
    .provider(provider)
    
    // 工具配置
    .tools([EchoTool, ShellTool::new()])
    
    // Skills 配置
    .with_skills("skills")
    .with_skills_config(SkillsConfig::new().with_cache(true))
    
    // MCP 配置
    .with_mcp("http://localhost:8080")
    .with_mcp_config(McpConfig::new().with_timeout(Duration::from_secs(30)))
    
    // A2A 配置
    .with_a2a("http://agent.example.com")
    .with_a2a_config(A2aConfig::new().with_retry(3))
    
    .build();
```
