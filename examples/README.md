# AgentKit 示例

本目录包含 AgentKit 的使用示例，帮助开发者快速上手。

## 🚀 快速开始

### 运行示例

```bash
# 1. 基础聊天（推荐从这里开始）
cargo run --bin basic-chat

# 2. 工具调用
cargo run --bin tool-calling

# 3. 天气 Agent
cargo run --bin weather-agent

# 4. 文件处理
cargo run --bin file-processor

# 5. 多 Agent 协作
cargo run --bin multi-agent

# 6. 自定义工具
cargo run --bin custom-tool

# 7. Skills 演示
cargo run --bin skills-demo

# 8. 深度研究（独立示例）
cargo run -p agentkit-deep-research
```

## 📚 示例列表

| 示例 | 说明 | 难度 | API Key |
|------|------|------|---------|
| **basic-chat** | 基础对话示例 | ⭐ 简单 | 可选 |
| **tool-calling** | 工具调用示例 | ⭐⭐ 中等 | 可选 |
| **weather-agent** | 天气 Agent | ⭐⭐ 中等 | 不需要 |
| **file-processor** | 文件处理 | ⭐⭐ 中等 | 不需要 |
| **multi-agent** | 多 Agent 协作 | ⭐⭐⭐ 高级 | 不需要 |
| **custom-tool** | 自定义工具 | ⭐⭐ 中等 | 不需要 |
| **skills-demo** | Skills 系统 | ⭐⭐⭐ 高级 | 不需要 |

## 📖 详细说明

### 1. basic-chat - 基础聊天

最简单的对话示例，适合快速上手。

**特点**：
- ✅ 支持 Mock Provider（无需 API Key）
- ✅ 支持真实 OpenAI API
- ✅ 交互式对话

**运行**：
```bash
cargo run --bin basic-chat
```

### 2. tool-calling - 工具调用

展示如何使用各种内置工具。

**工具**：
- EchoTool - 回显工具
- FileReadTool - 文件读取
- GitTool - Git 操作
- ShellTool - 执行命令

**运行**：
```bash
cargo run --bin tool-calling
```

### 3. weather-agent - 天气 Agent

展示 Agent API 的使用。

**运行**：
```bash
cargo run --bin weather-agent
```

### 4. file-processor - 文件处理

展示如何使用文件工具读取、写入和编辑文件。

**功能**：
- ✅ FileReadTool - 读取文件内容
- ✅ FileWriteTool - 写入文件内容
- ✅ 安全限制（文件类型、路径检查）

**运行**：
```bash
cargo run --bin file-processor
```

**输出示例**：
```
=== 文件处理示例 ===

1. 测试 FileReadTool：
   ✓ 读取文件成功
   文件大小：1234 字节
   前 100 字符：[package]
   name = "agentkit-examples"
   ...

2. 测试 FileWriteTool：
   ✓ 写入文件成功
   写入字节数：56
```

### 5. multi-agent - 多 Agent 协作

展示多个专用 Agent 之间如何协作完成复杂任务。

**功能**：
- ✅ TranslatorAgent - 翻译助手
- ✅ SummarizerAgent - 总结助手
- ✅ QualityCheckerAgent - 质量检查助手
- ✅ MultiAgentCoordinator - 协作管理器

**运行**：
```bash
cargo run --bin multi-agent
```

**输出示例**：
```
╔════════════════════════════════════════════════════════╗
║         多 Agent 协作示例                                ║
╚════════════════════════════════════════════════════════╝

创建多 Agent 协作系统:
✓ 已创建 3 个专用 Agent:
  - TranslatorAgent (翻译助手)
  - SummarizerAgent (总结助手)
  - QualityCheckerAgent (质量检查助手)

开始多 Agent 协作流程...

步骤 1/3: 翻译助手处理中...
✓ 翻译完成：Translation: Hello, World!

步骤 2/3: 总结助手处理中...
✓ 总结完成：总结：这是一个演示。

步骤 3/3: 质量检查助手处理中...
✓ 质量检查完成：✓ 质量检查通过
```

### 6. custom-tool - 自定义工具

展示如何实现和注册自定义工具。

**内容**：
- 实现 CalculatorTool
- 定义输入 Schema
- 注册到运行时

