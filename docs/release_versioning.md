# 发布与版本管理

本文说明 `rucora` 当前 workspace 统一版本策略下，如何管理版本号并发布到 crates.io。

## 当前现状

仓库目前采用的是 **workspace 统一版本**：

```toml
[workspace.package]
version = "0.1.2"
```

各个可发布 crate 使用：

```toml
version.workspace = true
```

这意味着：

1. `rucora`
2. `rucora-core`
3. `rucora-tools`
4. `rucora-providers`
5. `rucora-mcp`
6. `rucora-a2a`
7. `rucora-skills`
8. `rucora-embed`
9. `rucora-retrieval`

这些 crate 的版本号会一起变化。

而 `examples/*` 里的示例 crate 当前大多是：

```toml
publish = false
```

它们不会发布到 crates.io，不需要纳入发布顺序。

---

## 一、当前策略下如何理解“大版本更新”

### 1. 如果当前还是 `0.x`

现在仓库是 `0.1.2`，还没有到 `1.0.0`。

在 SemVer 里，`0.x` 阶段通常按下面理解：

- `0.1.2 -> 0.1.3`：兼容性小修复
- `0.1.2 -> 0.2.0`：**破坏性变更 / 相当于“大版本升级”**

也就是说，**在 `0.x` 阶段，破坏性更新通常通过提升 minor 来表达**。

所以如果你现在要做一次“有 breaking change 的升级”，通常不是：

```text
0.1.2 -> 1.1.0
```

而是：

```text
0.1.2 -> 0.2.0
```

### 2. 如果以后已经进入 `1.x`

那就按标准 SemVer：

- `1.2.3 -> 1.2.4`：patch
- `1.2.3 -> 1.3.0`：minor
- `1.2.3 -> 2.0.0`：major / breaking

---

## 二、统一版本策略下怎么升级版本

### 小版本 / 补丁发布

例如从：

```toml
version = "0.1.2"
```

升级到：

```toml
version = "0.1.3"
```

只需要修改 workspace 根 `Cargo.toml` 的：

```toml
[workspace.package]
version = "0.1.3"
```

然后更新 `Cargo.lock`，再按顺序发布。

### 破坏性升级（当前 `0.x` 下）

如果要发布 breaking change，应把：

```toml
version = "0.1.2"
```

改为：

```toml
version = "0.2.0"
```

但这里有一个额外动作不能漏：

当前很多 crate 之间的内部依赖应该显式写到当前最小需要版本，例如：

```toml
rucora-core = { path = "../rucora-core", version = "0.1.2" }
```

不要只写成过宽的：

```toml
version = "0.1"
```

因为这会让发布校验阶段解析到 crates.io 上已存在的旧 patch 版本。

因此当 workspace 从 `0.1.2` 升到 `0.2.0` 时，**这些内部依赖约束也必须一起改成当前目标版本线**，否则发布后的 crate 依赖关系会不一致。

也就是说，breaking 升级时至少要做两件事：

1. 修改根 `Cargo.toml` 里的 `workspace.package.version`
2. 批量更新各 crate `Cargo.toml` 中的内部依赖版本约束

例如：

```toml
rucora-core = { path = "../rucora-core", version = "0.2.0" }
rucora-tools = { path = "../rucora-tools", version = "0.2.0" }
rucora-providers = { path = "../rucora-providers", version = "0.2.0" }
```

---

## 三、没改代码的 crate，要不要一起发布

### 结论

**在当前“统一版本”策略下，通常要一起发布。**

原因有两个：

1. 版本号已经统一提升了，crates.io 上必须存在对应的新版本包
2. 很多 crate 之间存在内部依赖版本约束，顶层 crate 升级后，下游 crate 也需要对应的新版本号来保持依赖关系清晰

### 这是不是浪费

不算。

crates.io 并不要求“代码必须变化”才能发新版本，它只要求：

1. 包名存在
2. 版本号未被占用
3. 内容合法

所以即使某个 crate 本身源码完全没变，只要你希望整个 workspace 版本线保持一致，也可以正常重新发布它的新版本。

### 什么时候可以不发

只有在你明确决定：

- 不再坚持统一版本
- 或者该 crate 是 `publish = false`
- 或者该 crate 根本不对外发布

这种情况下才可以不跟着发布。

---

## 四、当前策略的优缺点

