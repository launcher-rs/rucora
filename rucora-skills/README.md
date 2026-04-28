# rucora Skills

rucora 的技能系统。

## 概述

本 crate 为 rucora 提供 Skills 系统支持，用于：
- YAML 格式的技能定义
- 命令模板技能
- 文件操作技能
- Rhai 脚本技能（可选）
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
use rucora_skills::{SkillLoader, SkillExecutor, SkillTool};
use rucora_tools::ToolRegistry;
use std::sync::Arc;

// 从目录加载技能
let mut loader = SkillLoader::new("skills/");
let skills = loader.load_from_dir().await?;

// 创建执行器
let executor = Arc::new(SkillExecutor::new());

// 注册技能为工具
let mut registry = ToolRegistry::new();
for skill in &skills {
    let tool = SkillTool::new(skill.clone(), executor.clone(), "skills/");
    registry = registry.register_arc(Arc::new(tool));
}
```

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

// 将所有技能转换为工具
let tools = skills_to_tools(&skills, executor, "skills/");

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
