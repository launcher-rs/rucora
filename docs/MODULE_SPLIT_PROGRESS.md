# AgentKit 模块拆分进度报告

> **开始日期**: 2026年4月9日  
> **拆分依据**: `docs/MODULE_SPLIT_ANALYSIS.md` 分析报告

---

## 拆分进度总览

| 阶段 | 任务 | 状态 | 完成度 |
|------|------|------|--------|
| 阶段 1 | 拆分 agentkit-mcp | ✅ 完成 | 100% |
| 阶段 1 | 拆分 agentkit-a2a | ✅ 完成 | 100% |
| 阶段 2 | 拆分 agentkit-providers | ⏸️ 待进行 | 0% |
| 阶段 2 | 拆分 agentkit-tools | ⏸️ 待进行 | 0% |
| 阶段 3 | 拆分 agentkit-embed | ⏸️ 待进行 | 0% |
| 阶段 3 | 拆分 agentkit-retrieval | ⏸️ 待进行 | 0% |
| 集成 | 更新 agentkit 主库 | ⏸️ 待进行 | 0% |
| 集成 | 更新 workspace | ⏸️ 待进行 | 0% |
| 验证 | 编译和测试 | ⏸️ 待进行 | 0% |

**总体进度**: 2/9 (22%)

---

## 已完成工作

### ✅ 阶段 1: 协议层拆分

#### 1. agentkit-mcp crate

**创建内容**:
- ✅ 目录结构: `agentkit-mcp/`
- ✅ Cargo.toml: 配置 rmcp 依赖
- ✅ 源代码: 复制 4 个文件 (mod.rs, protocol.rs, tool.rs, transport.rs)

**文件清单**:
```
agentkit-mcp/
├── Cargo.toml
└── src/
    ├── mod.rs        (184 行)
    ├── protocol.rs   (MCP 协议模型)
    ├── tool.rs       (McpToolAdapter)
    └── transport.rs  (StdioTransport, StreamableHttpTransport)
```

**依赖配置**:
```toml
[dependencies]
agentkit-core = { path = "../agentkit-core" }
rmcp = { version = "1.3", features = ["client", "reqwest", "transport-streamable-http-client-reqwest"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
thiserror = "2"
```

---

#### 2. agentkit-a2a crate

**创建内容**:
- ✅ 目录结构: `agentkit-a2a/`
- ✅ Cargo.toml: 配置 ra2a 依赖
- ✅ 源代码: 复制 3 个文件 (mod.rs, protocol.rs, transport.rs)

**文件清单**:
```
agentkit-a2a/
├── Cargo.toml
└── src/
    ├── mod.rs        (A2A 模块导出)
    ├── protocol.rs   (A2A 协议模型)
    └── transport.rs  (A2A 传输层)
```

**依赖配置**:
```toml
[dependencies]
agentkit-core = { path = "../agentkit-core" }
ra2a = { version = "0.9", default-features = false, features = ["client", "server"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
thiserror = "2"
```

---

## 待进行工作

### ⏸️ 阶段 2: 核心模块拆分

#### 3. agentkit-providers crate (预计 2 天)

**计划结构**:
```
agentkit-providers/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── openai.rs
    ├── anthropic.rs
    ├── gemini.rs
    ├── azure_openai.rs
    ├── ollama.rs
    ├── openrouter.rs
    ├── deepseek.rs
    ├── moonshot.rs
    ├── resilient.rs
    └── helpers.rs
```

**Feature 设计**:
```toml
[features]
default = ["openai"]
openai = []
anthropic = []
gemini = []
azure-openai = []
ollama = []
openrouter = []
deepseek = []
moonshot = []
resilient = []
all = ["openai", "anthropic", "gemini", "azure-openai", "ollama", "openrouter", "deepseek", "moonshot", "resilient"]
```

**工作量**: 12 个文件，4,373 行代码

---

#### 4. agentkit-tools crate (预计 2 天)

**计划结构**:
```
agentkit-tools/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── basic/
    ├── system/
    ├── file/
    ├── network/
    ├── browser/
    ├── memory/
    ├── datetime/
    └── github/
```

**Feature 设计**:
```toml
[features]
default = ["basic", "system", "file", "network", "memory"]
basic = []
system = ["tokio/process", "tokio/fs"]
file = ["tokio/fs"]
network = ["reqwest"]
browser = ["network"]
memory = []
datetime = []
github = ["network"]
all = ["basic", "system", "file", "network", "browser", "memory", "datetime", "github"]
```

