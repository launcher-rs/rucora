# AgentKit 改进计划

本文档列出了代码审查中发现的问题及修复计划。

## 修复优先级

### Phase 1: 关键架构问题（必须修复）

#### 1. DefaultAgent 设计问题
**问题**: `DefaultAgent` 持有不使用的 `tools` 字段，违反单一职责原则

**当前代码**:
```rust
pub struct DefaultAgent<P, T> {
    provider: P,
    tools: T,  // 未使用
    system_prompt: Option<String>,
    default_model: Option<String>,
}
```

**修复方案**:
- 方案 A: 移除 `tools` 字段，将 `DefaultAgent` 定位为纯决策者
- 方案 B: 将 `DefaultAgent` 移至 `agentkit` crate（而非 `agentkit-core`）
- 方案 C: 重构 `think()` 方法使其真正使用 `tools` 进行自主决策

**推荐**: 方案 A + C 结合
```rust
// 在 agentkit-core 中
pub struct DefaultAgent<P> {
    provider: P,
    system_prompt: Option<String>,
    default_model: Option<String>,
}

// 在 agentkit 中提供更高级的 Agent 实现
pub struct ToolUsingAgent<P, T> {
    agent: DefaultAgent<P>,
    tools: T,
}
```

**影响范围**: 
- `agentkit-core/src/agent/mod.rs`
- 所有使用 `DefaultAgent` 的代码

---

#### 2. ToolRegistry trait 位置问题
**问题**: `ToolRegistry` 定义在 `agentkit-runtime` 中，但被 `agentkit-core` 引用

**修复方案**:
1. 在 `agentkit-core/src/tool/` 中定义 trait:
```rust
// agentkit-core/src/tool/registry.rs
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>>;
    fn list_tools(&self) -> Vec<Arc<dyn Tool>>;
    async fn call(&self, name: &str, input: Value) -> Result<Value, ToolError>;
}
```

2. `agentkit-runtime::ToolRegistry` 实现该 trait

**影响范围**:
- 需要重构 `agentkit-core/src/tool/mod.rs`
- 更新 `agentkit-runtime/src/tool_registry.rs`

---

#### 3. 循环依赖风险
**问题**: `agentkit-core` 的 dev-dependencies 包含 `agentkit`

**当前代码**:
```toml
# agentkit-core/Cargo.toml
[dev-dependencies]
agentkit = { path = "../agentkit" }
```

**修复方案**:
1. 移除该依赖
2. 使用 mock 实现进行测试
3. 将集成测试移至 `agentkit` crate

---

### Phase 2: API 设计改进（重要）

#### 4. AgentInput 类型安全改进
**问题**: `extras: Value` 类型不安全

**当前代码**:
```rust
pub struct AgentInput {
    pub text: String,
    pub extras: Value,  // 不安全
}
```

**修复方案**:
```rust
// 方案 A: 使用泛型
pub struct AgentInput<T = ()> {
    pub text: String,
    pub extras: T,
}

// 方案 B: 使用 builder 模式
pub struct AgentInput {
    pub text: String,
    pub metadata: Option<Metadata>,
    pub context: Option<Context>,
}

impl AgentInput {
    pub fn builder(text: impl Into<String>) -> AgentInputBuilder {
        AgentInputBuilder::new(text)
    }
}
```

---

#### 5. 错误类型统一
**问题**: `AgentError::Message(String)` 过于笼统

**当前代码**:
```rust
pub enum AgentError {
    Message(String),  // 丢失结构化信息
    MaxStepsExceeded { max_steps },
    ProviderError { source: ProviderError },
}
```

**修复方案**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent 执行失败：{message}")]
    Execution { 
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("达到最大步数限制（max_steps={max_steps}）")]
    MaxStepsExceeded { 
        max_steps: usize,
        actual_steps: usize,
    },
    
    #[error("Provider 错误：{source}")]
    Provider { 
        #[from]
        source: ProviderError,
    },
    
    #[error("工具调用失败：{tool_name} - {message}")]
    ToolCall {
        tool_name: String,
        message: String,
    },
}
```

---

#### 6. RuntimeObserver 异步支持
**问题**: 同步方法限制扩展性

**当前代码**:
```rust
pub trait RuntimeObserver: Send + Sync {
    fn on_event(&self, event: ChannelEvent);  // 同步
}
```

**修复方案**:
```rust
// 方案 A: 改为异步
#[async_trait]
pub trait RuntimeObserver: Send + Sync {
    async fn on_event(&self, event: ChannelEvent);
}

