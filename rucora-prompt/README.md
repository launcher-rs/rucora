# rucora-prompt

> rucora 的 prompt 模板库，支持多语言、分门别类、易于扩展

## 特性

- **多语言支持**：内置中文和英文 prompt
- **分类组织**：按 Agent、Tool、Research、Filter 分类
- **易于扩展**：通过 TOML 文件添加自定义 prompt
- **灵活加载**：支持内置名称、文件路径、目录搜索

## 快速开始

```rust
use rucora_prompt::prompt;
use std::collections::HashMap;

// 使用内置 prompt
let tmpl = prompt("agent/tool");

// 渲染模板
let mut vars = HashMap::new();
vars.insert("input".to_string(), "你好".to_string());
let output = tmpl.render(&vars);

// 获取消息
let msgs = tmpl.messages(&vars);
```

## 目录结构

```
rucora-prompt/
├── src/
│   ├── lib.rs        # 主入口
│   ├── template.rs   # PromptTemplate
│   ├── built_in.rs  # 内置枚举
│   └── loader.rs    # 文件加载
└── prompts/          # prompt 配置文件
    ├── agent/       # Agent 类
    ├── tool/        # 工具类
    ├── research/    # 研究类
    └── filter/      # 过滤/分类类
```

## 内置 Prompt 清单

### Agent 类

| 名称 | 中文 | 英文 | 描述 |
|------|------|------|------|
| `agent_simple` | 简单问答 | simple | 简单直接回答问题 |
| `agent_tool` | 工具调用 | tool | 使用工具完成任务 |
| `agent_tool_en` | - | tool_en | 英文版工具调用 |

### Tool 类

| 名称 | 中文 | 英文 | 描述 |
|------|------|------|------|
| `tool_search` | 搜索 | search | 搜索查询优化 |
| `tool_summarize` | 总结 | summarize | 内容总结压缩 |
| `tool_translate` | 翻译 | translate | 多语言翻译 |
| `tool_browse` | 浏览 | browse | 网页内容分析 |

### Research 类

| 名称 | 中文 | 英文 | 描述 |
|------|------|------|------|
| `research_default` | 默认研究 | default | 专业研究助手 |
| `research_academic` | 学术研究 | academic | 学术规范研究 |

### Filter 类

| 名称 | 中文 | 英文 | 描述 |
|------|------|------|------|
| `filter_classify` | 分类 | classify | 内容分类 |
| `filter_extract` | 提取 | extract | 信息提取 |

## 使用方式

### 1. 按名称使用

```rust
use rucora_prompt::prompt;

// 使用内置中文 prompt
let tmpl = prompt("agent/tool");

// 使用内置英文 prompt
let tmpl = prompt("agent/tool_en");
```

### 2. 指定语言

```rust
use rucora_prompt::prompt_with_lang;

// 中文
let tmpl = prompt_with_lang("tool_search", "zh");

// 英文
let tmpl = prompt_with_lang("tool_search", "en");
```

### 3. 从文件加载

```rust
// 相对路径
let tmpl = prompt("prompts/agent/tool.toml");

// 绝对路径
let tmpl = prompt("/path/to/my_prompt.toml");
```

### 4. 渲染模板

```rust
use rucora_prompt::prompt;
use std::collections::HashMap;

let tmpl = prompt("tool_summarize");
let mut vars = HashMap::new();
vars.insert("content".to_string(), "要总结的内容...".to_string());
vars.insert("requirements".to_string(), "简洁明了".to_string());

let output = tmpl.render(&vars);
```

### 5. 获取消息

```rust
use rucora_prompt::prompt;
use std::collections::HashMap;

let tmpl = prompt("research_default");
let mut vars = HashMap::new();
vars.insert("topic".to_string(), "人工智能".to_string());
vars.insert("collected".to_string(), "已收集的信息...".to_string());

let msgs = tmpl.messages(&vars);
// msgs[0] 是 system message
// msgs[1] 是 user message
```

## 自定义 Prompt

### TOML 格式

```toml
# my_prompt.toml
name = "my_custom"
description = "我的自定义 prompt"

[system]
content = """你是..."""

[user]
template = """模板: {{variable}}"""
```

### 使用自定义 prompt

```rust
let tmpl = prompt("my_prompt.toml");
```

## 扩展 Prompt

在 `prompts/` 目录下添加新的 `.toml` 文件：

```toml
# prompts/tool/my_tool.toml
name = "my_tool"
description = "我的新工具"

[system]
content = "你是..."

[user]
template = "输入: {{input}}"
```

然后通过 `prompt("tool/my_tool")` 使用。

## 示例

### 示例 1：简单问答

```rust
use rucora_prompt::prompt;

let tmpl = prompt("agent_simple");
let output = tmpl.render(&[
    ("input", "今天天气怎么样?"),
].into_iter().collect());

println!("{}", output);
```

### 示例 2：工具调用

```rust
use rucora_prompt::prompt;
use std::collections::HashMap;

let tmpl = prompt("agent_tool");
let mut vars = HashMap::new();
vars.insert("input".to_string(), "帮我查下北京天气".to_string());
vars.insert("tools".to_string(), "search, browse".to_string());

let msgs = tmpl.messages(&vars);
println!("System: {}", msgs[0].content);
println!("User: {}", msgs[1].content);
```

### 示例 3：内容总结

```rust
use rucora_prompt::prompt;
use std::collections::HashMap;

let tmpl = prompt("tool_summarize");
let mut vars = HashMap::new();
vars.insert("content".to_string(), "这是一篇很长的文章内容...".to_string());
vars.insert("requirements".to_string(), "不超过100字".to_string());

let summary = tmpl.render(&vars);
```

## API 参考

### 核心函数

```rust
// 获取 prompt（默认中文）
pub fn prompt(name: &str) -> PromptTemplate

// 获取指定语言
pub fn prompt_with_lang(name: &str, lang: &str) -> PromptTemplate

// 渲染
pub fn prompt_render(name: &str, vars: &HashMap<String, String>) -> String

// 获取消息
pub fn messages(name: &str, vars: &HashMap<String, String>) -> Vec<Message>
```

### PromptTemplate

```rust
impl PromptTemplate {
    pub fn new(name: &str, system: &str, template: &str) -> Self
    pub fn render(&self, vars: &HashMap<String, String>) -> String
    pub fn messages(&self, vars: &HashMap<String, String>) -> Vec<Message>
}
```

## 依赖

```toml
[dependencies]
rucora-prompt = "0.2.0"
```

启用文件加载功能（默认启用）。

## 许可证

MIT