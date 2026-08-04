# rucora Prompt 设计（简化版 - LangChain 风格）

## 核心思路

1. **类似 LangChain PromptTemplate** - 简单模板语法
2. **单独 crate** - `rucora-prompt` 处理所有 prompt
3. **名称自动引入** - 按名称使用内置 prompt
4. **越小越好** - 核心只有几个类型

---

## 架构

```
rucora-prompt/
├── Cargo.toml
└── src/
    ├── lib.rs          # 导出
    ├── template.rs     # PromptTemplate
    ├── built_in.rs    # 内置 prompt
    └── loader.rs      # 文件加载
```

---

## 核心设计

### 1. 简单结构

```rust
// rucora-prompt/src/lib.rs

mod template;
mod built_in;
mod loader;

pub use template::PromptTemplate;
pub use built_in::BuiltInPrompt;
pub use loader::PromptLoader;

/// 主入口：按名称获取 prompt
pub fn prompt(name: &str) -> PromptTemplate {
    // 先尝试内置
    if let Some(p) = BuiltInPrompt::from_name(name) {
        return p.template();
    }
    // 再尝试加载文件
    if let Some(p) = PromptLoader::load(name) {
        return p;
    }
    // 默认
    BuiltInPrompt::default().template()
}
```

### 2. PromptTemplate 结构

```rust
// rucora-prompt/src/template.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板名称
    pub name: String,
    /// System prompt
    pub system: String,
    /// User prompt 模板
    pub template: String,
}

impl PromptTemplate {
    /// 渲染模板
    pub fn render(&self, vars: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in vars {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }

    /// 快捷方法：渲染并返回完整消息
    pub fn messages(&self, vars: &HashMap<String, String>) -> Vec<Message> {
        vec![
            Message::system(&self.system),
            Message::user(&self.render(vars)),
        ]
    }
}

/// 消息
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self { role: "system".to_string(), content: content.to_string() }
    }
    pub fn user(content: &str) -> Self {
        Self { role: "user".to_string(), content: content.to_string() }
    }
}
```

### 3. 内置 Prompt（核心几类）

```rust
// rucora-prompt/src/built_in.rs

use super::template::PromptTemplate;
use std::collections::HashMap;

/// 内置 Prompt
pub enum BuiltInPrompt {
    // === Agent ===
    Simple,
    Chat,
    Tool,
    ReAct,
    Reflect,
    
    // === Tool ===
    Search,
    Summarize,
    Translate,
    Browse,
    
    // === Research ===
    Research,
    FastResearch,
    Academic,
}

impl BuiltInPrompt {
    /// 按名称获取
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            // Agent
            "simple" | "agent_simple" => Some(Self::Simple),
            "chat" | "agent_chat" => Some(Self::Chat),
            "tool" | "agent_tool" => Some(Self::Tool),
            "react" | "agent_react" => Some(Self::ReAct),
            "reflect" | "agent_reflect" => Some(Self::Reflect),
            
            // Tool
            "search" | "tool_search" => Some(Self::Search),
            "summarize" | "tool_summarize" => Some(Self::Summarize),
            "translate" | "tool_translate" => Some(Self::Translate),
            "browse" | "tool_browse" => Some(Self::Browse),
            
            // Research
            "research" | "deep_research" => Some(Self::Research),
            "fast" | "fast_research" => Some(Self::FastResearch),
            "academic" | "research_academic" => Some(Self::Academic),
            
            _ => None,
        }
    }

    /// 转为模板
    pub fn template(&self) -> PromptTemplate {
        match self {
            Self::Simple => PromptTemplate {
                name: "simple".to_string(),
                system: "你是一个简单直接的助手。请简洁准确地回答问题。".to_string(),
                template: "{{input}}".to_string(),
            },
            Self::Chat => PromptTemplate {
                name: "chat".to_string(),
                system: "你是友好的对话助手。请保持自然的交流风格。".to_string(),
                template: "{{input}}".to_string(),
            },
            Self::Tool => PromptTemplate {
                name: "tool".to_string(),
                system: r#"你可以通过工具完成任务。
1. 理解用户需求
2. 选择合适的工具
3. 正确调用工具
4. 处理返回结果"#.to_string(),
                template: "任务: {{input}}\n可用工具: {{tools}}".to_string(),
            },
            Self::ReAct => PromptTemplate {
                name: "react".to_string(),
                system: r#"遵循 ReAct 模式：思考 -> 行动 -> 观察
每步都要明确你在做什么和为什么"#.to_string(),
                template: "问题: {{input}}\n历史: {{history}}".to_string(),
            },
            Self::Reflect => PromptTemplate {
                name: "reflect".to_string(),
                system: "你会在完成任务后进行反思，评估并改进。".to_string(),
                template: "任务: {{input}}\n结果: {{result}}".to_string(),
            },
            
            // Tool
            Self::Search => PromptTemplate {
                name: "search".to_string(),
                system: "你是一个搜索助手，请生成有效的搜索查询。".to_string(),
                template: "搜索: {{query}}\n目标: {{goal}}".to_string(),
            },
            Self::Summarize => PromptTemplate {
                name: "summarize".to_string(),
                system: "你是一个总结助手，请简洁压缩内容。".to_string(),
                template: "总结以下内容:\n{{content}}\n要求: {{requirements}}".to_string(),
            },
            Self::Translate => PromptTemplate {
                name: "translate".to_string(),
                system: "你是一个翻译助手，请准确翻译。".to_string(),
                template: "翻译: {{content}}\n从 {{from}} 到 {{to}}".to_string(),
            },
            Self::Browse => PromptTemplate {
                name: "browse".to_string(),
                system: "你是一个网页分析助手，请提取关键信息。".to_string(),
                template: "网址: {{url}}\n内容: {{content}}".to_string(),
            },
            
            // Research
            Self::Research => PromptTemplate {
                name: "research".to_string(),
                system: r#"你是专业研究助手。
1. 基于可靠来源
2. 提供有据分析
3. 结构化输出"#.to_string(),
                template: "研究主题: {{topic}}\n已收集: {{collected}}".to_string(),
            },
            Self::FastResearch => PromptTemplate {
                name: "fast_research".to_string(),
                system: "快速研究助手，请简洁回答。".to_string(),
                template: "快速研究: {{topic}}".to_string(),
            },
            Self::Academic => PromptTemplate {
                name: "academic".to_string(),
                system: "你是学术助手，请引用权威来源，保持严谨。".to_string(),
                template: "学术研究: {{topic}}\n参考: {{sources}}".to_string(),
            },
        }
    }
}

impl Default for BuiltInPrompt {
    fn default() -> Self {
        Self::Simple
    }
}
```

