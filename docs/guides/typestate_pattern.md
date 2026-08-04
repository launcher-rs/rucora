# Typestate 模式科普：让编译器帮你检查 API 使用顺序

> 本文用通俗的语言解释什么是 Typestate 模式，以及 rucora 的 Agent 构建器为什么用它。

## TL;DR

Typestate 模式（类型状态模式）是一种 Rust 编程技巧：**用一个"状态"类型来标记一个对象当前处于哪个阶段**，从而让编译器在编译期（而不是运行时）就禁止非法的操作序列。

rucora 用它在 Agent 构建器上**强制必须先设置 model 才能 build**：

```rust
// 编译错误！builder() 处于「未设置 model」状态，没有 build() 方法
let agent = ToolAgent::builder(provider).build();

// 正确：先 .model(...) 再 .build()
let agent = ToolAgent::builder(provider)
    .model("gpt-4o-mini")
    .build();
```

## 从问题说起

### 一个常见的烦恼

写 rucora 代码时，很多用户会忘记调用 `.model(...)`：

```rust
let agent = ToolAgent::builder(provider)
    .system_prompt("你是有用的助手")
    .build(); // 😱 忘记设置 model 了
```

在没有 Typestate 之前，`build()` 总是可以调用，model 字段只是 `Option<String>`，没设置就偷偷用 Provider 的默认模型。**错误没有被阻止，只是被"温柔地"忽视了**——直到运行时行为不符合预期，你才发现问题。

这就像炒菜忘了放盐：锅铲不会阻止你，只有吃到嘴里才发现。

### 我们希望发生什么

我们想要的是：**如果忘了放盐，锅铲直接罢工，根本不让你把菜盛出来。** 对应到代码里就是：

> 忘记调用 `.model(...)` 时，代码**编译都过不了**。

这就是 Typestate 模式要解决的问题。

## Typestate 的核心思想

### 把"状态"变成"类型"

Typestate 的关键洞察是：**把对象的内部状态编码到类型系统里**。

一个对象处于什么状态，就用不同的类型参数标记它。不同状态拥有不同的方法集合，非法操作在编译期就被拒绝了。

打个比方：

> 一个 `Door` 对象有"开着"和"关着"两个状态。普通的实现里，你可以在门开着时关门、在门关着时开门——这都对。但如果你误以为门是关着的而去开门，编译期它不拦你，运行时才可能出错。Typestate 把"开门"方法只定义在"关着"状态的类型上，"关门"方法只定义在"开着"状态的类型上——**状态用错了，编译器直接报错**。

### 两个状态的构建器

回到 rucora 的 Agent 构建器。我们用两个标记类型表示两个状态：

```rust
/// 状态标记：尚未设置 model
pub struct NoModel;

/// 状态标记：已设置 model
pub struct WithModel;
```

构建器本身多了一个泛型参数 `S`，表示当前状态：

```rust
pub struct ToolAgentBuilder<P, S = NoModel> {
    provider: P,
    model: Option<String>,
    // ... 其他字段
    _marker: std::marker::PhantomData<S>, // 类型标记
}
```

关键设计：

| 方法 | 定义在哪个状态 | 说明 |
|------|----------------|------|
| `builder(provider)` | — | 返回 `ToolAgentBuilder<P, NoModel>` |
| `.model(...)` | 仅 `NoModel` 状态 | 返回 `ToolAgentBuilder<P, WithModel>` |
| `.build()` | 仅 `WithModel` 状态 | 返回真正的 Agent |
| `.system_prompt(...)` 等 | 所有状态 | 任意顺序调用，保持当前状态 |

## 一个简化版示例

让我们从零写一个 Typestate 构建器，感受它的工作原理：

```rust
// 第一步：定义两个状态标记
struct NoModel;
struct WithModel;

// 第二步：构建器带一个状态泛型参数 S
struct ConfigBuilder<S> {
    model: Option<String>,
    temperature: Option<f32>,
    _marker: std::marker::PhantomData<S>,
}

// 第三步：初始状态 = NoModel
impl ConfigBuilder<NoModel> {
    fn new() -> Self {
        Self {
            model: None,
            temperature: None,
            _marker: std::marker::PhantomData,
        }
    }

    // 只有 NoModel 状态能调用 model()，调用后状态变为 WithModel
    fn model(self, model: &str) -> ConfigBuilder<WithModel> {
        ConfigBuilder {
            model: Some(model.to_string()),
            temperature: self.temperature,
            _marker: std::marker::PhantomData,
        }
    }
}

// 第四步：通用方法对所有状态可用，且保持当前状态
impl<S> ConfigBuilder<S> {
    fn temperature(self, value: f32) -> ConfigBuilder<S> {
        ConfigBuilder {
            model: self.model,
            temperature: Some(value),
            _marker: std::marker::PhantomData,
        }
    }
}

// 第五步：build() 只存在于 WithModel 状态
impl ConfigBuilder<WithModel> {
    fn build(self) -> Config {
        Config {
            model: self.model.expect("已保证 Some"),
            temperature: self.temperature,
        }
    }
}

// 最终配置
struct Config {
    model: String,
    temperature: Option<f32>,
}
```

