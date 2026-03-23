# AgentKit 示例

本目录包含 AgentKit 的使用示例，帮助开发者快速上手。

## 🚀 快速开始

### 运行示例

```bash
# 1. 基础聊天（推荐从这里开始）
cargo run --bin basic-chat

# 2. 工具调用
cargo run --bin tool-calling

# 3. 天气 Agent（展示新 API）
cargo run --bin weather-agent

# 4. 自定义工具
cargo run --bin custom-tool
```

## 📚 示例列表

| 示例 | 说明 | 难度 | API Key |
|------|------|------|---------|
| **basic-chat** | 基础对话示例 | ⭐ 简单 | 可选 |
| **tool-calling** | 工具调用示例 | ⭐⭐ 中等 | 可选 |
| **weather-agent** | 天气 Agent（新 API） | ⭐⭐ 中等 | 可选 |
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

展示新的 Agent 配置 API。

**新功能**：
- ✅ 多个 MCP 服务器配置
- ✅ 多个 Skills 目录配置
- ✅ 多个 A2A 代理配置
- ✅ Token 认证支持

**运行**：
```bash
cargo run --bin weather-agent
```

### 4. custom-tool - 自定义工具

展示如何实现和注册自定义工具。

**内容**：
- 实现 CalculatorTool
- 定义输入 Schema
- 注册到运行时

**运行**：
```bash
cargo run --bin custom-tool
```

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
└── agentkit-examples/
    ├── Cargo.toml
    └── src/
        ├── 01_basic_chat.rs      # 基础聊天
        ├── 02_tool_calling.rs    # 工具调用
        ├── 03_weather_agent.rs   # 天气 Agent
        ├── 04_file_processor.rs  # 文件处理（待实现）
        ├── 05_multi_agent.rs     # 多 Agent（待实现）
        ├── 06_custom_tool.rs     # 自定义工具
        └── 07_skills_demo.rs     # Skills 系统（待实现）
```

## 🎯 学习路径

### 初学者
1. 运行 `basic-chat` 了解基本用法
2. 运行 `tool-calling` 学习工具使用
3. 运行 `custom-tool` 实现自定义工具

### 进阶开发者
1. 运行 `weather-agent` 学习新 API
2. 阅读源码理解架构
3. 实现自己的 Agent 和工具

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
