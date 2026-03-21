# AgentKit 待修复问题清单

## 🔴 高优先级问题（阻止编译）

### 1. API 不兼容导致的大面积编译错误

**影响范围**: 6 个文件，38 处错误

**问题**: `AgentInput` 和 `AgentOutput` 重构后，示例代码和测试代码未更新

**需要修复的文件**:

| 文件 | 错误数 | 主要问题 |
|------|--------|---------|
| `agentkit/examples/agent_loop_demo.rs` | 7 | `AgentInput` 构造使用旧字段 |
| `agentkit/examples/skill_read_local_file_demo.rs` | 3 | `AgentInput` 构造 + `output.message` 访问 |
| `agentkit/examples/rhai_skill_demo.rs` | 3 | 缺少 `RhaiEngineRegistrar` 类型 |
| `agentkit-runtime/tests/tool_calling_agent.rs` | 18 | `AgentInput`/`AgentOutput` 全面不兼容 |
| `agentkit/tests/skills_ecosystem.rs` | 7 | `testkit` 模块未导出 |
| `agentkit/src/middleware.rs` | 8 | 测试代码使用旧 API |

**修复方案**:

```rust
// ❌ 旧 API (需要替换)
let input = AgentInput {
    messages: vec![ChatMessage::user("你好")],
    metadata: None,
};
let content = output.message.content;

// ✅ 新 API (应该使用)
let input = AgentInput::new("你好");
// 或
let input = AgentInput::builder("你好").build();

let content = output.value.get("content")
    .and_then(|v| v.as_str())
    .unwrap_or("无回复");
```

**预计工作量**: 2-3 小时

---

### 2. git 子模块未初始化

**错误**:
```
error: couldn't read `...\out\a2a.v1.rs`: 系统找不到指定的文件。
```

**修复命令**:
```bash
cd C:\code\agentkit
git submodule update --init --recursive
```

**预计工作量**: 5 分钟

---

### 3. Skills 模块导出问题

**问题**: `testkit` 模块未正确导出

**修复位置**:
- `agentkit-skills/src/lib.rs`
- `agentkit/src/lib.rs`

**修复方案**:
```rust
// agentkit-skills/src/lib.rs
#[cfg(test)]
pub mod testkit;

// agentkit/src/lib.rs (已有，无需修改)
#[cfg(feature = "skills")]
pub use agentkit_skills as skills;
```

**预计工作量**: 30 分钟

---

### 4. Rhai Skill API 缺失

**问题**: 示例代码引用不存在的 API

**需要添加**:
1. `RhaiEngineRegistrar` 类型（已在 `agentkit-skills` 中定义）
2. `load_skills_from_dir_with_rhai` 函数（已在 `agentkit-skills/registry.rs` 中定义）

**修复方案**: 在 `agentkit-skills/src/lib.rs` 中重新导出

```rust
#[cfg(feature = "rhai-skills")]
pub use registry::load_skills_from_dir_with_rhai;
```

**预计工作量**: 30 分钟

---

## 🟡 中优先级问题（建议修复）

### 5. Clippy 文档链接警告 (19 个)

**问题**: intra-doc 链接格式不正确

**示例**:
```rust
// ❌ 当前
//! - [`channel::types::ChannelEvent`]: 统一事件类型

// ✅ 应该改为
//! - [`channel::types::ChannelEvent`][]: 统一事件类型
```

**影响文件**: `agentkit-core/src/lib.rs`

**预计工作量**: 30 分钟

---

### 6. 测试代码类型推断失败

**问题**: 缺少类型注解

**示例**:
```rust
// ❌ 当前
let dir = unique_temp_dir("skills-stdlib");

// ✅ 应该改为
let dir: PathBuf = unique_temp_dir("skills-stdlib");
```

**影响文件**: `agentkit/tests/skills_ecosystem.rs`

**预计工作量**: 30 分钟

---

### 7. 断言比较类型不匹配

**问题**: `assert_eq!` 两边类型不一致

**示例**:
```rust
// ❌ 当前
assert_eq!(output, &json!({...}));

// ✅ 应该改为
assert_eq!(&output, &json!({...}));
```

**影响文件**: `agentkit-runtime/tests/tool_calling_agent.rs`

**预计工作量**: 15 分钟

---

### 8. 未使用的导入

**影响文件**:
- `agentkit/examples/skill_read_local_file_demo.rs`

**修复**: 删除未使用的 import

**预计工作量**: 5 分钟

---

### 9. 中间件测试使用旧 API

**影响文件**: `agentkit/src/middleware.rs` (行 369-420)

**预计工作量**: 30 分钟

---

## 🟢 低优先级问题（可选优化）

### 10. 架构设计讨论

**问题**: Agent 与 Runtime 职责边界

**建议**: 待讨论

**预计工作量**: N/A

---

### 11. 工具来源管理复杂性

**问题**: `ToolRegistry` 支持过多来源

**建议**: 评估必要性

**预计工作量**: N/A

---

### 12. 性能优化

**建议**:
- 使用 `Arc` 共享大数据
- 减少不必要的 `clone()`

**预计工作量**: 2 小时

---

### 13. 文档完善

**建议**:
- 为 `AgentInputBuilder` 添加完整示例
- 为 `ToolCallRecord` 添加文档

**预计工作量**: 1 小时

---

### 14. 测试覆盖提升

**建议**:
- 添加 `DefaultAgent::think()` 测试
- 添加边界条件测试
- 目标：覆盖率 >80%

**预计工作量**: 4 小时

---

## 修复计划

### Phase 1 (立即 - 今天完成)
- [ ] 问题 2: 初始化 git 子模块 (5 分钟)
- [ ] 问题 3: 修复 Skills 模块导出 (30 分钟)
- [ ] 问题 4: 添加 Rhai Skill API (30 分钟)

### Phase 2 (本周内)
- [ ] 问题 1: 修复 API 不兼容 (2-3 小时)
- [ ] 问题 5: 修复 Clippy 警告 (30 分钟)
- [ ] 问题 6-8: 修复测试问题 (1 小时)
- [ ] 问题 9: 修复中间件测试 (30 分钟)

### Phase 3 (下次迭代)
- [ ] 问题 10-11: 架构讨论
- [ ] 问题 12: 性能优化
- [ ] 问题 13: 文档完善
- [ ] 问题 14: 测试覆盖提升

---

## 总体评估

### 当前状态
- ✅ 核心功能完整
- ✅ 架构设计清晰
- ✅ 主库编译通过（0 警告）
- ❌ 示例和测试代码未更新

### 预计总工作量
- **高优先级**: 3-4 小时
- **中优先级**: 2 小时
- **低优先级**: 7 小时
- **总计**: 12-13 小时

### 建议
1. **立即修复高优先级问题** - 确保项目可编译
2. **本周内完成中优先级** - 提升代码质量
3. **下次迭代处理低优先级** - 持续改进

---

**创建时间**: 2026-03-22  
**最后更新**: 2026-03-22  
**状态**: 待修复
