# rucora 示例代码集合

本目录包含 rucora 各个模块的使用示例和综合应用，帮助你快速上手和深入理解框架。

## 📋 示例列表

### 入门系列（01-05）

| 编号 | 示例名称 | 文件 | 功能 | 难度 |
|------|----------|------|------|------|
| `01` | Hello World | `hello_world.rs` | 最简对话 | ⭐ |
| `02` | Basic Chat | `basic_chat.rs` | 交互式聊天 | ⭐ |
| `03` | Chat with Tools | `chat_with_tools.rs` | 带工具对话 | ⭐⭐ |
| `04` | Extractor | `extractor.rs` | 结构化数据提取 | ⭐⭐ |
| `05` | Conversation | `conversation.rs` | 对话管理 | ⭐⭐ |

### 核心模块（06-10）

| 编号 | 示例名称 | 文件 | 功能 | 难度 |
|------|----------|------|------|------|
| `06` | Memory | `memory.rs` | 记忆系统 | ⭐⭐ |
| `07` | RAG | `rag.rs` | 检索增强生成 | ⭐⭐⭐ |
| `08` | Middleware | `middleware.rs` | 中间件系统 | ⭐⭐⭐ |
| `09` | Prompt | `prompt.rs` | Prompt 模板 | ⭐⭐ |
| `10` | Custom Provider | `custom_provider.rs` | 自定义 Provider | ⭐⭐⭐ |

### 高级特性（11-14）

| 编号 | 示例名称 | 文件 | 功能 | 难度 | Feature |
|------|----------|------|------|------|---------|
| `11` | Resilient Provider | `resilient_provider.rs` | 带重试 Provider | ⭐⭐⭐ | - |
| `12` | MCP | `mcp.rs` | MCP 协议 | ⭐⭐⭐⭐ | `mcp` |
| `13` | A2A | `a2a.rs` | A2A 协议 | ⭐⭐⭐⭐ | `a2a` |
| `14` | Skills | `skills.rs` | 技能系统 | ⭐⭐⭐ | `skills` |

### Agent 类型（15-17）

| 编号 | 示例名称 | 文件 | 功能 | 难度 |
|------|----------|------|------|------|
| `15` | ReAct Agent | `react_agent.rs` | ReAct 模式 | ⭐⭐⭐⭐ |
| `16` | Reflect Agent | `reflect_agent.rs` | 反思迭代 | ⭐⭐⭐⭐ |
| `17` | Supervisor Agent | `supervisor_agent.rs` | 主管模式 | ⭐⭐⭐⭐⭐ |

### 综合应用（18-21）

| 编号 | 示例名称 | 文件 | 功能 | 难度 |
|------|----------|------|------|------|
| `18` | Research Assistant | `research_assistant.rs` | 研究助手 | ⭐⭐⭐⭐⭐ |
| `19` | Code Assistant | `code_assistant.rs` | 代码助手 | ⭐⭐⭐⭐⭐ |
| `21` | Task Decomposition | `task_decomposition.rs` | 任务拆解与综合 | ⭐⭐⭐⭐⭐ |

---

## 🚀 快速开始

### 环境准备

1. **设置 API Key**
```bash
# OpenAI
export OPENAI_API_KEY=sk-your-key

# 或 Ollama（本地）
export OPENAI_BASE_URL=http://localhost:11434
```

2. **运行示例**
```bash
# 运行 Hello World
cargo run --example 01_hello_world

# 运行基础聊天
cargo run --example 02_basic_chat

# 运行带工具聊天
cargo run --example 03_chat_with_tools
```

### 运行带 Feature 的示例

```bash
# MCP 示例
cargo run --example 12_mcp --features mcp

# A2A 示例
cargo run --example 13_a2a --features a2a

# Skills 示例
cargo run --example 14_skills --features skills
```

---

## 📖 示例详解

### 01 Hello World

**文件**: `hello_world.rs`

**作用**: 最简化的 rucora 示例，展示如何快速创建一个能对话的 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 01_hello_world
```

**学习要点**:
- 如何创建 Provider
- 如何创建 SimpleAgent
- 如何运行一次对话

---

### 02 Basic Chat

**文件**: `basic_chat.rs`

**作用**: 基础聊天示例，展示交互式多轮对话。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 02_basic_chat
```

