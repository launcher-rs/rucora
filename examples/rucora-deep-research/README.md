# rucora 深度研究助手

自动调用工具完成研究并生成报告的深度研究助手。

## 功能特点

- 🔍 **自动研究** - 自动调用搜索、网页抓取等工具
- 📝 **报告生成** - 生成结构化的 Markdown 研究报告
- ⚙️ **配置管理** - 支持多种 LLM Provider 配置
- 🔧 **工具丰富** - Web 搜索、网页抓取、文件写入等
- 💾 **配置持久化** - 配置保存到本地，下次自动加载

## 支持的 Provider

| Provider | 默认模型 | 说明 |
|----------|----------|------|
| OpenAI | gpt-4o-mini | OpenAI 官方 API |
| Anthropic | claude-3-5-sonnet | Claude 系列模型 |
| Google Gemini | gemini-1.5-pro | Google Gemini API |
| Azure OpenAI | gpt-4o | Azure 云服务 |
| OpenRouter | anthropic/claude-3-5-sonnet | 多模型聚合 |
| DeepSeek | deepseek-chat | 深度求索 |
| Moonshot | moonshot-v1-8k | 月之暗面 (Kimi) |
| Ollama | llama2 | 本地部署 |

## 快速开始

### 1. 安装依赖

```bash
cd D:\Desktop\ocr\rucora
```

### 2. 配置环境变量（推荐）

复制示例环境变量文件：

```bash
cp .env.example .env
```

编辑 `.env` 文件，填入你的配置：

```bash
# OpenAI 配置
PROVIDER=OpenAI
OPENAI_API_KEY=sk-your-api-key-here
OPENAI_DEFAULT_MODEL=gpt-4o-mini

# 或使用 NVIDIA
# PROVIDER=NVIDIA (DGX Cloud)
# NVIDIA_API_KEY=nvapi-your-api-key-here
# NVIDIA_DEFAULT_MODEL=nvidia/nemotron-4-340b-instruct

# 或使用 Ollama（本地）
# PROVIDER=Ollama (本地)
# OLLAMA_BASE_URL=http://localhost:11434/v1
# OLLAMA_DEFAULT_MODEL=qwen3.5:9b
```

### 3. 运行示例

```bash
cargo run -p rucora-deep-research
```

程序会自动从 `.env` 文件或环境变量加载配置。

### 4. 交互式配置（可选）

如果没有配置环境变量，程序会启动交互式配置向导：

```
━━━ 配置向导 ━━━

选择 Provider:
  0. OpenAI - gpt-4o-mini
  1. Anthropic - claude-3-5-sonnet
  ...

选择 Provider: 0
输入 API Key: sk-...
输入模型名称 [gpt-4o-mini]: 
输入 Base URL [https://api.openai.com/v1]: 
```

### 5. 输入研究主题

```
━━━ 研究主题 ━━━
请输入您要研究的主题（输入 'q' 退出）：
> 人工智能在医疗领域的应用
```

### 6. 查看研究报告

研究完成后，报告保存到当前目录：

```
✓ 报告已保存到：research_report_人工智能在医疗领域的应用.md
```

## 环境变量

支持多种环境变量配置方式：

### 通用配置（最高优先级）

```bash
# 直接指定配置
export API_KEY=sk-your-key
export MODEL=gpt-4o-mini
export BASE_URL=https://api.openai.com/v1
export PROVIDER=OpenAI
```

### Provider 特定配置

```bash
# OpenAI
export OPENAI_API_KEY=sk-your-key
export OPENAI_DEFAULT_MODEL=gpt-4o-mini
export OPENAI_BASE_URL=https://api.openai.com/v1

# Anthropic
export ANTHROPIC_API_KEY=sk-ant-key
export ANTHROPIC_DEFAULT_MODEL=claude-3-5-sonnet

# NVIDIA
export NVIDIA_API_KEY=nvapi-key
export NVIDIA_DEFAULT_MODEL=nvidia/nemotron-4-340b-instruct
export NVIDIA_BASE_URL=https://integrate.api.nvidia.com/v1

# Ollama
export OLLAMA_BASE_URL=http://localhost:11434/v1
export OLLAMA_DEFAULT_MODEL=qwen3.5:9b
```

### SerpAPI 配置

```bash
export SERPAPI_API_KEYS=key1,key2,key3
# 或
export SERPAPI_API_KEY=single-key
```

