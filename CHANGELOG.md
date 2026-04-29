# 变更日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [0.1.2] - 2026-04-29

### API 改进

**简化 Extractor 泛型用法**

- `Extractor<P, T>` 调整为对外只暴露目标类型 `T`
- 创建方式从 `Extractor::<_, T>::builder(...)` 简化为 `Extractor::<T>::builder(...)`
- 为 `Box<T>` / `Arc<T>` 补充 `LlmProvider` 转发实现，支持内部类型擦除

### 行为修复

**修复对话历史与 Agent 构建行为不一致的问题**

- 统一 `ChatAgent`、`ToolAgent`、`ReActAgent`、`ReflectAgent` 的 `with_conversation(true)` 语义，避免调用顺序影响最终行为
- 修复启用 `ConversationManager` 时 system prompt 可能重复注入的问题
- 为各类 Agent builder 增加 `try_build()`，提供显式错误返回入口

**补全 ConversationManager token 限制**

- 实现 `with_max_tokens(...)` 的实际裁剪逻辑
- 修复 `clear()`、`from_json()`、系统提示词注入后的 token 计数
- 新增对应回归测试，确保 token 限制真实生效

**统一工具构造错误边界**

- `SerpapiTool::with_keys(...)`
- `TavilyTool::with_keys(...)`
- `rucora_tools::web::search` 中同类 `with_keys(...)`

以上接口改为返回 `Result`，空 key 不再直接 panic

**修复 Extractor 重试错误语义**

- `ExtractionError::MaxRetriesExceeded` 现在会在配置重试且所有尝试失败时真实返回

### 文档更新

**同步文档到当前 API 行为**

- 重写自动对话历史文档，纠正过时的 builder 示例
- 补充发布与版本管理文档，说明 workspace 统一版本下的 crates.io 发布策略
- 修正文档中漏写 `model(...)` 或仍使用旧接口的示例

---

## [0.1.1] - 2026-04-29

### Extractor 修复

**修复 Extractor 无法正确提取结构化数据的问题**

- 修复 Ollama provider 解析 tool_calls 时 arguments 格式兼容问题 — Ollama 返回的 arguments 为 JSON 对象而非 JSON 字符串，现同时兼容两种格式
- 适配 schemars 1.2 API 变更 — `schema_for!` 返回的 Schema 直接实现 `Serialize`，移除对已废弃的 `.schema` 字段的访问

**修改文件**:
- `rucora-providers/src/ollama.rs`
- `rucora/src/agent/extractor.rs`

---

## [0.1.0] - 2026-04-28

### 概述

rucora 首个公开版本。一个高性能、类型安全的 LLM Agent 框架，支持多 Provider、多工具、多协议。

### 核心特性

- **多 Provider 支持** — OpenAI、Anthropic、Gemini、Ollama、DeepSeek、Moonshot、OpenRouter、Azure OpenAI
- **5 种 Agent 类型** — SimpleAgent、ChatAgent、ToolAgent、ReActAgent、ReflectAgent
- **统一 LLM 参数配置** — `LlmParams` 类型支持 temperature、top_p、max_tokens 等参数
- **Extractor 结构化数据提取** — 基于 tool calling 的 JSON 数据提取
- **20+ 内置工具** — 文件操作、Shell、HTTP、Web 爬取、搜索、数学计算等
- **技能系统** — YAML 定义的可复用技能模板
- **MCP / A2A 协议支持** — Model Context Protocol 和 Agent-to-Agent 协议
- **高级内存系统** — 命名空间隔离、重要性评分、GDPR 合规
- **上下文压缩** — 分层压缩引擎，支持 Aggressive/Balanced/Conservative 策略
- **循环检测** — 防止 Agent 陷入无限循环
- **错误分类器** — 14 种精细错误原因分类，自动判断重试/回退/压缩策略
- **Prompt 注入防护** — 8 种威胁类型检测

### 架构模块

| 模块 | 职责 |
|------|------|
| `rucora-core` | 核心抽象层（traits/types） |
| `rucora` | 主库（实现聚合） |
| `rucora-providers` | LLM Provider 实现 |
| `rucora-tools` | 工具实现 |
| `rucora-mcp` | MCP 协议支持 |
| `rucora-a2a` | A2A 协议支持 |
| `rucora-skills` | 技能系统 |
| `rucora-embed` | Embedding 支持 |
| `rucora-retrieval` | 向量存储 |

---

## 贡献

欢迎贡献代码、文档或反馈问题！

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。
