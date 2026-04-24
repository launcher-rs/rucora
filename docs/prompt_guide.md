# 提示模板系统指南

AgentKit 提供了轻量级的自定义提示模板引擎，支持变量替换、条件渲染和循环渲染。

## 概述

提示模板系统位于 `agentkit::prompt` 模块，是一个不依赖外部模板库（如 Jinja2、Handlebars、Tera）的轻量级引擎。使用 `serde_json::Value` 作为上下文数据格式，`regex` crate 处理条件和循环块。

## 核心类型

### PromptTemplate

```rust
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    template: String,           // 模板内容
    name: Option<String>,       // 模板名称（可选）
    variables: Vec<String>,     // 变量列表（自动提取）
}
```

**方法：**

| 方法 | 签名 | 用途 |
|------|------|------|
| `from_string` | `fn from_string(template: impl Into<String>) -> Self` | 从字符串创建，自动提取变量 |
| `from_file` | `fn from_file(path: &Path) -> Result<Self, PromptError>` | 从文件加载模板 |
| `with_name` | `fn with_name(mut self, name: impl Into<String>) -> Self` | 设置模板名称 |
| `variables` | `fn variables(&self) -> &[String]` | 返回自动提取的变量名 |
| `render` | `fn render(&self, context: &Value) -> Result<String, PromptError>` | 渲染模板（带转义，安全） |
| `render_unescaped` | `fn render_unescaped(&self, context: &Value) -> Result<String, PromptError>` | 渲染（不转义，谨慎使用） |
| `compile` | `fn compile(&self) -> CompiledPrompt` | 预解析，提高重复渲染性能 |

### CompiledPrompt

```rust
#[derive(Debug, Clone)]
pub struct CompiledPrompt {
    template: Arc<PromptTemplate>,
}
```

预解析的模板，包装在 `Arc` 中，适合多次重复渲染。

**方法：**
- `render(&self, context: &Value) -> Result<String, PromptError>` - 渲染编译后的模板

### PromptBuilder

```rust
#[derive(Debug, Default)]
pub struct PromptBuilder {
    messages: Vec<(String, String)>,
}
```

链式构建器，用于构建多角色提示消息（system、user、assistant、tool）。

**方法：**

| 方法 | 签名 | 用途 |
|------|------|------|
| `new` | `fn new() -> Self` | 创建新构建器 |
| `system` | `fn system(mut self, content: impl Into<String>) -> Self` | 添加 system 消息 |
| `user` | `fn user(mut self, content: impl Into<String>) -> Self` | 添加 user 消息 |
| `assistant` | `fn assistant(mut self, content: impl Into<String>) -> Self` | 添加 assistant 消息 |
| `tool` | `fn tool(mut self, content, tool_name) -> Self` | 添加工具结果消息 |
| `build` | `fn build(&self) -> String` | 渲染为 XML 标签格式 |
| `build_messages` | `fn build_messages(&self) -> Vec<(String, String)>` | 返回原始 (role, content) 元组 |

### PromptError

```rust
#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    IoError { source: std::io::Error },          // IO 错误
    JsonError { source: serde_json::Error },      // JSON 解析错误
    MissingVariable { name: String },             // 缺少变量
    SyntaxError { message: String },              // 模板语法错误
}
```

## 模板语法

### 变量替换

```
{{variable}}
```

示例：
```rust
use agentkit::prompt::PromptTemplate;
use serde_json::json;

let template = PromptTemplate::from_string("你好，{{name}}！你是{{role}}。");
let result = template.render(&json!({
    "name": "张三",
    "role": "工程师"
})).unwrap();
// 结果: "你好，张三！你是工程师。"
```

### 嵌套路径访问

```
{{object.field}}
```

示例：
```rust
let template = PromptTemplate::from_string("{{user.name}}: {{user.email}}");
let result = template.render(&json!({
    "user": {
        "name": "李四",
        "email": "lisi@example.com"
    }
})).unwrap();
// 结果: "李四: lisi@example.com"
```

### 条件渲染

```
{{#if variable}}...{{/if}}
```

示例：
```rust
let template = PromptTemplate::from_string("你好{{#if name}}，{{name}}{{/if}}！");

// 有 name
let result1 = template.render(&json!({"name": "张三"})).unwrap();
// 结果: "你好，张三！"

// 无 name
let result2 = template.render(&json!({})).unwrap();
// 结果: "你好！"
```

