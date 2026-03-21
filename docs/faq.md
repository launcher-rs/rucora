# AgentKit 常见问题解答 (FAQ)

## 📋 目录

1. [安装和配置](#安装和配置)
2. [使用问题](#使用问题)
3. [性能和成本](#性能和成本)
4. [故障排除](#故障排除)
5. [最佳实践](#最佳实践)

---

## 安装和配置

### Q: AgentKit 支持哪些 Rust 版本？

**A**: AgentKit 需要 Rust 1.70 或更高版本。建议使用最新稳定版：

```bash
rustup update stable
```

### Q: 如何安装 AgentKit？

**A**: 在 `Cargo.toml` 中添加：

```toml
[dependencies]
agentkit = "0.1"
agentkit-runtime = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Q: 支持哪些 LLM Provider？

**A**: 目前支持：
- ✅ OpenAI (GPT-4, GPT-3.5)
- ✅ Ollama (本地模型)
- 🔜 Anthropic (计划中)
- 🔜 自定义 Provider

### Q: 如何配置 API Key？

**A**: 有三种方式：

1. **环境变量**（推荐）
   ```bash
   export OPENAI_API_KEY=sk-your-key
   ```

2. **配置文件**
   ```yaml
   # config.yaml
   profiles:
     default:
       provider:
         kind: openai
         openai:
           api_key: "sk-your-key"
   ```

3. **代码中**
   ```rust
   let provider = OpenAiProvider::new(
       "https://api.openai.com/v1",
       "sk-your-key"
   );
   ```

---

## 使用问题

### Q: 如何创建第一个 Agent？

**A**: 参考 [快速入门](./quick_start.md)，5 分钟即可创建。

### Q: 如何实现多轮对话？

**A**: 使用 `ConversationManager`：

```rust
use agentkit::conversation::ConversationManager;

let mut conv = ConversationManager::new();
conv.add_user_message("你好");
conv.add_assistant_message("你好！");

// 获取历史
let messages = conv.get_messages();
```

### Q: 如何让 Agent 使用工具？

**A**: 在 `ToolRegistry` 中注册工具：

```rust
let tools = ToolRegistry::new()
    .register(FileReadTool::new())
    .register(HttpRequestTool::new());

let runtime = DefaultRuntime::new(provider, tools);
```

### Q: 如何创建自定义工具？

**A**: 实现 `Tool` trait：

```rust
#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> Option<&str> { "描述" }
    fn input_schema(&self) -> Value { json!({...}) }
    
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 实现逻辑
    }
}
```

### Q: 如何限制对话长度？

**A**: 使用 `with_max_messages`：

```rust
let manager = ConversationManager::new()
    .with_max_messages(20);  // 保留最近 20 条
```

---

## 性能和成本

### Q: 如何追踪 API 成本？

**A**: 使用 `CostTracker`：

```rust
use agentkit::cost::CostTracker;

let tracker = CostTracker::new();
tracker.record_usage("gpt-4", 100, 50, 0.0045).await;

let cost = tracker.get_current_cost().await;
```

### Q: 如何设置预算限制？

**A**: 

```rust
let tracker = CostTracker::new()
    .with_budget_limit(10.0);  // $10 预算

if !tracker.check_budget(10.0).await {
    // 超出预算，停止服务
}
```

### Q: 如何计算 Token 数？

**A**: 使用 `TokenCounter`：

```rust
let counter = TokenCounter::new("gpt-4");
let tokens = counter.count_text("Hello");
let msg_tokens = counter.count_messages(&messages);
```

### Q: AgentKit 的性能如何？

**A**: 
- ⚡ 内存占用：极低（<10MB 基础）
- ⚡ 启动时间：<100ms
- ⚡ 请求延迟：主要取决于 API，框架开销 <1ms

### Q: 如何优化成本？

**A**: 
1. 使用 `ConversationManager` 限制历史长度
2. 使用 `TokenCounter` 监控用量
3. 设置预算警报
4. 对简单任务使用更便宜的模型

---

## 故障排除

### Q: 提示 "缺少 OPENAI_API_KEY"

**A**: 确保已设置环境变量：

```bash
# Linux/Mac
export OPENAI_API_KEY=sk-your-key

# Windows PowerShell
$env:OPENAI_API_KEY="sk-your-key"
```

### Q: 请求超时怎么办？

**A**: 
1. 检查网络连接
2. 增加超时设置
3. 使用 `ResilientProvider` 自动重试

```rust
use agentkit::provider::ResilientProvider;

let resilient = ResilientProvider::new(Arc::new(provider))
    .with_config(RetryConfig::new()
        .with_max_retries(3)
        .with_timeout_ms(30000));
```

### Q: 如何处理限流错误？

**A**: `ResilientProvider` 会自动处理：

```rust
// 自动检测 429 错误并重试
let resilient = ResilientProvider::new(provider);
```

### Q: 工具调用失败怎么办？

**A**: 检查：
1. 工具是否正确注册
2. 输入是否符合 schema
3. 查看错误诊断信息

```rust
match tool.call(input).await {
    Ok(result) => { /* 成功 */ }
    Err(err) => {
        let diag = err.diagnostic();
        println!("错误类型：{}", diag.kind);
        println!("是否可重试：{}", diag.retriable);
    }
}
```

### Q: 对话历史丢失怎么办？

**A**: 确保：
1. 使用 `ConversationManager` 管理历史
2. 需要持久化时调用 `to_json()` 保存

```rust
// 保存
let json = manager.to_json()?;
std::fs::write("conv.json", json)?;

// 加载
let json = std::fs::read_to_string("conv.json")?;
let manager = ConversationManager::from_json(&json)?;
```

---

## 最佳实践

### Q: 如何组织大型项目？

**A**: 推荐结构：

```
my-project/
├── src/
│   ├── main.rs
│   ├── tools/      # 自定义工具
│   ├── prompts/    # Prompt 模板
│   └── config/     # 配置文件
├── Cargo.toml
└── .env            # 环境变量
```

### Q: 如何测试 Agent？

**A**: 使用 Mock Provider：

```rust
struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat(&self, _request: ChatRequest) 
        -> Result<ChatResponse, ProviderError> 
    {
        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content: "测试回复".to_string(),
                name: None,
            },
            tool_calls: vec![],
            usage: None,
            finish_reason: None,
        })
    }
}
```

### Q: 如何部署到生产环境？

**A**: 
1. 使用环境变量管理配置
2. 设置预算限制
3. 启用日志和监控
4. 使用 `Middleware` 添加限流

```rust
let chain = MiddlewareChain::new()
    .with(LoggingMiddleware::new())
    .with(RateLimitMiddleware::new(100))
    .with(MetricsMiddleware::new());
```

### Q: 如何保证安全性？

**A**: 
1. 不要硬编码 API Key
2. 使用 `PromptTemplate` 转义用户输入
3. 对工具调用设置权限
4. 启用审计日志

```rust
// 安全的 Prompt 模板
let template = PromptTemplate::from_string(
    "处理以下数据：{{data}}"
);
// 会自动转义危险字符
```

---

## 其他问题

### Q: 有社区支持吗？

**A**: 
- 📧 问题反馈：GitHub Issues
- 💬 讨论：GitHub Discussions
- 📖 文档：docs/ 目录

### Q: 如何贡献代码？

**A**: 
1. Fork 项目
2. 创建分支
3. 提交 PR
4. 通过 CI 检查

### Q: 许可证是什么？

**A**: 查看项目根目录的 `LICENSE` 文件。

---

## 📚 相关资源

- [用户指南](./user_guide.md) - 详细使用文档
- [快速入门](./quick_start.md) - 10 分钟上手
- [示例集合](./cookbook.md) - 实用代码示例
- [API 参考](./api_reference.md) - 完整 API 文档

**没有找到答案？欢迎提交 Issue！**
