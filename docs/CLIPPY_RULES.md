# AgentKit Clippy 代码规范指南

> **生效日期**: 2026年4月9日  
> **适用范围**: 整个 AgentKit 项目（workspace 所有 crate）

---

## 配置文件说明

### 1. clippy.toml

项目根目录的 `clippy.toml` 文件配置了 Clippy 的全局行为：

```toml
# 代码复杂度限制
cognitive-complexity-threshold = 15        # 函数认知复杂度上限
type-complexity-threshold = 250            # 函数类型复杂度上限

# 文档规范
missing-docs-in-crate-items = true         # 公开项必须有文档注释
doc-valid-idents = [...]                   # 文档中允许的特殊标识词

# 命名规范
upper-case-acronyms-aggressive = false     # 遵循 Rust 命名约定

# 代码风格
single-char-binding-names-threshold = 3    # 单字符变量名允许数量
too-many-arguments-threshold = 7           # 函数参数过多阈值
```

### 2. Cargo.toml Workspace Lints

在根 `Cargo.toml` 中定义了 workspace 级别的 lint 规则：

```toml
[workspace.lints.clippy]
# deny: 必须修复，违反将导致编译失败
# warn:  建议修复，显示警告但不阻止编译
# allow: 允许的特殊情况
```

所有子 crate 通过 `[lints] workspace = true` 继承这些规则。

---

## Lint 规则分类

### 🔴 Deny 规则（必须修复）

违反这些规则将导致**编译失败**。

#### 错误处理

| 规则 | 说明 | 修复方法 |
|------|------|----------|
| `unnecessary_wraps` | 不需要返回 Result 的函数不应返回 Result | 移除 `Result` 包装 |
| `result_unit_err` | Result<(), String> 应使用更具体的错误类型 | 使用自定义错误类型 |

#### 代码质量

| 规则 | 说明 | 修复方法 |
|------|------|----------|
| `clone_on_copy` | Copy 类型不应使用 clone() | 直接复制值 |
| `needless_borrow` | 不必要的借用 | 移除 `&` 或 `&mut` |
| `needless_borrowed_reference` | 不必要的借用引用 | 直接使用值 |
| `redundant_clone` | 冗余的 clone 调用 | 移除多余的 clone() |
| `redundant_field_names` | 冗余的字段名（如 x: x） | 简写为 `x` |
| `single_match` | 单分支 match 应使用 if let | 改用 `if let` |
| `unused_async` | 未使用 async 的 async 函数 | 移除 `async` |
| `manual_strip` | 手动实现 strip_prefix/suffix | 使用标准库方法 |
| `manual_let_else` | 应使用 let-else 模式 | 改用 `let ... else` |

#### 性能

| 规则 | 说明 | 修复方法 |
|------|------|----------|
| `iter_overeager_cloned` | 迭代器中不必要的 cloned() | 移除 cloned() |
| `needless_collect` | 不必要的 collect() | 直接使用迭代器 |
| `uninlined_format_args` | 应使用内联格式化参数 | `format!("{x}")` 替代 `format!("{}", x)` |

#### 可读性

| 规则 | 说明 | 修复方法 |
|------|------|----------|
| `explicit_iter_loop` | 显式的 iter() 调用 | `for x in &vec` 替代 `for x in vec.iter()` |
| `explicit_into_iter_loop` | 显式的 into_iter() 调用 | `for x in vec` 替代 `for x in vec.into_iter()` |
| `needless_range_loop` | 不必要的范围循环 | 使用 `enumerate()` 或直接迭代 |
| `same_item_push` | 重复推送相同项 | 使用 `vec![item; count]` |
| `manual_flatten` | 手动展平 Option/Result | 使用 `for x in iter.flatten()` |

---

### 🟡 Warn 规则（建议修复）

违反这些规则会显示**警告**，但不阻止编译。

#### 代码复杂度

| 规则 | 说明 | 建议 |
|------|------|------|
| `cognitive_complexity` | 认知复杂度过高（>15） | 拆分为多个小函数 |
| `too_many_arguments` | 函数参数过多（>7） | 使用结构体或 Builder 模式 |
| `type_complexity` | 类型过于复杂（>250） | 使用类型别名 |

#### 文档

