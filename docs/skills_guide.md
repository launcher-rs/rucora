# 技能系统 (Skills Guide)

Skills 是 AgentKit 的高级功能单元，比 Tool 更抽象、更可复用。Skills 通过 YAML/TOML/JSON 配置定义，可以自动转换为 Tools。

## Skills vs Tools

| 特性 | Skill | Tool |
|------|-------|------|
| 定义方式 | 配置文件（YAML/TOML/JSON） | Rust 代码 |
| 复杂度 | 高（可包含多步骤逻辑） | 低（单一功能） |
| 复用性 | 强（可跨项目共享） | 一般 |
| 执行方式 | 通过 SkillExecutor | 直接调用 |
| 适用场景 | 复杂业务流程 | 简单原子操作 |

## 模块结构

```
agentkit-skills/src/
├── cache/        # 技能缓存
├── config/       # 技能配置解析
├── integrator/   # 技能自动集成器
├── loader/       # 技能加载和执行
└── tool_adapter/ # 技能到工具的适配
```

## Skill 配置格式

### 最小配置 (TOML)

```toml
name = "datetime"
description = "获取当前日期和时间信息"
version = "1.0.0"

[trigger]
keywords = ["时间", "日期", "几点"]

[execution]
type = "command"
command = "python3 scripts/datetime.py"
```

### 完整配置 (YAML)

```yaml
name: weather_query
description: 查询指定城市的天气信息
version: 1.0.0
author: your_name
tags:
  - weather
  - query

trigger:
  keywords:
    - 天气
    - 温度
    - 下雨
  patterns:
    - "(.*)天气怎么样"
    - "今天(.*)下雨吗"

execution:
  type: command
  command: python3 scripts/weather.py
  timeout: 30
  env:
    API_KEY: ${WEATHER_API_KEY}

input_schema:
  type: object
  properties:
    city:
      type: string
      description: 城市名称
  required:
    - city
```

## 使用方式

### 1. 加载 Skills

```rust
use agentkit::skills::{SkillLoader, SkillExecutor};
use std::path::Path;

let skills_dir = Path::new("skills/");
let mut loader = SkillLoader::new(skills_dir);
let skills = loader.load_from_dir().await?;

println!("加载了 {} 个 Skills", skills.len());
for skill in &skills {
    println!("  - {}: {}", skill.name, skill.description);
}
```

### 2. 创建 Skill Executor

```rust
use agentkit::skills::SkillExecutor;
use std::sync::Arc;

let executor = Arc::new(SkillExecutor::new());
```

### 3. 将 Skills 转换为 Tools

```rust
use agentkit::skills::SkillTool;
use agentkit::agent::ToolRegistry;

let mut registry = ToolRegistry::new();

for skill in &skills {
    let skill_tool = SkillTool::new(
        skill.clone(),
        executor.clone(),
        skills_dir.join(&skill.name),
    );
    registry = registry.register_arc(Arc::new(skill_tool));
}
```

### 4. 在 Agent 中使用

```rust
use agentkit::agent::ToolAgent;

let agent = ToolAgent::builder()
    .provider(provider)
    .model("gpt-4o-mini")
    .system_prompt("你是有用的助手。")
    .tool_registry(registry)
    .max_steps(15)
    .build();

let output = agent.run("现在几点了？").await?;
```

## SkillsAutoIntegrator

自动集成 Skills 到 Agent：

```rust
use agentkit::skills::SkillsAutoIntegrator;

let integrator = SkillsAutoIntegrator::new(skills_dir);
let agent = integrator
    .with_provider(provider)
    .with_model("gpt-4o-mini")
    .build_tool_agent()
    .await?;
```

## Skill 目录结构

```
skills/
├── datetime/
│   ├── SKILL.md          # 技能元数据（TOML/YAML/JSON）
│   └── scripts/
│       └── datetime.py   # 执行脚本
├── calculator/
│   ├── SKILL.md
│   └── scripts/
│       └── calc.py
└── weather/
    ├── SKILL.md
    └── scripts/
        └── weather.py
```

## Skill 缓存

```rust
use agentkit::skills::cache::CachedSkillLoader;

let mut loader = CachedSkillLoader::new(skills_dir);
// 首次加载后缓存，加速后续访问
let skills = loader.load_from_dir().await?;
```

## 提示词注入模式

### Full 模式（完整信息）

```rust
use agentkit::skills::SkillsPromptMode;

let prompt = skills_to_prompt_with_mode(&skills, SkillsPromptMode::Full);
```

### Compact 模式（精简信息）

```rust
let prompt = skills_to_prompt_with_mode(&skills, SkillsPromptMode::Compact);
```

## 最佳实践

1. **单一职责**: 每个 Skill 只做一件事
2. **清晰描述**: name 和 description 要准确，帮助 LLM 选择
3. **错误处理**: 脚本应处理错误并返回清晰的错误信息
4. **环境变量**: 敏感信息通过环境变量传递
5. **超时配置**: 为长时间运行的脚本设置超时
6. **版本管理**: 维护 Skill 版本，便于追踪变更