### 配置文件

配置保存在 `~/.rucora/config.toml`：

```toml
provider = "OpenAI"
api_key = "sk-..."
model = "gpt-4o-mini"
base_url = "https://api.openai.com/v1"
serpapi_keys = "key1,key2"
```

### 优先级顺序

1. **环境变量** (最高优先级)
2. **配置文件** (`~/.rucora/config.toml`)
3. **交互式配置** (首次运行)

## 工具说明

### 内置工具

| 工具 | 功能 | 说明 |
|------|------|------|
| WebSearchTool | 网络搜索 | 搜索相关信息 |
| WebScraperTool | 网页抓取 | 抓取网页内容 |
| DatetimeTool | 日期时间 | 获取当前时间 |
| FileWriteTool | 文件写入 | 保存研究报告 |
| SerpapiTool | SerpAPI 搜索 | 专业搜索服务 (可选) |

### 工具配置

在配置中可以设置 SerpAPI Keys：

```toml
serpapi_keys = "key1,key2,key3"
```

或使用环境变量：

```bash
export SERPAPI_API_KEYS=key1,key2,key3
```

## 研究报告格式

生成的报告为 Markdown 格式：

```markdown
# 人工智能在医疗领域的应用 深度研究报告

**研究日期**: 2026 年 03 月 31 日
**研究轮数**: 1

## 研究结果

[详细研究内容...]

---
*报告生成时间：2026-03-31 10:30:00*
*本研究报告由 rucora 深度研究助手自动生成*
```

## 配置文件

配置文件保存在 `~/.rucora/config.toml`：

```toml
provider = "OpenAI"
api_key = "sk-..."
model = "gpt-4o-mini"
base_url = "https://api.openai.com/v1"
serpapi_keys = "key1,key2"
```

### 修改配置

重新运行程序，选择"否"重新配置：

```
✓ 已加载现有配置
  Provider: OpenAI
  模型：gpt-4o-mini

是否使用现有配置？ (y/n): n

━━━ 配置向导 ━━━
...
```

## 高级用法

### 自定义研究轮数

修改 `main.rs` 中的研究轮数：

```rust
// 多轮迭代研究
for round in 1..=3 {
    info!("\n━━━ 第 {} 轮研究 ━━━", round);
    // 执行研究...
}
```

### 自定义研究计划

修改研究提示词：

```rust
let research_prompt = format!(
    "请研究以下主题：{}\n\n\
     请从以下几个方面进行研究：\n\
     1. 主题的基本情况和背景\n\
     2. 主要影响因素和相关方\n\
     3. 可能的影响和后果\n\
     4. 未来发展趋势\n\n\
     请提供详细、客观的分析。",
    topic
);
```

### 添加自定义工具

在 `run_research` 函数中添加：

```rust
let mut tools = ToolRegistry::new()
    .register(WebSearchTool::new().with_max_results(5))
    .register(WebScraperTool::new())
    .register(DatetimeTool::new())
    .register(FileWriteTool::new())
    .register(YourCustomTool::new());  // 添加自定义工具
```

## 故障排除

### API Key 无效

```
❌ 研究失败：provider error (authentication): invalid API key

可能的原因：
  1. API Key 无效或过期
  2. Base URL 不正确
  3. Model 名称不正确
  4. 网络连接问题
```

**解决方法**：
1. 检查 API Key 是否正确
2. 确认 Base URL 是否正确
3. 验证模型名称是否正确
4. 检查网络连接

### 模型名称错误

```
❌ 研究失败：model not found
```

**解决方法**：
1. 检查模型名称拼写
2. 确认该模型在您的账户中可用
3. 使用默认模型名称

### SerpAPI 不可用

```
⚠ SerpAPI 工具加载失败
```

**解决方法**：
1. 检查 SerpAPI Keys 是否正确
2. 确认 SerpAPI 账户有效
3. 可以跳过 SerpAPI，使用内置 WebSearchTool

## 依赖

- Rust 1.70+
- Tokio 运行时
- 有效的 LLM Provider API Key
- (可选) SerpAPI Key

## 相关文档

- [rucora 用户指南](../../docs/user_guide.md)
- [Runtime 使用](../../docs/runtime_guide.md)
- [Tools 系统](../../docs/tools_guide.md)

## License

MIT License - See [LICENSE](../../LICENSE) for details.
