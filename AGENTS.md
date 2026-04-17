# AGENTS.md

This file provides guidance to Qoder (qoder.com) when working with code in this repository.

## Build and Test Commands

### Building
```bash
# Check all packages compile
cargo check --all-targets

# Build all packages
cargo build

# Build with all features
cargo build --all-features
```

### Testing
```bash
# Run all tests in workspace
cargo test --workspace

# Run tests for specific package
cargo test -p agentkit-core
cargo test -p agentkit

# Run a specific test by name
cargo test test_name_pattern

# Run tests with output visible
cargo test -- --nocapture

# Run contract tests (behavior validation for core traits)
cargo test -p agentkit-core --test contract_provider
cargo test -p agentkit-core --test contract_tool
cargo test -p agentkit-core --test contract_vector_store
```

### Linting
```bash
# Run clippy with workspace lint configuration
cargo clippy --all-targets

# Auto-fix clippy warnings where possible
cargo clippy --fix --all-targets
```

### Running Examples
```bash
# Run a specific example
cargo run --example 01_hello_world
cargo run --example 03_chat_with_tools

# Run example with all features
cargo run --example 12_mcp --all-features
```

## Project Architecture

### Workspace Structure

This is a Rust workspace with multiple crates:

- **`agentkit-core/`** - Core abstractions (traits, types, errors). No implementations, minimal dependencies.
- **`agentkit/`** - Aggregator crate providing "batteries-included" implementations (providers, tools, skills, memory, retrieval).
- **`agentkit-providers/`** - LLM provider implementations (OpenAI, Anthropic, Gemini, Ollama, etc.)
- **`agentkit-tools/`** - Tool implementations (Shell, File, HTTP, Git, etc.)
- **`agentkit-mcp/`** - MCP (Model Context Protocol) integration
- **`agentkit-a2a/`** - A2A (Agent-to-Agent) protocol integration
- **`agentkit-skills/`** - Skill system (Rhai scripts, command templates)
- **`agentkit-embed/`** - Embedding providers
- **`agentkit-retrieval/`** - Vector store implementations
- **`examples/`** - Example applications

### Key Design Principles

1. **Interface/Implementation Separation**: `agentkit-core` defines only traits and types. Third parties can implement their own providers/tools without depending on heavy implementations.

2. **Feature-gated Modules**: The main `agentkit` crate uses Cargo features to enable optional functionality:
   - `providers` - LLM provider implementations
   - `tools` - Tool implementations
   - `skills` - Skill system
   - `mcp` - MCP protocol support
   - `a2a` - A2A protocol support
   - `embed` - Embedding support
   - `retrieval` - Vector store support

3. **Unified Event Model**: `ChannelEvent` is the central event type for all communication between runtime, tools, and skills.

4. **Pure Interface Layer**: Error classifier and injection guard traits are separated from their implementations (in `error_classifier_trait.rs` and `injection_guard_trait.rs`). The old implementations in `error_classifier.rs` and `injection_guard.rs` are deprecated.

### Core Traits (agentkit-core)

- **`LlmProvider`** - LLM provider abstraction (chat, stream_chat)
- **`Tool`** - Tool abstraction (name, description, input_schema, call)
- **`VectorStore`** - Vector store abstraction (upsert, search, delete)
- **`Memory`** - Memory abstraction (store, retrieve)
- **`Skill`** - Skill abstraction (higher-level reusable capabilities)
- **`Runtime`** - Runtime execution abstraction
- **`ErrorClassifier`** - Error classification trait (pure interface)
- **`InjectionGuard`** - Prompt injection detection trait (pure interface)

### Lint Configuration

The workspace has extensive clippy lint configuration in the root `Cargo.toml`. Key lints that will fail CI:
- `unnecessary_wraps` - Functions that don't need Result shouldn't return Result
- `clone_on_copy` - Copy types shouldn't use clone()
- `unused_async` - Async functions must actually use async
- `needless_collect` - Unnecessary collect() calls

### Code Style Requirements

Per `.cursor/rules/chinese-comments.mdc`: All code comments should be in Chinese.

### Environment Variables for Testing

Common environment variables used by providers:
- `OPENAI_API_KEY` - OpenAI API key
- `ANTHROPIC_API_KEY` - Anthropic API key
- `GOOGLE_API_KEY` - Google Gemini API key
- `OPENAI_BASE_URL` - Custom API base URL (for Ollama, etc.)

### Contract Tests

The `agentkit-core` crate contains contract tests that validate trait behavior expectations:
- `tests/contract_provider.rs` - Validates LlmProvider implementations
- `tests/contract_tool.rs` - Validates Tool implementations
- `tests/contract_vector_store.rs` - Validates VectorStore implementations

These tests ensure third-party implementations meet the expected behavioral contracts.
