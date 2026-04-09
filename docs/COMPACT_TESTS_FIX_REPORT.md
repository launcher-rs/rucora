# Compact 模块测试修复报告

> **修复日期**: 2026年4月9日  
> **修复范围**: 3 个失败的 compact 模块测试

---

## 修复概述

成功修复了所有 3 个失败的 compact 模块测试，现在 **所有 81 个测试全部通过**（72 agentkit + 9 agentkit-core）。

### 修复清单

| # | 测试 | 问题 | 修复方案 | 状态 |
|---|------|------|----------|------|
| 1 | `test_group_messages` | 消息分组逻辑错误 | 修复分组算法 | ✅ |
| 2 | `test_generate_partial_compact_prompt` | 提示词缺少关键字 | 添加"最近的消息" | ✅ |
| 3 | `test_should_compact` | 消息量不足以触发阈值 | 调整测试参数 | ✅ |

---

## 详细修复内容

### 1. ✅ test_group_messages 修复

**问题**:  
测试期望 4 条消息（user-assistant-user-assistant）分成 2 组，但实际只返回 1 组。

**根本原因**:  
`group_messages_by_api_round` 函数的逻辑有误。原逻辑只在"连续 assistant 消息"时才开始新组，但正常的对话轮次是 user → assistant 为一组。

**修复方案**:  
改进分组算法，当遇到以下情况时开始新组：
1. 遇到用户消息且当前组不为空（开始了新的 API 轮次）
2. 遇到连续 assistant 消息（工具调用等情况）

**修改文件**: `agentkit/src/compact/grouping.rs`

**修改前**:
```rust
pub fn group_messages_by_api_round(messages: &[ChatMessage]) -> Vec<Vec<ChatMessage>> {
    let mut groups: Vec<Vec<ChatMessage>> = Vec::new();
    let mut current_group: Vec<ChatMessage> = Vec::new();

    for msg in messages {
        // ❌ 只在 assistant 消息时检查
        if msg.role == Role::Assistant && !current_group.is_empty() {
            if should_start_new_group(&current_group, msg) {
                groups.push(current_group);
                current_group = vec![msg.clone()];
                continue;
            }
        }
        current_group.push(msg.clone());
    }
    // ...
}
```

**修改后**:
```rust
pub fn group_messages_by_api_round(messages: &[ChatMessage]) -> Vec<Vec<ChatMessage>> {
    let mut groups: Vec<Vec<ChatMessage>> = Vec::new();
    let mut current_group: Vec<ChatMessage> = Vec::new();
    let mut last_role: Option<Role> = None;

    for msg in messages {
        // ✅ 当遇到用户消息且当前组不为空时，开始新组
        if msg.role == Role::User && !current_group.is_empty() {
            groups.push(current_group);
            current_group = vec![msg.clone()];
            last_role = Some(msg.role.clone());
            continue;
        }

        // ✅ 连续 assistant 消息时也开始新组
        if msg.role == Role::Assistant && last_role == Some(Role::Assistant) && !current_group.is_empty() {
            if should_start_new_group(&current_group, msg) {
                groups.push(current_group);
                current_group = vec![msg.clone()];
                last_role = Some(msg.role.clone());
                continue;
            }
        }

        current_group.push(msg.clone());
        last_role = Some(msg.role.clone());
    }
    // ...
}
```

**测试结果**:
```
test compact::grouping::tests::test_group_messages ... ok
```

---

### 2. ✅ test_generate_partial_compact_prompt 修复

**问题**:  
测试期望提示词包含"最近的消息"，但实际只包含"最近部分"。

**根本原因**:  
`PARTIAL_COMPACT_PROMPT` 常量中使用了"最近消息"而非"最近的消息"。

**修复方案**:  
更新提示词常量，确保包含"最近的消息"这个短语。

**修改文件**: `agentkit/src/compact/prompt.rs`

**修改前**:
```rust
pub const PARTIAL_COMPACT_PROMPT: &str = r#"...
专注于总结最近消息中讨论、学习和完成的内容。
..."#;
```

**修改后**:
```rust
pub const PARTIAL_COMPACT_PROMPT: &str = r#"...
专注于总结最近的消息中讨论、学习和完成的内容。
..."#;
```

**测试结果**:
```
test compact::prompt::tests::test_generate_partial_compact_prompt ... ok
```

---

### 3. ✅ test_should_compact 修复

**问题**:  
测试添加 2000 条短消息后，`should_compact("gpt-4o")` 返回 false。

**根本原因**:  
- gpt-4o 的上下文窗口是 128,000 tokens
- `auto_compact_buffer_tokens` 默认是 13,000
- 触发阈值 = 128,000 - 13,000 = 115,000 tokens
- 2000 条短消息（"消息 N" 和 "回复 N"）只有约 4000-6000 tokens
- 远低于 115,000 tokens 的阈值

