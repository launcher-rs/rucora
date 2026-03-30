# 文档和注释改进计划

> 基于 CODE_COMMENT_AUDIT.md 的检查结果

## 立即改进（今天完成）

### 1. 新增基础示例

#### examples/hello_world.rs
```rust
//! AgentKit Hello World 示例
//!
//! 这个示例展示如何用最少的代码创建一个 Agent 应用。
//!
//! ## 运行
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! cargo run --example hello_world
//! ```

use agentkit::provider::OpenAiProvider;
use agentkit::agent::DefaultAgent;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("╔════════════════════════════════════════╗");
    info!("║   AgentKit Hello World 示例           ║");
    info!("╚════════════════════════════════════════╝\n");

    // 1. 检查 API Key
    if std::env::var("OPENAI_API_KEY").is_err() {
        info!("⚠ 未设置 OPENAI_API_KEY");
        info!("   请运行：export OPENAI_API_KEY=sk-your-key");
        info!("   或使用 Ollama: export OPENAI_BASE_URL=http://localhost:11434");
        return Ok(());
    }

    // 2. 创建 Provider
    info!("1. 创建 Provider...");
    let provider = OpenAiProvider::from_env()?;
    info!("✓ Provider 创建成功\n");

    // 3. 创建 Agent
    info!("2. 创建 Agent...");
    let agent = DefaultAgent::builder()
        .provider(provider)
        .model("qwen3.5:9b")  // 使用 Ollama 时
        .system_prompt("你是友好的智能助手，简洁地回答用户问题。")
        .build();
    info!("✓ Agent 创建成功\n");

    // 4. 测试对话
    info!("3. 测试对话...\n");
    
    let queries = vec![
        "你好，请介绍一下自己",
        "1+1 等于多少？",
    ];
    
    for query in queries {
        info!("用户：{}", query);
        
        match agent.run(query).await {
            Ok(output) => {
                if let Some(text) = output.text() {
                    info!("助手：{}\n", text);
                }
            }
            Err(e) => {
                info!("错误：{}\n", e);
            }
        }
    }

    info!("示例完成！");
    
    Ok(())
}
```

#### examples/Cargo.toml 配置
```toml
[[example]]
name = "hello_world"
path = "examples/hello_world.rs"
required-features = ["runtime"]
```

### 2. 改进 lib.rs 注释

```rust
//! # AgentKit
//!
//! 用 Rust 编写的高性能、类型安全的 LLM 应用开发框架
//!
//! ## 特性
//!
//! - ⚡ **极速性能** - Rust 原生，零成本抽象
//! - 🔒 **类型安全** - 编译时错误检查，运行时更可靠
//! - 💰 **成本监控** - 内置 Token 计数和成本管理
//! - 🧰 **丰富工具** - 12+ 内置工具（Shell/File/HTTP/Git/Memory 等）
//! - 🔌 **灵活集成** - 支持 10+ LLM Provider
//! - 🧠 **Agent 架构** - 思考与执行分离，支持自定义 Agent
//!
//! ## 快速开始
//!
//! ### 1. 添加依赖
//!
//! ```toml
//! [dependencies]
//! agentkit = "0.1"
//! tokio = { version = "1", features = ["full"] }
//! anyhow = "1"
//! ```
//!
//! ### 2. 设置环境变量
//!
//! ```bash
//! export OPENAI_API_KEY=sk-your-key
//! ```
//!
//! ### 3. 编写代码
//!
//! ```rust,no_run
//! use agentkit::provider::OpenAiProvider;
//! use agentkit::agent::DefaultAgent;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let provider = OpenAiProvider::from_env()?;
//!     
//!     let agent = DefaultAgent::builder()
//!         .provider(provider)
//!         .model("gpt-4o-mini")
//!         .system_prompt("你是有用的助手")
//!         .build();
//!     
//!     let output = agent.run("你好").await?;
//!     println!("{}", output.text().unwrap_or("无回复"));
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### 4. 运行
//!
//! ```bash
//! cargo run
//! ```
//!
//! ## 核心概念
//!
//! ### Agent（智能体）
//!
//! Agent 负责思考和决策。它接收用户输入，分析需求，决定是否需要调用工具。
//!
//! ```rust,no_run
//! use agentkit::agent::DefaultAgent;
//!
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .system_prompt("你是有用的助手")
//!     .build();
//!
//! let output = agent.run("北京天气怎么样？").await?;
//! ```
//!
//! ### Runtime（运行时）
//!
//! Runtime 负责执行和编排。它管理工具调用、对话历史和执行流程。
//!
//! ```rust,no_run
//! use agentkit::provider::OpenAiProvider;
//! use agentkit_runtime::{DefaultRuntime, ToolRegistry};
//! use std::sync::Arc;
//!
//! let provider = OpenAiProvider::from_env()?;
//! let tools = ToolRegistry::new();
//!
//! let runtime = DefaultRuntime::new(
//!     Arc::new(provider),
//!     tools,
//!     "gpt-4o-mini"
//! );
//! ```
//!
//! ### Tool（工具）
//!
//! 工具提供具体能力，如执行命令、读取文件、HTTP 请求等。
//!
//! ```rust,no_run
//! use agentkit::tools::{ShellTool, FileReadTool};
//! use agentkit::agent::DefaultAgent;
//!
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .model("gpt-4o-mini")
//!     .tool(ShellTool)
//!     .tool(FileReadTool)
//!     .build();
//! ```
//!
//! ### Skill（技能）
//!
//! 技能是可配置的自动化任务，通过配置文件定义。
//!
//! ```rust,no_run
//! use agentkit::skills::{SkillLoader, skills_to_tools, SkillExecutor};
//! use std::sync::Arc;
//!
//! // 加载 Skills
//! let mut loader = SkillLoader::new("skills/");
//! let skills = loader.load_from_dir().await?;
//!
//! // 转换为 Tools
//! let executor = Arc::new(SkillExecutor::new());
//! let tools = skills_to_tools(&skills, executor, skills_dir);
//!
//! // 注册到 Agent
//! let agent = DefaultAgent::builder()
//!     .provider(provider)
//!     .tools(tools)
//!     .build();
//! ```
//!
//! ## 学习路径
//!
//! ### 新手
//! 1. 运行 [Hello World](#快速开始) 示例
//! 2. 阅读 [快速开始](docs/quick_start.md)
//! 3. 查看 [用户指南](docs/user_guide.md)
//! 4. 参考 [示例集合](docs/cookbook.md)
//!
//! ### 开发者
//! 1. 阅读 [设计文档](docs/design.md)
//! 2. 学习 [Agent 与 Runtime](docs/agent_runtime_relationship.md)
//! 3. 参考 [快速参考](docs/QUICK_REFERENCE.md)
//!
//! ### 技能开发者
//! 1. 阅读 [Skill 配置规范](docs/skill_yaml_spec.md)
//! 2. 参考 [Skill 配置示例](docs/skill_yaml_examples.md)
//!
//! ## 相关文档
//!
//! - [完整文档](docs/README.md)
//! - [示例集合](docs/cookbook.md)
//! - [常见问题](docs/faq.md)
//! - [更新日志](docs/CHANGELOG.md)