**工作量**: 16 个文件，3,089 行代码

---

### ⏸️ 阶段 3: 可选模块拆分

#### 5. agentkit-embed crate (预计 1 天)

**计划结构**:
```
agentkit-embed/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── openai.rs
    ├── ollama.rs
    └── cache.rs
```

**工作量**: 4 个文件，464 行代码

---

#### 6. agentkit-retrieval crate (预计 1 天)

**计划结构**:
```
agentkit-retrieval/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── in_memory.rs
    ├── chroma.rs
    ├── chroma_persistent.rs
    └── qdrant.rs
```

**Feature 设计**:
```toml
[features]
default = ["in-memory"]
in-memory = []
chroma = []
qdrant = []
```

**工作量**: 6 个文件，1,271 行代码

---

### ⏸️ 集成工作

#### 7. 更新 agentkit 主库

**需要修改**:
- `agentkit/Cargo.toml`: 添加可选依赖
- `agentkit/src/mcp/mod.rs`: 改为重新导出
- `agentkit/src/a2a/mod.rs`: 改为重新导出
- 其他模块的导出逻辑

**新的依赖配置**:
```toml
[dependencies]
agentkit-core = { path = "../agentkit-core" }
agentkit-mcp = { path = "../agentkit-mcp", optional = true }
agentkit-a2a = { path = "../agentkit-a2a", optional = true }
# 后续添加
# agentkit-providers = { path = "../agentkit-providers", optional = true }
# agentkit-tools = { path = "../agentkit-tools", optional = true }
# agentkit-embed = { path = "../agentkit-embed", optional = true }
# agentkit-retrieval = { path = "../agentkit-retrieval", optional = true }

[features]
default = []
mcp = ["dep:agentkit-mcp"]
a2a = ["dep:agentkit-a2a"]
# 后续添加
# providers = ["dep:agentkit-providers"]
# tools = ["dep:agentkit-tools"]
# embed = ["dep:agentkit-embed"]
# retrieval = ["dep:agentkit-retrieval"]
# all = ["providers", "tools", "mcp", "a2a", "embed", "retrieval"]
```

---

#### 8. 更新 workspace Cargo.toml

**需要添加**:
```toml
[workspace]
resolver = "3"
members = [
    "agentkit",
    "agentkit-core",
    "agentkit-mcp",         # 新增
    "agentkit-a2a",         # 新增
    # 后续添加
    # "agentkit-providers",
    # "agentkit-tools",
    # "agentkit-embed",
    # "agentkit-retrieval",
    "examples/a2a-client",
    "examples/a2a-server",
    "examples/agentkit-skills-example",
    "examples/agentkit-deep-research"
]
```

---

#### 9. 验证编译和测试

**验证清单**:
- [ ] `cargo check --workspace` 编译通过
- [ ] `cargo test --workspace` 所有测试通过
- [ ] `cargo build --workspace` 构建成功
- [ ] 验证 feature 组合:
  - [ ] `cargo check -p agentkit --no-default-features`
  - [ ] `cargo check -p agentkit --features mcp`
  - [ ] `cargo check -p agentkit --features a2a`
  - [ ] `cargo check -p agentkit --all-features`

---

## 下一步计划

### 立即执行
1. 更新 workspace Cargo.toml 添加新 crate
2. 更新 agentkit 主库的 feature 配置
3. 验证 mcp 和 a2a 拆分后编译通过

### 后续执行
1. 拆分 agentkit-providers (阶段 2)
2. 拆分 agentkit-tools (阶段 2)
3. 拆分 agentkit-embed 和 retrieval (阶段 3)
4. 完整测试验证

---

## 预计时间线

| 阶段 | 任务 | 预计时间 | 状态 |
|------|------|----------|------|
| 今天 | 阶段 1 (mcp/a2a) | 已完成 | ✅ |
| 今天 | 更新 workspace 和主库 | 0.5 天 | ⏸️ |
| 明天 | 阶段 2 (providers/tools) | 2 天 | ⏸️ |
| 后天 | 阶段 3 (embed/retrieval) | 1 天 | ⏸️ |
| 第 4 天 | 集成测试和文档 | 1 天 | ⏸️ |

**总计**: 约 4-5 天完成全部拆分

---

**报告生成时间**: 2026年4月9日  
**下次更新**: 完成阶段 2 拆分后
