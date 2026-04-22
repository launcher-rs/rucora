# Agent LLM 请求参数统一配置 - 研究报告

## 问题分析

### 当前架构缺陷

AgentKit 的 Agent 设计缺少 LLM 请求参数配置能力：

1. **硬编码 temperature**: `AgentContext::default_chat_request()` 硬编码 `temperature: Some(0.7)`
2. **参数不完整**: 仅 SimpleAgent/ChatAgent 有 temperature builder，其他 Agent 完全没有
3. **无法配置**: top_p、top_k、max_tokens、frequency_penalty、presence_penalty、stop 等参数无法设置
4. **缺乏扩展性**: 无法传递 provider 特定的额外参数

### 影响范围

| Agent 类型 | temperature 配置 | 其他参数 | 问题 |
|-----------|-----------------|---------|------|
| SimpleAgent | 仅 builder 支持 | 无 | 参数不完整 |
| ChatAgent | 仅 builder 支持 | 无 | 参数不完整 |
| ToolAgent | 硬编码 0.7 | 无 | 完全不可配置 |
| ReActAgent | 硬编码 0.7 | 无 | 完全不可配置 |
| ReflectAgent | 硬编码 0.7 | 无 | 完全不可配置 |
| Extractor | 硬编码 0.0 | 无 | **需要支持 llm_params**（开源模型可能需要调整参数） |

## 解决方案

### 核心设计: `LlmParams` 类型

在 `agentkit-core/src/provider/types.rs` 中新增统一参数类型：

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub max_tokens: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub response_format: Option<ResponseFormat>,
    pub extra: Option<Value>,
}
```

**关键特性**:
- 所有字段默认 `None`（使用模型默认值）
- Builder 模式支持链式调用
- `apply_to(&self, request: &mut ChatRequest)` 方法合并参数
- `from_request()` 从现有请求提取参数

### 实施步骤

#### 1. 添加 LlmParams 类型

位置: `agentkit-core/src/provider/types.rs`

- 定义结构体和默认值
- 实现 builder 方法
- 实现 `apply_to()` 和 `from_request()`

#### 2. 更新 AgentContext

位置: `agentkit-core/src/agent/mod.rs`

- 移除 `default_chat_request()` 中的硬编码 temperature
- 新增 `default_chat_request_with(&LlmParams)` 方法

#### 3. 更新所有 Agent 类型

每个 Agent 添加:
- `llm_params: LlmParams` 字段
- 9 个 builder 方法（temperature、top_p、top_k、max_tokens 等）
- `_build_chat_request()` 中使用 `context.default_chat_request_with(&self.llm_params)`

#### 4. 更新 Extractor

位置: `agentkit/src/agent/extractor.rs`

Extractor 内部的 `_extract_json_with_usage()` 方法硬编码了 `temperature: Some(0.0)`。
虽然 Extractor 需要确定性输出，但对于能力较弱的开源模型，用户可能需要：
- 调整 temperature（不一定为 0.0，某些模型 0.0 反而效果不好）
- 增加 max_tokens（复杂 schema 可能需要更多输出）
- 调整其他参数以提高提取成功率

因此 Extractor 也需要支持 `llm_params` 配置：
- `Extractor` 结构体添加 `llm_params: LlmParams` 字段
- `ExtractorBuilder` 添加 `llm_params()` 和便捷 builder 方法
- `_extract_json_with_usage()` 中使用 `self.llm_params.apply_to(&mut request)` 替代硬编码

#### 5. 更新 DefaultExecution

位置: `agentkit/src/agent/execution.rs`

- 添加 `llm_params` 字段
- 流式执行路径使用 llm_params 而非硬编码值

#### 6. 更新导出

- `agentkit-core/src/lib.rs`: 导出 `LlmParams`
- `agentkit/src/lib.rs`: 重新导出 `LlmParams`

## 使用示例

```rust
use agentkit::LlmParams;
use agentkit::agent::ToolAgent;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4")
    .system_prompt("你是有用的助手")
    .tool_registry(registry)
    // 方式 1: 使用独立 builder 方法
    .temperature(0.7)
    .top_p(0.9)
    .max_tokens(2048)
    // 方式 2: 使用完整 LlmParams
    .llm_params(
        LlmParams::new()
            .temperature(0.8)
            .top_p(0.95)
            .top_k(50)
            .frequency_penalty(0.1)
            .presence_penalty(0.1)
            .stop(vec!["\n".into()])
    )
    .build();
```

## 设计决策

### 为什么使用 Option<f32> 而非 f32？

- `None` 表示"使用模型默认值"，不发送该参数给 provider
- 提供更灵活的控制：可以只设置需要的参数，其余使用默认值
- 避免覆盖 provider 端的优化默认值

### 为什么不保持向后兼容？

- 用户明确要求"不需要考虑兼容问题，大胆改进保持最优最合理为准"
- 旧代码使用 `.temperature(0.7)` 仍然有效（builder 方法兼容）
- 仅内部字段结构变更，不影响使用体验

### Extractor 为何要改？

原设计认为 Extractor 需要确定性输出（temperature=0.0），不应允许用户修改。
但实际使用中：
- 对于能力较弱的开源模型（如某些本地部署的小模型），temperature=0.0 反而可能导致输出质量下降
- 复杂 schema 可能需要更大的 max_tokens 才能完整输出
- 用户可能需要多次调整参数以提高提取成功率（如调整 top_p、增加重试等）
- 某些模型对 temperature 的实现不同，0.0 不一定是最确定的

因此 Extractor 也应当支持 `llm_params` 配置，让用户根据具体模型调整参数。
默认仍然使用 temperature=0.0（通过 `LlmParams::new().temperature(0.0)`），但用户可自定义。