**条件判断规则：**
值为以下之一被视为 "falsy"：
- 字符串 `"false"`
- 字符串 `"null"`
- 空字符串 `""`

### 循环渲染

```
{{#each array}}...{{/each}}
```

示例：
```rust
let template = PromptTemplate::from_string("{{#each items}}- {{this}}\n{{/each}}");
let result = template.render(&json!({
    "items": ["苹果", "香蕉", "橙子"]
})).unwrap();
// 结果: "- 苹果\n- 香蕉\n- 橙子\n"
```

在 `{{#each}}` 块内，`{{this}}` 引用当前项。

## 使用示例

### 示例 1：简单变量替换

```rust
use agentkit::prompt::PromptTemplate;
use serde_json::json;

let template = PromptTemplate::from_string("欢迎，{{name}}！你的角色是{{role}}。");
let output = template.render(&json!({
    "name": "王五",
    "role": "管理员"
}))?;

println!("{}", output);
// 输出: "欢迎，王五！你的角色是管理员。"
```

### 示例 2：条件渲染

```rust
let template = PromptTemplate::from_string(
    "你好，{{name}}！\
     {{#if bio}}个人简介：{{bio}}\
     {{/if}}\
     {{#if website}}网站：{{website}}{{/if}}"
);

let output = template.render(&json!({
    "name": "赵六",
    "bio": "Rust 开发者",
    "website": "https://example.com"
}))?;
```

### 示例 3：循环渲染

```rust
let template = PromptTemplate::from_string(
    "技能列表：\n{{#each skills}}\n- {{this}}\n{{/each}}"
);

let output = template.render(&json!({
    "skills": ["Rust", "Python", "Go"]
}))?;

// 输出:
// 技能列表：
// - Rust
// - Python
// - Go
```

### 示例 4：PromptBuilder 构建多角色消息

```rust
use agentkit::prompt::PromptBuilder;

let prompt = PromptBuilder::new()
    .system("你是一个专业的 Rust 编程助手。")
    .user("请解释 Rust 的所有权系统。")
    .assistant("Rust 的所有权系统是内存安全的保证...")
    .user("那借用是什么呢？")
    .build();

// 结果:
// "<system>你是一个专业的 Rust 编程助手。</system>
//  <user>请解释 Rust 的所有权系统。</user>
//  <assistant>Rust 的所有权系统是内存安全的保证...</assistant>
//  <user>那借用是什么呢？</user>"
```

### 示例 5：从文件加载模板

```rust
use std::path::Path;

let template = PromptTemplate::from_file(Path::new("prompts/system.txt"))?;
let result = template.render(&json!({
    "name": "World",
    "role": "Developer"
}))?;
```

### 示例 6：编译模板以提高性能

```rust
let template = PromptTemplate::from_string("Hello, {{name}}!");
let compiled = template.compile();

// 重复使用编译后的模板
for name in &["Alice", "Bob", "Charlie"] {
    let result = compiled.render(&json!({"name": name})).unwrap();
    println!("{}", result);
}
```

### 示例 7：与 SimpleAgent 集成

```rust
use agentkit::agent::SimpleAgent;
use agentkit::prelude::Agent;
use agentkit::provider::OpenAiProvider;
use agentkit::prompt::PromptTemplate;

let provider = OpenAiProvider::from_env()?;

// 创建系统提示模板
let agent_template = PromptTemplate::from_string(
    "你是{{role}}，专注于{{specialty}}。请用{{tone}}的语气回答。"
);

// 渲染系统提示
let system_prompt = agent_template.render(&json!({
    "role": "技术写作助手",
    "specialty": "技术文档和教程编写",
    "tone": "专业但友好"
}))?;

// 创建 Agent
let agent = SimpleAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt(system_prompt)
    .build();

let output = agent.run("请帮我写一个文档示例".into()).await?;
```

### 示例 8：复杂模板（组合所有特性）