### 4. 文件加载

```rust
// rucora-prompt/src/loader.rs

use super::template::PromptTemplate;
use serde::Deserialize;

#[derive(Deserialize)]
struct TomlPrompt {
    name: String,
    system: String,
    template: String,
}

/// 从文件或名称加载
pub fn load(name: &str) -> Option<PromptTemplate> {
    // 尝试作为文件路径
    if name.contains('/') || name.contains('\\') || name.ends_with(".toml") {
        return load_file(name);
    }
    // 尝试内置
    BuiltInPrompt::from_name(name).map(|p| p.template())
}

fn load_file(path: &str) -> Option<PromptTemplate> {
    let content = std::fs::read_to_string(path).ok()?;
    let prompt: TomlPrompt = toml::from_str(&content).ok()?;
    Some(PromptTemplate {
        name: prompt.name,
        system: prompt.system,
        template: prompt.template,
    })
}
```

---

## 使用方式

### 1. 零代码使用内置

```rust
use rucora_prompt::prompt;

// 按名称获取，直接使用
let tmpl = prompt("tool");
let messages = tmpl.render(&hashmap!("input" => "查询天气", "tools" => "search,browse"));
```

### 2. 在 Agent 中使用

```rust
use rucora_prompt::prompt;

// 搜索工具
let search_prompt = prompt("search");
let query = search_prompt.render(&hashmap!("query" => "北京天气", "goal" => "获取天气预报"));

// 研究
let research_prompt = prompt("research");
let messages = research_prompt.messages(&hashmap!(
    "topic" => "人工智能发展",
    "collected" => "已收集10条信息"
));
```

### 3. 自定义文件

```toml
# my_prompt.toml
name = "my_custom"
system = "你是专家助手"
template = "问题: {{question}}\n背景: {{context}}"
```

```rust
// 使用自定义
let tmpl = prompt("my_prompt.toml");
// 或
let tmpl = prompt("my_prompt");
```

---

## 目录结构

```
rucora/
├── Cargo.toml              # workspace
├── rucora-prompt/          # 新建 crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs         # 主入口，prompt() 函数
│       ├── template.rs    # PromptTemplate
│       ├── built_in.rs   # 内置枚举
│       └── loader.rs     # 文件加载
├── rucora-core/
├── rucora/
└── ...
```

---

## 对比 LangChain

| LangChain | rucora-prompt |
|-----------|---------------|
| `PromptTemplate.from_template()` | `prompt(name)` |
| `template.format(**kwargs)` | `tmpl.render(&vars)` |
| `ChatPromptTemplate.from_messages()` | 内置 system+user |
| 内置很多 templates | 内置核心几种 |

---

## 核心 API

```rust
// 一行获取
let tmpl = rucora_prompt::prompt("tool");

// 渲染
let output = tmpl.render(&vars);

// 或获取消息
let messages = tmpl.messages(&vars);
```

简单直接，按名称使用，文件自定义。够简单吗？