**修复方案**:  
1. 使用较小的 `buffer_tokens`（1000）降低触发阈值
2. 使用较小上下文窗口的模型（gpt-4: 8192 tokens）
3. 增加消息长度和数量以确保超过阈值

**计算**:
- 阈值 = 8192 - 1000 = 7192 tokens
- 每条消息约 50 字符 ≈ 12-13 tokens + 角色开销 ≈ 16 tokens
- 需要约 7192 / 16 ≈ 450 条消息
- 测试使用 500 对消息（1000 条）确保触发

**修改文件**: `agentkit/src/compact/mod.rs`

**修改前**:
```rust
#[test]
fn test_should_compact() {
    let mut manager = ContextManager::new(CompactConfig::default());

    // 添加大量消息以触发压缩
    for i in 0..1000 {
        manager.add_message(ChatMessage::user(&format!("消息 {}", i)));
        manager.add_message(ChatMessage::assistant(&format!("回复 {}", i)));
    }

    assert!(manager.should_compact("gpt-4o"));
}
```

**修改后**:
```rust
#[test]
fn test_should_compact() {
    // 使用较小的 buffer 来触发压缩
    // 使用 gpt-4（8192 上下文窗口）
    // buffer 设置为 1000，所以阈值是 7192 tokens
    let config = CompactConfig::default().with_buffer_tokens(1000);
    let mut manager = ContextManager::new(config);

    // 添加消息直到超过阈值
    // 每条消息约 50 个字符，约 12-13 tokens + 角色开销 3-4 = 约 16 tokens
    // 需要约 7192 / 16 = 450 条消息
    for i in 0..500 {
        manager.add_message(ChatMessage::user(&format!("这是第 {} 条测试消息，包含一些额外的内容来增加 token 数量", i)));
        manager.add_message(ChatMessage::assistant(&format!("这是第 {} 条回复，同样包含一些额外的内容来增加 token 数量", i)));
    }

    assert!(manager.should_compact("gpt-4"));
}
```

**测试结果**:
```
test compact::tests::test_should_compact ... ok
```

---

## 验证结果

### 编译检查

```bash
cargo check --workspace
```

✅ **结果**: 编译成功，无错误，无警告

### 完整测试套件

```bash
cargo test --workspace --lib
```

**结果**: 
- ✅ **72 个 agentkit 测试通过**
- ✅ **9 个 agentkit-core 测试通过**
- ✅ **总计 81 个测试全部通过**
- ✅ **0 个测试失败**

### Compact 模块专项测试

```bash
cargo test -p agentkit compact
```

**结果**: 14 个测试全部通过
- `compact::config::tests::test_default_config` ... ok
- `compact::config::tests::test_threshold_calculation` ... ok
- `compact::config::tests::test_should_compact` ... ok
- `compact::grouping::tests::test_group_messages` ... ok
- `compact::grouping::tests::test_select_groups_to_compact` ... ok
- `compact::grouping::tests::test_groups_to_text` ... ok
- `compact::prompt::tests::test_generate_compact_prompt` ... ok
- `compact::prompt::tests::test_generate_partial_compact_prompt` ... ok
- `compact::tests::test_context_manager_creation` ... ok
- `compact::tests::test_add_message` ... ok
- `compact::tests::test_should_compact` ... ok
- `compact::token_counter::tests::test_estimate` ... ok
- `compact::token_counter::tests::test_context_window_manager` ... ok
- `compact::token_counter::tests::test_is_near_limit` ... ok

---

## 修改文件清单

| 文件 | 修改内容 | 行数变化 |
|------|----------|----------|
| `agentkit/src/compact/grouping.rs` | 修复消息分组算法 | +20 -5 |
| `agentkit/src/compact/prompt.rs` | 更新提示词常量 | +1 -1 |
| `agentkit/src/compact/mod.rs` | 改进测试用例 | +15 -8 |

---

## 技术总结

### 1. 消息分组算法改进

**改进前**: 只在连续 assistant 消息时分组  
**改进后**: 
- 用户消息开始新轮次时分组
- 连续 assistant 消息时也分组
- 正确识别 API 往返边界

### 2. 提示词一致性

确保提示词常量与测试期望一致，避免文字游戏导致测试失败。

### 3. 测试参数调优

测试压缩功能时需要：
- 理解阈值计算公式：`threshold = context_window - buffer_tokens`
- 估算消息的 token 数量
- 选择合适的模型和配置以确保测试可触发

---

## 总结

所有 3 个失败的测试已成功修复，现在 compact 模块的 14 个测试全部通过。整个项目的 81 个测试也全部通过，代码质量得到显著提升。

### 关键收获

1. **算法逻辑要符合业务场景**: 消息分组需要正确识别 API 轮次边界
2. **测试数据要足够**: 压缩测试需要足够的消息量来触发阈值
3. **提示词要一致**: 常量和测试期望需要保持文字一致

---

**修复完成时间**: 2026年4月9日  
**修复人员**: AI Code Assistant  
**验证结果**: ✅ 所有 81 个测试通过
