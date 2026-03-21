# AgentKit 示例

本目录包含 AgentKit 的使用示例，帮助开发者快速上手和深入理解。

## 📚 示例列表

### 1. agentkit-examples-complete - 全部使用功能示例

展示 AgentKit 所有核心功能的完整示例。

**功能覆盖：**
- ✅ Provider 配置和使用（OpenAI、Mock）
- ✅ Tool 注册和调用（Echo、Git、HTTP、Shell、Memory）
- ✅ Memory 存储和检索
- ✅ Runtime 配置和执行
- ✅ 完整 Agent 集成

**运行方式：**
```bash
# 设置环境变量（可选，用于测试真实 API）
export OPENAI_API_KEY=your-api-key

# 运行示例
cargo run -p agentkit-examples-complete
```

**输出示例：**
```
=== AgentKit 全部使用功能示例 ===

=== 1. Provider 示例 ===
✓ Mock Provider 创建成功
✓ 非流式聊天成功：Rust 是一门系统编程语言...

=== 2. Tool 示例 ===
Echo 工具：echo
✓ Echo 结果：{"text":"Hello, AgentKit!"}

=== 3. Memory 示例 ===
✓ 添加记忆：user:name = 张三
✓ 检索记忆：1 条结果
```

### 2. agentkit-examples-deep-dive - 深入研究示例

深入展示特定功能的高级用法。

**功能覆盖：**
- ✅ 自定义 Provider 实现（Mock、Delayed）
- ✅ 自定义 Tool 实现（Calculator）
- ✅ 自定义 Runtime 实现（Simple、Logging、Retry）
- ✅ 高级错误处理
- ✅ 性能优化技巧（批量、并发、内存管理）

**运行方式：**
```bash
# 运行所有示例
cargo run -p agentkit-examples-deep-dive

# 运行特定示例
cargo run -p agentkit-examples-deep-dive -- custom_provider
cargo run -p agentkit-examples-deep-dive -- custom_tool
cargo run -p agentkit-examples-deep-dive -- custom_runtime
cargo run -p agentkit-examples-deep-dive -- error_handling
cargo run -p agentkit-examples-deep-dive -- performance
```

**输出示例：**
```
=== AgentKit 深入研究示例 ===

=== 自定义 Provider 示例 ===
✓ Mock Provider 创建成功
✓ 非流式聊天成功：你好！我是一个模拟的 AI 助手。

--- 流式输出 ---
你 好！我 是 一 个 模 拟 的  A I  助 手。
✓ 流式聊天成功
```

## 🎯 学习路径

### 初学者
1. 先运行 `agentkit-examples-complete` 了解基本功能
2. 阅读代码注释理解每个组件的用途
3. 修改示例代码尝试不同配置

### 进阶开发者
1. 运行 `agentkit-examples-deep-dive` 学习高级用法
2. 参考示例实现自定义 Provider/Tool/Runtime
3. 学习性能优化和错误处理技巧

### 高级开发者
1. 结合两个示例创建完整应用
2. 实现自定义组件并集成到 AgentKit 生态
3. 贡献新的示例到项目

## 📋 示例结构

```
examples/
├── README.md
├── agentkit-examples-complete/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs              # 完整功能示例
└── agentkit-examples-deep-dive/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # 入口
        ├── custom_provider.rs   # 自定义 Provider
        ├── custom_tool.rs       # 自定义 Tool
        ├── custom_runtime.rs    # 自定义 Runtime
        ├── error_handling.rs    # 错误处理
        └── performance.rs       # 性能优化
```

## 🔧 环境要求

### 必需
- Rust 1.70+
- Tokio 运行时

### 可选（用于完整功能）
- OpenAI API Key（用于 OpenAI Provider）
  ```bash
  export OPENAI_API_KEY=sk-...
  ```

## 📖 示例说明

### Provider 示例

展示如何实现自定义 Provider：

```rust
use agentkit_core::provider::LlmProvider;
use async_trait::async_trait;

struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // 实现聊天逻辑
        Ok(ChatResponse { ... })
    }
}
```

### Tool 示例

展示如何实现自定义 Tool：

```rust
use agentkit_core::tool::Tool;
use async_trait::async_trait;

struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str { "calculator" }
    
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 实现计算逻辑
        Ok(json!({"result": 42}))
    }
}
```

### Runtime 示例

展示如何实现自定义 Runtime：

```rust
use agentkit_core::runtime::Runtime;
use async_trait::async_trait;

struct SimpleRuntime<P> {
    provider: Arc<P>,
}

#[async_trait]
impl<P: LlmProvider + Send + Sync> Runtime for SimpleRuntime<P> {
    async fn run(&self, input: AgentInput) -> Result<AgentOutput, AgentError> {
        // 实现运行时逻辑
        Ok(AgentOutput { ... })
    }
}
```

## 🐛 问题排查

### 常见问题

1. **API Key 错误**
   ```
   错误：缺少 OpenAI api_key
   解决：设置 OPENAI_API_KEY 环境变量
   ```

2. **编译错误**
   ```
   错误：依赖项缺失
   解决：运行 cargo update 更新依赖
   ```

3. **运行时错误**
   ```
   错误：Provider 调用失败
   解决：检查网络连接和 API 配置
   ```

## 🤝 贡献示例

欢迎贡献新的示例！请遵循以下规范：

1. 在对应示例 crate 中创建新模块
2. 添加详细的中文注释
3. 包含完整的错误处理
4. 提供运行说明
5. 确保代码可编译和运行

## 📄 许可证

与主项目相同。