// 方案 B: 提供两个版本
pub trait RuntimeObserver: Send + Sync {
    fn on_event(&self, event: ChannelEvent);
}

#[async_trait]
pub trait AsyncRuntimeObserver: Send + Sync {
    async fn on_event(&self, event: ChannelEvent);
}
```

---

### Phase 3: 性能优化（重要）

#### 7. 减少不必要的克隆
**问题**: `tool_definitions()` 每次都返回新的 `Vec`

**修复方案**:
```rust
// 当前
pub fn definitions(&self) -> Vec<ToolDefinition> {
    self.tools.iter().map(|t| t.definition()).collect()
}

// 改进
pub fn definitions(&self) -> &[ToolDefinition] {
    &self.tool_definitions  // 缓存
}

// 或使用 Arc
pub fn definitions(&self) -> Arc<Vec<ToolDefinition>> {
    self.tool_definitions.clone()
}
```

---

#### 8. 异步代码中的阻塞操作
**问题**: 文件 IO 未使用 `spawn_blocking`

**修复方案**:
```rust
// 当前
let content = tokio::fs::read_to_string(path).await?;

// 改进（对于大文件）
let content = tokio::task::spawn_blocking(move || {
    std::fs::read_to_string(path)
}).await??;
```

---

### Phase 4: 用户体验改进（重要）

#### 9. 改进错误信息
**问题**: 错误信息缺少上下文和修复建议

**当前**:
```
错误：加载配置失败 - 文件不存在
```

**改进后**:
```
错误：加载配置文件失败

原因：文件 '/path/to/config.toml' 不存在

建议：
1. 检查文件路径是否正确
2. 运行 `agentkit init` 创建默认配置文件
3. 设置环境变量 AGENTKIT_CONFIG 指定配置文件路径

详细信息：
  文件路径：/path/to/config.toml
  错误代码：CONFIG_FILE_NOT_FOUND
```

---

#### 10. 完善示例
**问题**: 示例缺少错误处理和实际场景

**修复方案**:
为每个示例添加：
- 完整的错误处理
- 实际使用场景
- 性能基准测试
- 最佳实践说明

---

## 实施时间表

### Week 1-2: Phase 1
- [ ] 重构 `DefaultAgent`
- [ ] 移动 `ToolRegistry` trait
- [ ] 移除循环依赖

### Week 3-4: Phase 2
- [ ] 改进 `AgentInput` 设计
- [ ] 统一错误类型
- [ ] 添加 `AsyncRuntimeObserver`

### Week 5-6: Phase 3
- [ ] 性能分析和优化
- [ ] 减少克隆操作
- [ ] 优化异步代码

### Week 7-8: Phase 4
- [ ] 改进错误信息
- [ ] 完善示例和文档
- [ ] 增加测试覆盖率

---

## 影响评估

| 改动 | 影响范围 | 破坏性 | 工作量 |
|------|---------|--------|--------|
| DefaultAgent 重构 | 高 | 是 | 中 |
| ToolRegistry 移动 | 高 | 是 | 高 |
| AgentInput 改进 | 中 | 是 | 中 |
| 错误类型统一 | 中 | 是 | 中 |
| RuntimeObserver 异步 | 低 | 是 | 低 |
| 性能优化 | 低 | 否 | 中 |

---

## 迁移指南

针对破坏性改动，需要提供：
1. 详细的迁移文档
2. 代码迁移工具（codemod）
3. 兼容性层（deprecation period）
4. 示例代码更新

---

## 验收标准

- [ ] 所有高优先级问题已修复
- [ ] 编译无警告
- [ ] 测试覆盖率 > 80%
- [ ] 文档完整
- [ ] 示例可运行
- [ ] 性能回归测试通过
