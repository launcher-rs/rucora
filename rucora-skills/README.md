# rucora Skills

rucora 的技能系统。

## 概述

本 crate 为 rucora 提供 Skills 系统支持，用于：
- YAML 格式的技能定义
- 命令模板技能
- 文件操作技能
- 从目录动态加载技能
- 技能到工具的适配器

## 安装

```toml
[dependencies]
rucora-skills = "0.1"
```

或通过主 rucora crate：

```toml
[dependencies]
rucora = { version = "0.1", features = ["skills"] }
```

## 使用方式

### 从目录加载技能

```rust
use rucora_skills::{SkillExecutor, SkillLoader, skills_to_tools};
use rucora_tools::ToolRegistry;
use std::path::Path;
use std::sync::Arc;

// 从目录加载技能
let mut loader = SkillLoader::new("skills/");
let skills = loader.load_from_dir().await?;

// 创建执行器
let executor = Arc::new(SkillExecutor::new());

// 注册技能为工具
let mut registry = ToolRegistry::new();
for tool in skills_to_tools(&skills, executor.clone(), Path::new("skills/")) {
    registry = registry.register_arc(tool);
}
```

### 加载机制

`SkillLoader::load_from_dir()` 会扫描传入目录下的一级子目录，每个子目录视为一个独立 skill。目录名只是本地路径标识，不要求和 `name` 字段一致。

每个 skill 目录内的定义文件按以下顺序读取：

1. 优先读取 `skill.yaml`
2. 如果 `skill.yaml` 不存在，读取 `SKILL.md` 或 `skill.md` 的 YAML frontmatter
3. 如果 Markdown 也不存在，再尝试读取 `skill.toml`
4. 最后尝试读取 `skill.json`

如果 `skill.yaml` 存在但读取或解析失败，加载器会自动回退到 `SKILL.md` / `skill.md` 的 frontmatter。这样可以在 YAML 配置损坏或迁移期间继续使用 Markdown 头部信息。

加载成功后，`SkillDefinition` 会记录真实来源目录。后续通过 `skills_to_tools()` 转换为工具时，会优先使用这个来源目录寻找 `SKILL.py`、`SKILL.js` 或 `SKILL.sh`，因此 skill 名称和目录名不一致也可以正常运行。

### 技能 YAML 格式

```yaml
name: my_skill
description: 一个用于执行某项任务的技能
version: "1.0"

trigger:
  keywords:
    - "执行某任务"
    - "帮我做某事"

parameters:
  - name: input
    type: string
    required: true
    description: 要处理的输入

execution:
  type: command
  template: "echo {{input}}"
```

### 技能转工具

```rust
use rucora_skills::skills_to_tools;
use std::path::Path;

// 将所有技能转换为工具
let tools = skills_to_tools(&skills, executor, Path::new("skills/"));

// 在 Agent 中使用
let agent = ToolAgent::builder()
    .provider(provider)
    .tools(tools)
    .build();
```

## Feature 配置

| Feature | 说明 |
|---------|------|
| `default` | 默认配置 |
| `all` | 启用所有功能 |

## 子模块

- `cache`：技能缓存
- `config`：技能配置解析
- `file_skills`：文件操作技能
- `integrator`：技能集成工具
- `loader`：技能加载器
- `tool_adapter`：技能到工具的适配器

## 许可证

MIT