**学习要点**:
- 如何创建 ChatAgent
- 如何管理对话历史
- 交互式输入输出

---

### 03 Chat with Tools

**文件**: `chat_with_tools.rs`

**作用**: 展示如何让 Agent 使用工具完成具体任务。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 03_chat_with_tools
```

**学习要点**:
- 如何注册工具
- ToolAgent 的自动工具调用
- 工具执行循环

---

### 04 Extractor

**文件**: `extractor.rs`

**作用**: 展示如何从非结构化文本中提取结构化数据。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 04_extractor
```

**学习要点**:
- 定义目标结构体
- 创建 Extractor
- 提取结构化数据
- Usage 追踪

---

### 05 Conversation

**文件**: `conversation.rs`

**作用**: 展示如何管理多轮对话历史。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 05_conversation
```

**学习要点**:
- 对话历史管理
- 系统提示词
- 最大消息数限制

---

### 06 Memory

**文件**: `memory.rs`

**作用**: 展示如何使用记忆系统存储和检索信息。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 06_memory
```

**学习要点**:
- 添加记忆
- 语义检索
- 记忆相似度搜索

---

### 07 RAG

**文件**: `rag.rs`

**作用**: 展示 RAG（检索增强生成）的完整流程。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 07_rag
```

**学习要点**:
- RAG 完整流程
- 向量数据库使用
- 文档分块策略

---

### 08 Middleware

**文件**: `middleware.rs`

**作用**: 展示如何使用中间件系统增强 Agent 功能。

**运行**:
```bash
cargo run --example 08_middleware
```

**学习要点**:
- 创建自定义中间件
- 中间件链式调用
- 请求/响应拦截

---

### 09 Prompt

**文件**: `prompt.rs`

**作用**: 展示如何使用 Prompt 模板系统。

**运行**:
```bash
cargo run --example 09_prompt
```

**学习要点**:
- 创建模板
- 变量替换
- 模板组合

---

### 10 Custom Provider

**文件**: `custom_provider.rs`

**作用**: 展示如何实现自定义 LLM Provider。

**运行**:
```bash
cargo run --example 10_custom_provider
```

**学习要点**:
- Provider 接口设计
- 流式聊天实现
- 错误处理

---

### 11 Resilient Provider

**文件**: `resilient_provider.rs`

**作用**: 展示如何使用带重试机制的 Provider。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 11_resilient_provider
```

**学习要点**:
- 重试策略配置
- 错误恢复
- 延迟计算

---

### 12 MCP

**文件**: `mcp.rs`

**作用**: 展示如何集成 MCP（Model Context Protocol）服务器。

**运行**:
```bash
cargo run --example 12_mcp --features mcp
```

**学习要点**:
- MCP 协议理解
- 远程工具调用
- 错误处理

---

### 13 A2A

**文件**: `a2a.rs`

**作用**: 展示 A2A（Agent-to-Agent）协议集成。

**运行**:
```bash
cargo run --example 13_a2a --features a2a
```

**学习要点**:
- A2A 协议
- 多 Agent 协作
- 任务分发

---

### 14 Skills

**文件**: `skills.rs`

**作用**: 展示如何使用技能系统。

**运行**:
```bash
cargo run --example 14_skills --features skills
```

**学习要点**:
- 技能配置
- 技能注册

---

### 15 ReAct Agent

**文件**: `react_agent.rs`

**作用**: 展示 ReAct（Reason + Act）模式的 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 15_react_agent
```

**学习要点**:
- ReAct 模式理解
- 思考 - 行动循环
- 多步推理

---

### 16 Reflect Agent

**文件**: `reflect_agent.rs`

**作用**: 展示反思迭代模式的 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 16_reflect_agent
```

**学习要点**:
- 反思迭代模式
- 自我批评
- 持续改进

---

### 17 Supervisor Agent

**文件**: `supervisor_agent.rs`

