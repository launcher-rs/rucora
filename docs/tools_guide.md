# 工具参考 (Tools Guide)

AgentKit 提供 17+ 内置工具，覆盖文件操作、系统命令、Web 请求、搜索、数学计算等场景。

## 工具分类

| 分类 | 模块 | 工具 |
|------|------|------|
| 文件操作 | `file/` | FileReadTool, FileWriteTool, FileEditTool |
| 系统命令 | `system/` | ShellTool, CmdExecTool, DatetimeTool |
| Web 请求 | `web/` | HttpRequestTool, WebFetchTool, BrowseTool, BrowserOpenTool |
| 搜索 API | `web/` | GithubTrendingTool, SerpapiTool, TavilyTool |
| 文件搜索 | `search/` | GlobSearchTool, ContentSearchTool |
| 数学计算 | `math/` | CalculatorTool |
| 媒体信息 | `media/` | ImageInfoTool |
| Git 操作 | `git/` | GitTool |
| 记忆操作 | `memory/` | MemoryRecallTool, MemoryStoreTool |
| 调试 | `echo/` | EchoTool |

---

## 文件操作工具

### FileReadTool

读取文件内容。

```rust
use agentkit::tools::FileReadTool;

let tool = FileReadTool::new();
// 输入: {"path": "path/to/file.txt"}
// 输出: 文件内容
```

### FileWriteTool

写入文件内容。

```rust
use agentkit::tools::FileWriteTool;

let tool = FileWriteTool::new();
// 输入: {"path": "path/to/file.txt", "content": "Hello World"}
```

### FileEditTool

编辑文件内容（支持精确替换）。

```rust
use agentkit::tools::FileEditTool;

let tool = FileEditTool::new();
// 输入: {"path": "file.txt", "old_string": "...", "new_string": "..."}
```

---

## 系统命令工具

### ShellTool

执行系统命令，支持安全策略配置。

```rust
use agentkit::tools::system::ShellTool;

// 基础使用
let tool = ShellTool::new();

// 带安全策略
use agentkit::tools::system::ShellConfig;
let config = ShellConfig {
    allowed_commands: Some(vec!["ls", "cat", "pwd"]),  // 命令白名单
    blocked_commands: vec!["rm", "sudo", "curl"],      // 命令黑名单
    work_dir: Some("/tmp".to_string()),                 // 工作目录
};
let tool = ShellTool::with_config(config);
```

### 安全策略

| 策略 | 说明 |
|------|------|
| 命令白名单 | 只允许执行指定命令 |
| 命令黑名单 | 阻止执行危险命令 |
| 路径遍历防护 | 防止 `../` 攻击 |
| 环境变量泄露检测 | 防止 `env`/`printenv` 泄露 |
| 工作目录限制 | 限制命令执行范围 |

### CmdExecTool

执行命令（简化版本）。

```rust
use agentkit::tools::CmdExecTool;
// 输入: {"command": "echo hello"}
```

### DatetimeTool

获取当前日期时间。

```rust
use agentkit::tools::DatetimeTool;
// 支持公历、农历、干支、生肖、星座等信息
// 输入: {"format": "text"} 或 {"format": "json"}
```

---

## Web 请求工具

### HttpRequestTool

发送 HTTP 请求。

```rust
use agentkit::tools::HttpRequestTool;

let tool = HttpRequestTool::new();
// 输入: {"method": "GET", "url": "https://api.example.com/data"}
```

### WebFetchTool

获取网页内容。

```rust
use agentkit::tools::web::WebFetchTool;
// 输入: {"url": "https://example.com"}
```

### BrowseTool / BrowserOpenTool

浏览器操作（需要外部浏览器驱动）。

```rust
use agentkit::tools::web::BrowseTool;
use agentkit::tools::web::BrowserOpenTool;
```

---

## 搜索 API 工具

### GithubTrendingTool

获取 GitHub 趋势项目。

```rust
use agentkit::tools::web::GithubTrendingTool;
// 输入: {} （无参数）
```

### SerpapiTool

SerpAPI 搜索引擎。

```rust
use agentkit::tools::web::SerpapiTool;
// 输入: {"query": "search query", "api_key": "..."}
```

### TavilyTool

Tavily 搜索 API。

```rust
use agentkit::tools::web::TavilyTool;
// 输入: {"query": "search query", "api_key": "..."}
```

---

## 文件搜索工具

### GlobSearchTool

通配符文件搜索。

```rust
use agentkit::tools::search::GlobSearchTool;
// 输入: {"pattern": "**/*.rs", "path": "."}
// 输出: 匹配的文件列表
```

### ContentSearchTool

文件内容正则搜索。

```rust
use agentkit::tools::search::ContentSearchTool;
// 输入: {"pattern": "fn main", "path": ".", "file_pattern": "*.rs"}
// 输出: 匹配的行和文件
```

---

## 数学计算工具

### CalculatorTool

支持 25+ 数学函数。

```rust
use agentkit::tools::math::CalculatorTool;

// 支持的函数类型：
// - 算术: +, -, *, /, ^, sqrt
// - 对数: log, ln
// - 统计: mean, median, sum, min, max
// - 聚合: count, variance, stdev

// 输入: {"expression": "10 + 20 * 3"}
// 输出: 计算结果
```

---

## 媒体信息工具

### ImageInfoTool

读取图片元数据（PNG/JPEG/GIF/WebP/BMP）。

```rust
use agentkit::tools::media::ImageInfoTool;

let tool = ImageInfoTool::new();
// 输入: {"path": "image.png"}
// 输出: 宽度、高度、格式、大小等
```

---

## Git 操作工具

### GitTool

执行 Git 命令。

```rust
use agentkit::tools::git::GitTool;

let tool = GitTool::new();
// 输入: {"command": "log", "args": ["--oneline", "-10"]}
```

---

## 记忆操作工具

### MemoryStoreTool / MemoryRecallTool

存储和召回记忆。

```rust
use agentkit::tools::memory::{MemoryStoreTool, MemoryRecallTool};
// 配合 Memory 实例使用
```

---

## 调试工具

### EchoTool

回显输入参数，用于测试和调试。

```rust
use agentkit::tools::EchoTool;

let tool = EchoTool;
// 输入: {"text": "hello"}
// 输出: {"text": "hello"}
```

---

## 自定义 Tool

实现 `Tool` trait 即可创建自定义工具：

```rust
use agentkit::prelude::Tool;
use agentkit::error::ToolError;
use serde_json::{Value, json};

struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    fn description(&self) -> &str {
        "描述这个工具的功能".to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "param1": {"type": "string", "description": "参数1说明"}
            },
            "required": ["param1"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let param1 = input["param1"].as_str().unwrap_or("");
        // 执行逻辑
        Ok(json!({"result": format!("Hello {}", param1)}))
    }
}
```

### 注册到 Agent

```rust
use agentkit::agent::ToolAgent;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .tool(MyTool)  // 注册自定义工具
    .build();
```

---

## 工具使用最佳实践

1. **精简工具列表**: 只注册当前任务需要的工具
2. **清晰的描述**: 工具 name 和 description 要清晰，帮助 LLM 正确选择
3. **严格的 Schema**: 定义严格的 input_schema，防止 LLM 传递错误参数
4. **错误处理**: 工具执行失败时返回清晰的错误信息
5. **安全策略**: ShellTool 等危险工具务必配置安全策略