// ===== 模块导出 =====

pub use agentkit_core as core;

#[cfg(feature = "runtime")]
pub use agentkit_runtime as runtime;

// ... 其他模块
```

### 3. 改进错误信息

#### 改进前
```rust
Err(SkillExecuteError::NotFound("未找到脚本实现"))
```

#### 改进后
```rust
#[derive(Debug, thiserror::Error)]
pub enum SkillExecuteError {
    #[error(
        "无法执行技能 \"{skill_name}\"\n\
         原因：未找到脚本实现文件\n\
         技能目录：{skill_dir:?}\n\
         期望文件：SKILL.py, SKILL.js, SKILL.sh 之一\n\
         建议：请确保技能目录中包含有效的脚本文件"
    )]
    ScriptNotFound {
        skill_name: String,
        skill_dir: PathBuf,
    },
    
    #[error(
        "技能执行超时\n\
         技能：{skill_name}\n\
         超时时间：{timeout}秒\n\
         建议：检查脚本是否陷入死循环，或增加 timeout 配置"
    )]
    Timeout {
        skill_name: String,
        timeout: u64,
    },
    
    #[error(
        "IO 错误\n\
         技能：{skill_name}\n\
         详情：{message}\n\
         建议：检查文件权限和路径是否正确"
    )]
    IoError {
        skill_name: String,
        message: String,
    },
}
```

### 4. 新增故障排除文档

创建 `docs/TROUBLESHOOTING.md`:

```markdown
# 故障排除指南