### 优点

1. 版本管理简单，整个仓库只有一个对外版本线
2. 用户看到 `rucora-*` crate 时更容易理解兼容关系
3. 文档、tag、发布记录更统一

### 缺点

1. 一个 crate 改动，可能导致一批 crate 都要重新发
2. breaking 升级时，不只改根版本号，还要同步改内部依赖约束
3. 某些没有实际代码变更的 crate 也会被迫出新版本

这就是你现在遇到的第二个问题的根源。

---

## 五、推荐的发布规则

建议把当前仓库的版本策略明确成下面这套规则。

### 规则 1：继续采用统一版本

如果当前主要目标是：

- 管理简单
- 对外版本清晰
- 不想让各 crate 自己独立滚版本

那么继续统一版本是合理的。

### 规则 2：`0.x` 阶段把 breaking change 当作 minor 升级

例如：

- `0.1.2 -> 0.1.3`：非 breaking
- `0.1.2 -> 0.2.0`：breaking

### 规则 3：统一版本升级时，可发布 crate 一起发

即使有些 crate 没改代码，也建议统一发版，保持 crates.io 上的版本集合一致。

### 规则 4：示例 crate 不参与发布

对 `publish = false` 的示例 crate：

- 可以跟着 workspace 版本一起改
- 但不参与 `cargo publish`

---

## 六、推荐的实际发布流程

以下流程适用于你现在的仓库结构。

### 1. 修改版本号

修改根 `Cargo.toml`：

```toml
[workspace.package]
version = "0.1.3"
```

或 breaking 升级时：

```toml
[workspace.package]
version = "0.2.0"
```

### 2. 如果是 breaking 升级，批量修改内部依赖版本

把所有内部依赖里的旧版本约束：

```toml
version = "0.1.2"
```

改成：

```toml
version = "0.2.0"
```

### 3. 更新锁文件并检查

```bash
cargo check --all-targets
cargo test --workspace
```

### 4. 按依赖顺序发布

因为 crates.io 要求依赖先存在，建议按下面顺序发布：

```bash
cargo publish -p rucora-core --registry crates-io
cargo publish -p rucora-tools --registry crates-io
cargo publish -p rucora-providers --registry crates-io
cargo publish -p rucora-mcp --registry crates-io
cargo publish -p rucora-a2a --registry crates-io
cargo publish -p rucora-skills --registry crates-io
cargo publish -p rucora-embed --registry crates-io
cargo publish -p rucora-retrieval --registry crates-io
cargo publish -p rucora --registry crates-io
```

发布前建议先 dry run：

```bash
cargo publish -p rucora-core --registry crates-io --dry-run
```

---

## 七、针对你提到的两个具体问题

### 问题 1：大版本更新如何更新

如果当前仍是 `0.x`：

- breaking change 直接升 `0.(x+1).0`
- 同时修改内部依赖约束里的 `version = "0.x"`

如果将来已经是 `1.x`：

- breaking change 升 `2.0.0`、`3.0.0` 这类 major

### 问题 2：有些 crate 没改代码，要不要更新发布

如果你坚持统一版本管理：

- **建议一起发布**

原因不是“代码有变化”，而是“版本线和依赖关系需要一致”。

如果你不想这样做，就只能切换到“各 crate 独立版本”策略，但管理复杂度会明显增加。

---

## 八、后续可选优化

如果以后你觉得“每次升级都要手改很多内部依赖版本很烦”，可以考虑把内部依赖进一步集中到 workspace 里管理，例如使用 `workspace.dependencies` 做统一声明。

这样做的收益是：

1. 内部依赖版本只需要改一个地方
2. 大版本迁移时不容易漏改
3. 发布规则更容易脚本化

这不是当前发布必须做的事，但非常适合作为后续治理项。

---

## 九、建议的仓库约定

建议把下面这段约定当成仓库规则：

1. 当前仓库采用 workspace 统一版本。
2. `0.x` 阶段，breaking change 通过提升 minor 表示。
3. 每次统一版本升级时，所有可发布 crate 一并发布。
4. `publish = false` 的示例 crate 不参与 crates.io 发布。
5. breaking 升级时，必须同步修改内部依赖版本约束。

如果后面不想继续承担“未改 crate 也要发布”的成本，再考虑改成独立版本策略，而不是在统一版本策略里半统一、半独立。
