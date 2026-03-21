# Changelog

All notable changes to AgentKit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed (Breaking)

#### DefaultAgent API 重构
- **移除了 `tools` 字段**: `DefaultAgent` 现在只持有 `provider`，职责更单一
- **迁移指南**:
  ```rust
  // 旧代码 (v0.1.0)
  let agent = DefaultAgent::builder()
      .provider(provider)
      .tools(tools)  // 不再需要
      .build();
  
  // 新代码 (v0.2.0)
  let agent = DefaultAgent::builder()
      .provider(provider)
      .build();
  ```

#### AgentInput API 改进
- **字段重命名**: `extras` → `context`（语义更清晰）
- **新增 builder 模式**: 支持流式添加上下文
- **迁移指南**:
  ```rust
  // 旧代码 (v0.1.0)
  let input = AgentInput {
      text: "你好".to_string(),
      extras: json!({"key": "value"}),
  };
  
  // 新代码 (v0.2.0)
  let input = AgentInput::builder("你好")
      .with_context("key", json!("value"))
      .build();
  
  // 或简单用法
  let input = AgentInput::new("你好");
  ```

#### AgentOutput 辅助方法
- **新增方法**: `text()`, `message_count()`, `tool_call_count()`
- **向后兼容**: 字段保持不变

### Added

#### New Providers
- **OpenRouterProvider** - Multi-model aggregator service (70+ models)
  - Supports Anthropic Claude, Google Gemini, OpenAI, and more
  - Environment variable: `OPENROUTER_API_KEY`
  - Default base URL: `https://openrouter.ai/api/v1`

- **AnthropicProvider** - Anthropic Claude models
  - Supports Claude 3.5/3 series
  - Environment variable: `ANTHROPIC_API_KEY`
  - Default base URL: `https://api.anthropic.com/v1`

- **GeminiProvider** - Google Gemini models
  - Supports Gemini 1.5 Pro/Flash
  - Environment variable: `GOOGLE_API_KEY` or `GEMINI_API_KEY`
  - Default base URL: `https://generativelanguage.googleapis.com/v1beta`

- **AzureOpenAiProvider** - Azure OpenAI Service
  - Enterprise-grade GPT deployment
  - Environment variables: `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`
  - Supports custom API versions

- **DeepSeekProvider** - DeepSeek models
  - Supports DeepSeek-V3, DeepSeek-R1
  - Environment variable: `DEEPSEEK_API_KEY`
  - Default base URL: `https://api.deepseek.com/v1`

- **MoonshotProvider** - Moonshot AI (月之暗面) Kimi models
  - Supports moonshot-v1-8k/32k/128k
  - Environment variable: `MOONSHOT_API_KEY`
  - Default base URL: `https://api.moonshot.cn/v1`

#### Agent Architecture
- **Agent trait** - Core abstraction for intelligent agents
  - `think()` method for decision making
  - `AgentDecision` enum for action types (Chat, ToolCall, Return, ThinkAgain, Stop)
  - `AgentContext` for managing conversation state
  - `AgentInput` and `AgentOutput` types

- **DefaultAgent** - Built-in agent implementation
  - Builder pattern for configuration
  - Supports custom system prompts
  - Supports default model selection

- **Agent + Runtime integration**
  - `Runtime::run_with_agent()` method for executing agents
  - Support for both standalone and runtime modes
  - Full tool calling support in runtime mode

#### Skills System
- **agentkit-skills crate** - Independent skills module
  - Feature-gated Rhai scripting support (`rhai-skills` feature)
  - Command-based skills from SKILL.md templates
  - File operation skills (FileReadSkill)
  - Dynamic skill loading from directory

#### Documentation
- `docs/agent_runtime_relationship.md` - Detailed explanation of Agent vs Runtime
- `examples/agentkit-examples-complete/src/agent_example.rs` - Complete agent usage examples
- Updated README with provider comparison table

### Changed