**运行**：
```bash
cargo run --bin custom-tool
```

### 7. skills-demo - Skills 系统

展示 Skills 系统的使用。

**功能**：
- ⏳ 从目录加载 Skills
- ⏳ 使用 Rhai 脚本技能
- ⏳ 使用命令模板技能

**运行**：
```bash
cargo run --bin skills-demo
```

**状态**：待实现

## 🔬 高级示例

### 8. agentkit-deep-research - 深度研究

**独立的深度研究示例**，展示 Agent 如何自动调用各种工具完成研究任务并生成完整报告。

**功能**：
- ✅ 自动制定研究计划
- ✅ 调用多种工具收集信息（Shell、Git、文件操作等）
- ✅ 分析整理收集的信息
- ✅ 生成完整的 Markdown 格式研究报告
- ✅ 包含 SWOT 分析、学习路径等

**运行**：
```bash
cargo run -p agentkit-deep-research
```

**输出示例**：
```
╔════════════════════════════════════════════════════════╗
║           深度研究助手                                 ║
╚════════════════════════════════════════════════════════╝

📚 研究主题：Rust 编程语言在系统开发中的应用

【阶段 1/4】制定研究计划
✓ 研究计划已制定

【阶段 2/4】收集信息
✓ 收集到 6 条信息

【阶段 3/4】分析整理
✓ 分析完成

【阶段 4/4】撰写研究报告
✓ 报告完成
✓ 报告已保存到 research_report.md

📄 报告标题：Rust 编程语言在系统开发中的应用深度研究报告
📊 信息来源：6 个
🎯 关键发现：5 条
💡 行动建议：5 条
```

**报告内容**：
- 执行摘要
- 研究背景
- 关键发现
- SWOT 分析
- 发展趋势
- 信息来源
- 结论
- 行动建议
- 学习路径（入门/进阶/高级）

## 🔧 环境配置

### 使用 Mock Provider（默认）

所有示例默认使用 Mock Provider，无需配置。

### 使用真实 API（可选）

```bash
# OpenAI
export OPENAI_API_KEY=sk-xxx

# Anthropic
export ANTHROPIC_API_KEY=sk-ant-xxx

# Google Gemini
export GOOGLE_API_KEY=xxx
```

## 📝 代码结构

```
examples/
├── README.md
├── agentkit-examples/           # 基础示例集合
│   ├── Cargo.toml
│   └── src/
│       ├── utils.rs             # 共享工具模块
│       ├── 01_basic_chat.rs     # 基础聊天 ✅
│       ├── 02_tool_calling.rs   # 工具调用 ✅
│       ├── 03_weather_agent.rs  # 天气 Agent ✅
│       ├── 04_file_processor.rs # 文件处理 ✅
│       ├── 05_multi_agent.rs    # 多 Agent 协作 ✅
│       ├── 06_custom_tool.rs    # 自定义工具 ✅
│       ├── 07_skills_demo.rs    # Skills 系统 ⏳
│       └── 08_deep_research.rs  # 深度研究 ⏳
│
└── agentkit-deep-research/      # 深度研究示例（独立）
    ├── Cargo.toml
    └── src/
        └── main.rs              # 完整的研究助手实现 ✅
```

**图例**：✅ 已完成 | ⏳ 待实现

## 🎯 学习路径

### 初学者
1. 运行 `basic-chat` 了解基本用法
2. 运行 `tool-calling` 学习工具使用
3. 运行 `custom-tool` 实现自定义工具

### 进阶开发者
1. 阅读源码理解架构
2. 实现自己的 Agent 和工具
3. 贡献新的示例

## 💡 提示

1. **Mock Provider**：所有示例都支持 Mock Provider，无需 API Key 即可运行
2. **真实 API**：设置环境变量后可使用真实 AI 服务
3. **代码修改**：欢迎修改示例代码尝试不同配置
4. **问题反馈**：遇到问题请查看 GitHub Issues

## 🤝 贡献

欢迎贡献新的示例！请遵循：

1. 在 `src/` 目录创建新文件
2. 使用清晰的命名（如 `08_xxx.rs`）
3. 添加详细的中文注释
4. 确保可编译和运行
5. 更新本 README

## 📄 许可证

与主项目相同。
