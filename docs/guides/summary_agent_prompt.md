# SummaryAgent 提示词设计

## 概述

SummaryAgent 使用 **MapReduce** 策略处理长文本。当文本较短（不超过一个分块大小）时一次性完成摘要；当文本较长时，先分块生成局部摘要（Map），再合并为最终结果（Reduce）。

## 决策路径

```
输入文本
    │
    ├── 文本 ≤ 1 个分块 ──→ 使用 prompt_template 直接生成摘要
    │
    └── 文本 > 1 个分块 ──→ Step 0: 使用 chunk_template 逐块生成局部摘要（并发 Map）
                            │
                            └── Step 1: 使用 combine_template 合并局部摘要（Reduce）
```

## 模板与 Mode 的关系

三个模板提供**结构骨架**，`mode` 提供 **`{mode}` 占位符的具体值**。两者是**互补关系**：

```
模板（结构）          mode（风格）
    │                     │
    └───── 拼接 ──────────┘
              │
        最终发给 LLM 的 prompt
```

## 模板详情

| 模板 | 路径 | 阶段 | 占位符 |
|------|------|------|--------|
| `prompt_template` | 短文本一步完成 | 直接摘要 | `{text}`, `{mode}` |
| `chunk_template` | 长文本 Map 阶段 | 逐块摘要 | `{index}`, `{total}`, `{text}`, `{mode}` |
| `combine_template` | 长文本 Reduce 阶段 | 合并摘要 | `{summaries}`, `{total}`, `{mode}` |

## Mode 详解

`mode` 决定 `{mode}` 占位符最终被替换成什么文本。预定义模式有对应的内置指令：

| 模式 | `instruction()` 输出 |
|------|---------------------|
| `Concise` | "请用简洁的 2-3 句话总结以下文本的核心内容。直接给出总结，不要添加额外说明。" |
| `Detailed` | "请详细总结以下文本的内容，保留重要细节、论据和结论。确保覆盖所有关键部分。" |
| `BulletPoints` | "请用要点列表形式总结以下文本的核心内容。每个要点应独立且完整。" |
| `KeyPoints` | "请提取以下文本的关键信息和核心观点。只列出最重要的内容，忽略次要细节。" |
| `Custom("...")` | 直接使用自定义指令文本 |

## 完整示例

以下示例展示各模板如何与 mode 配合：

```rust
let agent = SummaryAgent::builder(provider)
    .model("gpt-4o-mini")
    .mode(SummaryMode::BulletPoints)          // ← 决定 {mode} 的值
    .prompt_template(                         // ← 短文本使用
        "请总结：\n\n{text}\n\n{mode}"
    )
    .chunk_template(                          // ← 长文本 Map 阶段使用
        "第 {index}/{total} 块：\n{text}\n\n{mode}"
    )
    .combine_template(                        // ← 长文本 Reduce 阶段使用
        "以下为各块摘要：\n{summaries}\n\n{mode}"
    )
    .build();
```

对于长文本（>1 块），最终生成的 prompt 大致为：

**Map 阶段（每块）：**
```
第 1/3 块：
这是第一块原文...

请用要点列表形式总结以下文本的核心内容。每个要点应独立且完整。
```

**Reduce 阶段：**
```
以下为各块摘要：
【部分 1】
- 第一个块的要点

【部分 2】
- 第二个块的要点

【部分 3】
- 第三个块的要点

请用要点列表形式总结以下文本的核心内容。每个要点应独立且完整。
```

## 常见问题

### 同时设置了 mode 和模板，哪个优先级更高？

两者不冲突。**模板**控制**说在哪里**（prompt 结构），**mode**控制**说什么风格**（指令内容）。`mode.instruction()` 只是替换模板中的 `{mode}` 占位符。

### 可以不设置模板吗？

可以。三个模板都有合理的默认值，直接使用 `mode` 切换风格即可覆盖大部分场景。只有当你需要自定义 prompt 结构（如增减上下文、调整措辞）时才需要设置模板。

### 自定义模板时需要注意什么？

- `prompt_template` 必须包含 `{text}` 和 `{mode}` 占位符（否则 LLM 得不到原文和指令）
- `chunk_template` 必须包含 `{text}` 和 `{mode}`（`{index}`/`{total}` 可选但推荐保留）
- `combine_template` 必须包含 `{summaries}` 和 `{total}`（`{mode}` 可选）