**作用**: 展示主管模式的 Agent，协调多个专家 Agent。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 17_supervisor_agent
```

**学习要点**:
- 多 Agent 协作
- 任务分配
- 结果聚合

---

### 18 Research Assistant

**文件**: `research_assistant.rs`

**作用**: 综合示例，创建一个智能研究助手。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 18_research_assistant
```

**学习要点**:
- 模块集成
- 完整应用架构
- 最佳实践

---

### 19 Code Assistant

**文件**: `code_assistant.rs`

**作用**: 综合示例，创建一个代码助手。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 19_code_assistant
```

**学习要点**:
- 专业领域 Agent
- 代码理解
- 质量保证

---

### 21 Task Decomposition

**文件**: `task_decomposition.rs`

**作用**: 展示如何让 AI 拆解复杂问题为子问题，分别回答后再综合总结。

**运行**:
```bash
export OPENAI_API_KEY=sk-your-key
cargo run --example 21_task_decomposition
```

**学习要点**:
- 问题拆解策略
- 子问题独立回答
- 答案综合方法
- 质量评估机制

**架构**:
```
┌─────────────────────────────────────────┐
│           复杂问题输入                   │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│         TaskDecomposer                   │
│    (问题拆解专家)                         │
│  - 分析问题结构                          │
│  - 识别核心要点                          │
│  - 生成独立子问题                        │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│    SubQuestion #1    SubQuestion #2 ... │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│      SubQuestionAnswerer                 │
│    (子问题回答专家)                       │
│  - 独立回答每个子问题                    │
│  - 可使用工具获取信息                    │
│  - 评估答案置信度                        │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│      Answer #1    Answer #2    Answer   │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│       AnswerSynthesizer                  │
│    (答案综合专家)                         │
│  - 整合所有答案                          │
│  - 消除重复和矛盾                        │
│  - 生成结构化总结                        │
│  - 评估整体质量                          │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│         综合总结输出                     │
│    + 质量评分 + 改进建议                 │
└─────────────────────────────────────────┘
```

**适用场景**:
- 复杂技术咨询（需要多角度分析）
- 研究报告生成（全面调研）
- 决策支持（权衡利弊）
- 学习规划（系统性安排）

**演示任务**:
1. 技术调研：如何设计高可用的分布式系统？
2. 学习计划：3 个月内学会 Rust 编程
3. 产品分析：AI 驱动的个人知识管理应用

---

## 📚 学习路径

### 新手路线
1. `01_hello_world` - 了解基本概念
2. `02_basic_chat` - 学习对话管理
3. `03_chat_with_tools` - 学习工具使用
4. `04_extractor` - 学习结构化数据提取
5. `18_research_assistant` - 综合应用

### 开发者路线
1. `10_custom_provider` - 自定义 Provider
2. `08_middleware` - 中间件系统
3. `07_rag` - RAG 系统
4. `15_react_agent` - 自定义 Agent
5. `17_supervisor_agent` - 多 Agent 系统

### 专家路线
1. `12_mcp` - MCP 协议
2. `13_a2a` - A2A 协议
3. `14_skills` - 技能系统
4. `16_reflect_agent` - 高级 Agent 模式
5. 实现自己的 Agent 类型

---

## 🔧 故障排除

### 常见问题

#### 1. 未找到 API Key
```
错误：未设置 API 配置
```
**解决方案**: 设置相应的环境变量

#### 2. 编译错误
```bash
# 清理并重新构建
cargo clean
cargo build --workspace

# 更新依赖
cargo update
```

#### 3. Feature 未启用
```
error: target `12_mcp` in package `rucora` requires the features: `mcp`
```
**解决方案**: 添加 `--features mcp` 参数

---

## 📝 贡献示例

欢迎贡献新的示例！请遵循以下规范：

### 示例结构

```rust
//! 示例名称 - 简短描述
//!
//! ## 运行方法
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example XX_example_name
//! ```

use rucora::agent::ToolAgent;
use rucora::provider::OpenAiProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 你的代码
    Ok(())
}
```

### 提交要求

1. 代码有详细注释
2. 包含运行说明
3. 更新本 README.md
4. 通过 `cargo clippy` 检查

---

*最后更新：2026 年 3 月 31 日*