## 常见问题

### Q1: "未找到脚本实现" 错误

**错误信息**:
```
无法执行技能 "weather-query"
原因：未找到脚本实现文件
技能目录：skills/weather-query
期望文件：SKILL.py, SKILL.js, SKILL.sh 之一
```

**解决方案**:
1. 检查技能目录是否存在
2. 确认目录中包含 SKILL.py/SKILL.js/SKILL.sh 之一
3. 检查文件权限是否可执行

**示例**:
```bash
# 创建技能目录
mkdir -p skills/weather-query

# 创建脚本
cat > skills/weather-query/SKILL.py << 'EOF'
#!/usr/bin/env python3
print('{"success": true, "weather": "Sunny"}')
EOF

# 设置执行权限
chmod +x skills/weather-query/SKILL.py
```

### Q2: "OPENAI_API_KEY 未设置" 错误

**错误信息**:
```
⚠ 未设置 OPENAI_API_KEY
   请运行：export OPENAI_API_KEY=sk-your-key
```

**解决方案**:
```bash
# OpenAI
export OPENAI_API_KEY=sk-your-key

# 或使用 Ollama（本地）
export OPENAI_BASE_URL=http://localhost:11434
```

### Q3: 技能加载失败

**错误信息**:
```
跳过技能 "skills/ai_news": 解析错误：SKILL.md 必须以 --- 开始
```

**解决方案**:
确保 skill.yaml 或 SKILL.md 格式正确：

```yaml
# skill.yaml（推荐）
skill:
  name: ai_news
  description: AI 新闻聚合技能
```

或

```markdown
<!-- SKILL.md -->
---
name: ai_news
description: AI 新闻聚合技能
---

# AI 新闻技能说明
```

### Q4: 工具调用失败

**错误信息**:
```
工具执行错误：tool error: 脚本输出为空
```

**解决方案**:
1. 检查脚本是否正确输出 JSON
2. 添加错误处理
3. 检查超时设置

**示例**:
```python
#!/usr/bin/env python3
import sys
import json

try:
    input_data = json.loads(sys.stdin.read())
    # 处理逻辑
    result = {"success": True, "data": "result"}
    print(json.dumps(result))
except Exception as e:
    print(json.dumps({
        "success": False,
        "error": str(e)
    }))
```

## 调试技巧

### 启用详细日志

```rust
use tracing_subscriber::{FmtSubscriber, Level};

let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::DEBUG)  // 或 Level::TRACE
    .finish();
tracing::subscriber::set_global_default(subscriber)?;
```

### 查看工具调用

```rust
// 在 Agent 创建前
info!("可用工具：");
for tool in &tools {
    info!("  - {}", tool.name());
}
```

### 检查技能配置

```bash
# 验证 skill.yaml
cat skills/weather-query/skill.yaml

# 检查脚本文件
ls -la skills/weather-query/

# 测试脚本
echo '{"city": "Beijing"}' | python3 skills/weather-query/SKILL.py
```

## 获取帮助

- 📖 查看 [常见问题](faq.md)
- 💬 查看 [示例集合](cookbook.md)
- 🐛 提交 Issue
```

## 本周改进计划

### 周一
- [ ] 创建 hello_world 示例
- [ ] 改进 lib.rs 注释
- [ ] 添加错误信息改进

### 周二
- [ ] 创建 TROUBLESHOOTING.md
- [ ] 创建 EXAMPLES.md
- [ ] 更新 quick_start.md

### 周三
- [ ] 添加更多示例代码
- [ ] 改进技能加载器注释
- [ ] 添加 API 文档示例

### 周四
- [ ] 检查所有公共 API 注释
- [ ] 添加使用示例到每个模块
- [ ] 改进错误类型定义

### 周五
- [ ] 完整测试所有示例
- [ ] 收集用户反馈
- [ ] 持续改进

## 验证清单

- [ ] cargo check --workspace 通过
- [ ] cargo doc --workspace 生成完整文档
- [ ] cargo test --workspace 所有测试通过
- [ ] cargo run --example hello_world 正常运行
- [ ] 所有中文注释正确显示
- [ ] 错误信息清晰易懂
- [ ] 示例代码完整可运行