```rust
let template = PromptTemplate::from_string(
    "你是一位专业的{{role}}。\n\n\
     {{#if user_name}}你好，{{user_name}}！\n{{/if}}\
     以下是任务列表：\n\
     {{#each tasks}}\
     - [ ] {{this}}\n\
     {{/each}}\
     \n\
     {{#if deadline}}\
     截止日期：{{deadline}}\n\
     {{/if}}\
     \n\
     请开始工作。"
);

let output = template.render(&json!({
    "role": "代码审查员",
    "user_name": "张三",
    "tasks": ["审查 PR #123", "检查测试覆盖率", "更新文档"],
    "deadline": "2026-04-30"
}))?;

// 输出:
// 你是一位专业的代码审查员。
//
// 你好，张三！
// 以下是任务列表：
// - [ ] 审查 PR #123
// - [ ] 检查测试覆盖率
// - [ ] 更新文档
//
// 截止日期：2026-04-30
//
// 请开始工作。
```

## 转义行为

`render()` 方法会自动应用基本转义以防止提示注入：

- `` ``` `` → `` \` \`\` `` （代码块转义）
- `"` → `\"` （引号转义）
- `\n\n\n` → `\n\n` （压缩多余换行）

`render_unescaped()` 跳过此转义，仅在完全信任输入时使用。

## 最佳实践

1. **使用 `render()` 而非 `render_unescaped()`** - 默认的 `render()` 会转义代码块和引号，防止提示注入。

2. **模板组合优于单片模板** - 创建小型、专注的模板（角色、约束、格式），然后使用 `format!()` 组合它们。

3. **对重复渲染使用 `compile()`** - 如果在循环中或跨多次请求渲染同一模板，先编译以获得更好的性能。

4. **变量自动提取** - 无需手动声明变量，它们会在创建时从模板字符串中解析。

5. **嵌套数据使用点路径** - 使用 `{{user.name}}` 访问嵌套的 JSON 字段。

6. **注意语法限制** - 不支持 Jinja2 风格的 `{% for %}`、`{% if %}` 语法。使用 `{{#each}}` 和 `{{#if}}`。

7. **与 PromptBuilder 结合使用** - 对于多轮对话，使用 `PromptBuilder` 构建角色标签消息序列。

8. **模板文件组织** - 将常用模板存放在 `prompts/` 目录，按功能分类：
   ```
   prompts/
   ├── system/
   │   ├── assistant.txt
   │   └── coding_expert.txt
   ├── tasks/
   │   ├── code_review.txt
   │   └── documentation.txt
   └── formats/
       ├── json_response.txt
       └── markdown_report.txt
   ```

## 不支持的特性

以下特性 **不** 被支持（尽管在某些示例文件中可能出现）：

| 不支持的语法 | 说明 |
|-------------|------|
| `{% for item in list %}...{% endfor %}` | Jinja2 风格循环 |
| `{% if cond %}...{% endif %}` | Jinja2 风格条件 |
| `{% else %}` | else 分支 |
| `loop.index`, `loop.length` | 循环元数据变量 |
| `{{item.field}}` | 在 `{{#each}}` 内访问嵌套字段 |

## 上下文压缩提示模板

在 `agentkit::compact::prompt` 模块中提供了预构建的上下文压缩提示模板：

```rust
use agentkit::compact::prompt;

// 常量
prompt::BASE_COMPACT_PROMPT      // 完整压缩提示
prompt::PARTIAL_COMPACT_PROMPT   // 部分压缩提示
prompt::COMPACT_INSTRUCTIONS     // 额外指令模板

// 函数
prompt::generate_compact_prompt(instructions)      // 生成完整压缩提示
prompt::generate_partial_compact_prompt(instructions) // 生成部分压缩提示
```

这些使用简单的 `{instructions}` 占位符（不是 `{{variable}}` 模板语法）。

## 与 Agent 的集成

所有 Agent 类型都通过 `system_prompt()` builder 方法与提示模板系统集成：

- `SimpleAgent` - 第 151 行
- `ChatAgent` - 第 185 行
- `ReActAgent` - 第 254 行
- `ReflectAgent` - 第 306 行
- `ToolAgent` - 第 275 行

典型模式：
1. 创建带变量占位符的 `PromptTemplate`
2. 使用 `serde_json::json!` 上下文数据渲染
3. 将渲染后的字符串传递给 agent 的 `.system_prompt()` builder 方法

## 相关文件

- `agentkit/src/prompt.rs` - 主要提示模板实现（485 行）
- `agentkit/examples/09_prompt.rs` - 示例文件（注意：示例 2-5 语法有误）
- `agentkit/src/compact/prompt.rs` - 上下文压缩提示（150 行）
- `docs/context_compression.md` - 上下文压缩指南
