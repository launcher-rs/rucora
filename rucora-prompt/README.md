# rucora-prompt

> rucora 的 prompt 模板库，通过静态常量提供内置 prompt

## 特性

- **直接访问**：通过模块路径直接访问静态常量
- **多语言支持**：内置中文和英文 prompt
- **分类组织**：按 Agent、Tool、Research、Filter 分类
- **IDE 友好**：支持代码跳转和自动补全

## 快速开始

```rust
use rucora_prompt::prompts;

// 直接使用静态常量
let system = prompts::agent::tool::SYSTEM;
let template = prompts::agent::tool::TEMPLATE;

// 或通过函数获取
let tmpl = prompts::agent::tool::template();
```

## 目录结构

```
rucora-prompt/src/
├── lib.rs           # 主入口
├── template.rs     # PromptTemplate
└── prompts/        # 内置 prompt
    ├── agent/      # Agent 类
    ├── tool/       # 工具类
    ├── research/   # 研究类
    └── filter/     # 过滤/分类类
```

## 使用方式

### 1. 直接访问静态常量

```rust
use rucora_prompt::prompts;

// Agent 类
let system = prompts::agent::tool::SYSTEM;
let template = prompts::agent::tool::TEMPLATE;

// Tool 类
let system = prompts::tool::search::SYSTEM;

// Research 类
let system = prompts::research::default::SYSTEM;

// Filter 类
let system = prompts::filter::classify::SYSTEM;
```

### 2. 使用 template() 函数

```rust
use rucora_prompt::prompts;

let tmpl = prompts::tool::search::template();

// 渲染模板
let mut vars = std::collections::HashMap::new();
vars.insert("query".to_string(), "Python 异步".to_string());
vars.insert("goal".to_string(), "学习最佳实践".to_string());
vars.insert("keywords".to_string(), "".to_string());

let output = tmpl.render(&vars);
```

### 3. 获取消息

```rust
use rucora_prompt::prompts;

let tmpl = prompts::research::default::template();
let mut vars = std::collections::HashMap::new();
vars.insert("topic".to_string(), "人工智能".to_string());
vars.insert("collected".to_string(), "已收集的信息".to_string());

let msgs = tmpl.messages(&vars);
// msgs[0] 是 system message
// msgs[1] 是 user message
```

## 内置 Prompt 清单

### Agent 类

| 模块 | 描述 |
|------|------|
| `prompts::agent::simple` | 简单问答 |
| `prompts::agent::simple_en` | 简单问答 (英文) |
| `prompts::agent::tool` | 工具调用 |
| `prompts::agent::tool_en` | 工具调用 (英文) |

### Tool 类

| 模块 | 描述 |
|------|------|
| `prompts::tool::search` | 搜索查询优化 |
| `prompts::tool::search_en` | 搜索 (英文) |
| `prompts::tool::summarize` | 内容总结 |
| `prompts::tool::summarize_en` | 总结 (英文) |
| `prompts::tool::translate` | 多语言翻译 |
| `prompts::tool::translate_en` | 翻译 (英文) |
| `prompts::tool::browse` | 网页内容分析 |
| `prompts::tool::browse_en` | 浏览 (英文) |

### Research 类

| 模块 | 描述 |
|------|------|
| `prompts::research::default` | 专业研究助手 |
| `prompts::research::default_en` | 研究 (英文) |
| `prompts::research::academic` | 学术规范研究 |
| `prompts::research::academic_en` | 学术研究 (英文) |

### Filter 类

| 模块 | 描述 |
|------|------|
| `prompts::filter::classify` | 内容分类 |
| `prompts::filter::classify_en` | 分类 (英文) |
| `prompts::filter::extract` | 信息提取 |
| `prompts::filter::extract_en` | 提取 (英文) |

## 依赖

```toml
[dependencies]
rucora-prompt = "0.2.0"
```

## 许可证

MIT