| 规则 | 说明 | 建议 |
|------|------|------|
| `missing_errors_doc` | 缺少 # Errors 文档 | 添加 `/// # Errors` 段落 |
| `missing_panics_doc` | 缺少 # Panics 文档 | 添加 `/// # Panics` 段落 |

#### 代码风格

| 规则 | 说明 | 建议 |
|------|------|------|
| `enum_variant_names` | 枚举变体命名不规范 | 遵循 Rust 命名约定 |
| `wrong_self_convention` | 错误的 self 约定 | `is_*` 用 `&self`, `into_*` 用 `self` |
| `needless_pass_by_value` | 可按引用传递却按值传递 | 改为 `&T` 或 `&mut T` |
| `map_unwrap_or` | map().unwrap_or() 可简化 | 使用 `map_or()` |
| `or_fun_call` | or() 中的函数调用可优化 | 使用 `or_else()` |

#### 最佳实践

| 规则 | 说明 | 建议 |
|------|------|------|
| `let_and_return` | 不必要的 let 和 return | 直接返回表达式 |
| `needless_return` | 不必要的 return | 移除 `return` 关键字 |
| `explicit_counter_loop` | 显式计数器循环 | 使用 `enumerate()` |
| `filter_map_identity` | filter_map 中的恒等函数 | 使用 `flatten()` |

---

## 使用指南

### 本地开发

```bash
# 运行 clippy 检查
cargo clippy --workspace

# 自动修复可修复的问题
cargo clippy --fix --allow-dirty --allow-staged --workspace

# 查看所有警告
cargo clippy --workspace -- -W clippy::all
```

### CI/CD 集成

建议在 CI 中添加以下检查：

```yaml
# GitHub Actions 示例
- name: Clippy Check
  run: |
    cargo clippy --workspace -- -D warnings
```

### 特殊情况处理

对于某些确实需要违反规则的场景，使用 `#[allow(...)]` 注解：

```rust
// 允许特定的 lint
#[allow(clippy::too_many_arguments)]
fn complex_function(
    arg1: Type1,
    arg2: Type2,
    // ... 更多参数
) {
    // ...
}

// 在模块级别允许
#[allow(clippy::missing_errors_doc)]
mod test_helpers {
    // ...
}
```

---

## 示例：修复前后对比

### 示例 1: uninlined_format_args

**修复前**:
```rust
println!("Hello, {}!", name);
```

**修复后**:
```rust
println!("Hello, {name}!");
```

### 示例 2: needless_range_loop

**修复前**:
```rust
for i in 0..items.len() {
    println!("{}: {}", i, items[i]);
}
```

**修复后**:
```rust
for (i, item) in items.iter().enumerate() {
    println!("{i}: {item}");
}
```

### 示例 3: redundant_field_names

**修复前**:
```rust
let point = Point { x: x, y: y };
```

**修复后**:
```rust
let point = Point { x, y };
```

### 示例 4: single_match

**修复前**:
```rust
match value {
    Some(x) => println!("{x}"),
    None => {}
}
```

**修复后**:
```rust
if let Some(x) = value {
    println!("{x}");
}
```

---

## 维护建议

### 定期审查

1. **每周运行** `cargo clippy --workspace` 检查新问题
2. **PR 审查时**关注 lint 警告
3. **季度审查** clippy.toml 配置是否需调整

### 规则调整

随着项目发展，可调整规则阈值：

```toml
# 如果团队经验提升，可降低复杂度阈值
cognitive-complexity-threshold = 10  # 从 15 降低到 10

# 如果某些规则过于严格，可降级为 warn
# 在 Cargo.toml 中:
[workspace.lints.clippy]
cognitive_complexity = "warn"  # 从默认的 deny 降级
```

---

## 常见问题

### Q: 为什么某些规则设为 deny 而不是 warn？

**A**: Deny 规则通常是明显的代码质量问题，修复成本低但收益高。强制修复可确保代码库保持高质量。

### Q: 测试代码也需要遵守这些规则吗？

**A**: 是的，但测试代码可通过 `#[allow(...)]` 灵活处理某些规则（如 `unwrap()` 使用）。

### Q: 如何暂时忽略某个文件的 lint 检查？

**A**: 在文件顶部添加：
```rust
#![allow(clippy::all)]
```

---

**文档版本**: 1.0  
**最后更新**: 2026年4月9日  
**维护人员**: AgentKit 开发团队