现在使用它：

```rust
// ✅ 编译通过：先 model 再 build
let config = ConfigBuilder::new().model("gpt-4o-mini").build();

// ❌ 编译错误：NoModel 状态没有 build() 方法
let config = ConfigBuilder::new().build();

// ✅ 编译通过：temperature 可以放在 model 之前或之后
let config = ConfigBuilder::new()
    .temperature(0.7)
    .model("gpt-4o-mini")
    .build();
```

> 注意 `PhantomData<S>` 的作用：它告诉编译器"这个类型假装持有 `S` 类型的一个值"。因为状态类型 `NoModel`/`WithModel` 是零大小类型（不占内存），`PhantomData` 让它们参与类型推导，却不产生任何运行时开销。

## 在 rucora 中的真实应用

rucora 的 6 个 Agent 构建器全部采用了这一模式：

| 构建器 | 对应的 Agent |
|--------|--------------|
| `SimpleAgentBuilder` | 简单问答 |
| `ChatAgentBuilder` | 多轮对话 |
| `ToolAgentBuilder` | 工具调用 |
| `ReActAgentBuilder` | 推理 + 行动 |
| `ReflectAgentBuilder` | 反思迭代 |
| `SummaryAgentBuilder` | 文本摘要 |

它们共享 `NoModel`/`WithModel` 两个状态标记（定义在 `rucora::agent` 模块），用法完全一致：

```rust
use rucora::agent::{ChatAgent, NoModel, WithModel};
use rucora::prelude::Agent;

// 返回 NoModel 状态
let builder = ChatAgent::builder(provider);

// 调用 .model() 后变为 WithModel 状态
let builder = builder.model("gpt-4o-mini");

// 只有 WithModel 状态能 build()
let agent = builder.build();
```

甚至可以用类型标注显式检查状态：

```rust
let builder: ChatAgentBuilder<_, NoModel> = ChatAgent::builder(provider);
let builder: ChatAgentBuilder<_, WithModel> = builder.model("gpt-4o-mini");
```

## 常见问题

### Q1: 是不是让 API 更麻烦了？

表面上看多了 `.model()` 必须调用这一步，但这正是目的——**它把"运行时才可能发现的错误"提前到了"编译期"**。少写一行 `.model()` 的"便利"，远不如编译期强制来得安心。

### Q2: 如果我真的想用 Provider 默认模型怎么办？

现在 rucora 强制设置 model，是为了避免"忘记设置"和"故意不设置"无法区分。如果确实想用默认模型，仍可传入 Provider 的默认模型名——API 是明确的、有意的，而不是"忘了写"。

### Q3: Typestate 有性能开销吗？

没有。状态标记类型是零大小类型（ZST），`PhantomData<S>` 也不占内存。**编译期检查，零运行时开销**——这是 Typestate 最迷人的地方。

### Q4: 我能自己实现一个 Typestate 构建器吗？

完全可以。本文的简化示例就是最小可用的实现。关键步骤回顾：

1. 定义状态标记类型（如 `NoModel`、`WithModel`）；
2. 构建器带 `PhantomData<S>` 状态参数；
3. 状态转换方法（如 `.model()`）从旧状态返回新状态；
4. `build()` 只定义在最终状态上。

## 与其他方案的对比

| 方案 | 检查时机 | 优点 | 缺点 |
|------|----------|------|------|
| **Typestate**（当前方案） | 编译期 | 忘记设置直接编译失败，零开销 | 实现稍复杂，API 签名带状态参数 |
| `build() -> Result<Agent, Error>` | 运行时 | 实现简单 | 调用方要处理 Result，忘设 model 运行时才报错 |
| `builder(provider, model)` 必填参数 | 编译期 | 最简单 | 不灵活，无法链式配置；签名固定 |

Typestate 是三者中"最安全"的：既保留了链式配置的灵活性，又让错误在编译期暴露。

## 延伸阅读

- [用户指南](user_guide.md) - rucora 各 Agent 的完整用法
- [Agent 架构](../design/agent_runtime_relationship.md) - 理解 Agent 决策与执行
- [快速参考](QUICK_REFERENCE.md) - API 快速查询
