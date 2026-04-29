# 使用 cargo-release 一次性发布所有 crate

## 前置准备

### 1. 安装 cargo-release

```bash
cargo install cargo-release
```

### 2. 登录 crates.io

```bash
cargo login
```

会提示输入 crates.io API token。在 https://crates.io/settings/tokens 创建一个新的 token 并粘贴进去。

### 3. 处理 crates.io 镜像替换

如果你的 `~/.cargo/config.toml` 中配置了镜像替换（如 rsproxy），cargo-release 默认会使用镜像而非 crates.io。需要临时配置使用 crates.io 源。

**方案 A：临时覆盖配置（推荐）**

在发布时通过环境变量指定 registry：

```bash
cargo release --registry crates-io
```

**方案 B：临时注释镜像配置**

编辑 `~/.cargo/config.toml`，注释掉 replace-with：

```toml
# [source.crates-io]
# replace-with = 'rsproxy-sparse'
```

发布完成后再恢复。

---

## 发布步骤

### 方法一：使用 cargo-release（推荐）

cargo-release 会自动解析依赖顺序并发布所有 crate。

```bash
# 模拟发布（不实际推送，验证配置是否正确）
cargo release --workspace --dry-run

# 模拟发布（使用 crates.io registry）
cargo release --workspace --registry crates-io
```

**常用参数：**

| 参数 | 说明 |
|------|------|
| `--workspace` | 发布 workspace 中所有 crate |
| `--dry-run` | 模拟发布，不实际推送 |
| `--registry crates-io` | 明确指定使用 crates.io |
| `--allow-branch master` | 允许在 master 分支发布 |
| `--execute` | 确认执行（cargo-release 0.25+ 需要） |
| `major/minor/patch` | 版本号升级类型 |

**完整命令示例：**

```bash
# 发布补丁版本（0.1.0 -> 0.1.1）
cargo release patch --workspace --registry crates-io --allow-branch master --execute

# 发布次版本（0.1.0 -> 0.2.0）
cargo release minor --workspace --registry crates-io --allow-branch master --execute

# 发布主版本（0.1.0 -> 1.0.0）
cargo release major --workspace --registry crates-io --allow-branch master --execute
```

---

### 方法二：手动按顺序发布

如果 cargo-release 不可用，可以手动逐个发布。必须严格按照依赖顺序：

```bash
# 第 1 步：发布核心包（无内部依赖）
cargo publish -p rucora-core --registry crates-io

# 第 2 步：发布依赖核心的子包（可并行）
cargo publish -p rucora-retrieval --registry crates-io
cargo publish -p rucora-embed --registry crates-io
cargo publish -p rucora-providers --registry crates-io
cargo publish -p rucora-tools --registry crates-io
cargo publish -p rucora-mcp --registry crates-io
cargo publish -p rucora-a2a --registry crates-io

# 第 3 步：发布依赖多个包的包
cargo publish -p rucora-skills --registry crates-io

# 第 4 步：发布主聚合包
cargo publish -p rucora --registry crates-io
```

**等待 crates.io 索引更新：**

每个包发布后，crates.io 需要几分钟更新索引。如果下一个包报错说找不到依赖，等待几分钟再试。

---

## 发布前检查清单

- [ ] `cargo clippy --workspace --all-targets --all-features` 零警告
- [ ] `cargo test --workspace --all-features` 全部通过
- [ ] `cargo build --workspace --all-features` 编译成功
- [ ] `git status` 工作区干净
- [ ] 所有 `Cargo.toml` 中版本号正确
- [ ] 所有 `Cargo.toml` 包含 `license`, `description`, `repository`
- [ ] `CHANGELOG.md` 已更新
- [ ] `README.md` 已更新

**一键检查命令：**

```bash
cargo clippy --workspace --all-targets --all-features && \
cargo test --workspace --all-features && \
echo "All checks passed!"
```

---

## 发布后验证

### 验证包已在 crates.io 上

```bash
# 搜索包
cargo search rucora

# 查看特定包信息
cargo search rucora-core
cargo search rucora --limit 10
```

### 验证安装

```bash
# 在空白项目中测试添加依赖
cargo init /tmp/test-rucora
cd /tmp/test-rucora
cargo add rucora
cargo build
```

### 验证文档

发布后约 10-30 分钟，文档会出现在：

- https://docs.rs/rucora
- https://docs.rs/rucora-core
- https://docs.rs/rucora-providers
- https://docs.rs/rucora-tools
- https://docs.rs/rucora-skills
- https://docs.rs/rucora-mcp
- https://docs.rs/rucora-a2a
- https://docs.rs/rucora-embed
- https://docs.rs/rucora-retrieval

---

## 常见问题

### Q: 发布时报错 "no matching package found"

**原因：** 依赖的包还未在 crates.io 上发布。

**解决：** 确保按依赖顺序发布，先发布 `rucora-core`，等待索引更新后再发布其他包。

### Q: 发布时报错 "version already exists"

**原因：** 该版本已发布过。

**解决：** 升级版本号后再发布：

```bash
cargo release patch --workspace --execute
```

### Q: cargo-release 提示 "uncommitted changes"

**原因：** 工作区有未提交的修改。

**解决：**

```bash
git add -A
git commit -m "chore: prepare for release"
cargo release patch --workspace --execute
```

### Q: 某个包发布失败，需要重试

**解决：** 已成功发布的包会跳过，重新运行命令即可：

```bash
cargo release --workspace --registry crates-io --execute
```

### Q: 如何撤销已发布的包？

如果发布出错需要撤销：

```bash
cargo yank --vers 0.1.0 rucora-core
cargo yank --vers 0.1.0 rucora
```

注意：`cargo yank` 只是标记为废弃，已下载的用户仍可使用。不能删除已发布的版本。

---

## 版本发布策略

| 类型 | 命令 | 说明 | 示例 |
|------|------|------|------|
| 补丁版 | `cargo release patch` | Bug 修复，向后兼容 | 0.1.0 → 0.1.1 |
| 次版本 | `cargo release minor` | 新功能，向后兼容 | 0.1.0 → 0.2.0 |
| 主版本 | `cargo release major` | 破坏性变更 | 0.1.0 → 1.0.0 |

**建议：** 首次发布使用 `0.1.0`，稳定后发布 `1.0.0`。