#### Breaking Changes
- **AgentInput structure changed**:
  - Old: `AgentInput { messages: Vec<ChatMessage>, metadata: Option<Value> }`
  - New: `AgentInput { text: String, extras: Value }`
  - Migration: Use `AgentInput::new(text)` for simple text input

- **AgentOutput structure changed**:
  - Old: `AgentOutput { message: ChatMessage, tool_results: Vec<ToolResult> }`
  - New: `AgentOutput { value: Value, messages: Vec<ChatMessage>, tool_calls: Vec<ToolCallRecord> }`
  - Migration: Access `output.value` or use `output.value.get("content")`

- **ToolRegistry trait added** to agentkit-core
  - Required for Agent implementation
  - Methods: `get_tool()`, `list_tools()`, `call()`

#### Improvements
- Separated skills into independent crate for better modularity
- Improved error handling with better error messages
- Enhanced streaming support for all providers
- Better type safety with explicit FinishReason enum

### Fixed

- Fixed lifetime issues in async stream implementations
- Fixed type conversions between ProviderError and AgentError
- Fixed ToolRegistry method naming consistency
- Fixed compilation warnings across all crates

### Removed

- Deprecated `AgentInput::from()` implementations in favor of `AgentInput::new()`

## [0.1.0] - 2024-01-XX

### Added

- Initial release of AgentKit framework
- Core abstractions in `agentkit-core`
- Runtime implementation in `agentkit-runtime`
- Built-in providers: OpenAI, Ollama
- Tool system with 12+ built-in tools
- Skills system with Rhai scripting support
- MCP and A2A protocol support
- CLI and server applications

### Project Structure

```
agentkit/
├── agentkit-core       # Core traits and types
├── agentkit            # Main crate (aggregates all)
├── agentkit-runtime    # Runtime orchestration
├── agentkit-skills     # Skills system (NEW)
├── agentkit-cli        # Command-line interface
├── agentkit-server     # HTTP server
├── agentkit-mcp        # MCP protocol support
└── agentkit-a2a        # A2A protocol support
```

### Supported Providers (Initial)

- OpenAI (GPT-4, GPT-3.5)
- Ollama (Local models)

### Built-in Tools

- ShellTool - Execute shell commands
- FileReadTool - Read local files
- FileWriteTool - Write to files
- HttpRequestTool - Make HTTP requests
- GitTool - Git operations
- And more...

---

## Migration Guide

### Migrating from 0.1.0 to Unreleased

#### 1. Update AgentInput usage

```rust
// Old (0.1.0)
let input = AgentInput {
    messages: vec![ChatMessage::user("Hello")],
    metadata: None,
};

// New (Unreleased)
let input = AgentInput::new("Hello");
// Or with extras
let input = AgentInput::with_extras("Hello", json!({"key": "value"}));
```

#### 2. Update AgentOutput access

```rust
// Old (0.1.0)
println!("{}", output.message.content);

// New (Unreleased)
// Option 1: Access value directly
println!("{}", output.value);

// Option 2: Extract content
if let Some(content) = output.value.get("content").and_then(|v| v.as_str()) {
    println!("{}", content);
}
```

#### 3. Using Agent with Runtime

```rust
// Old (0.1.0) - Runtime only
let runtime = DefaultRuntime::new(provider, tools);
let output = runtime.run(input).await?;

// New (Unreleased) - Agent + Runtime
let agent = DefaultAgent::builder()
    .provider(provider)
    .build();
let runtime = DefaultRuntime::new(provider, tools);
let output = runtime.run_with_agent(&agent, input).await?;
```

#### 4. Skills feature flag

```toml
# Old (0.1.0)
[dependencies]
agentkit = "0.1.0"

# New (Unreleased) - Skills is now optional
[dependencies]
agentkit = { version = "0.2.0", features = ["skills"] }
# Or with Rhai support
agentkit = { version = "0.2.0", features = ["skills", "rhai-skills"] }
```

---

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
