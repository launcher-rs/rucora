# agentkit-skills

Skills 系统 - 为 agentkit 提供灵活的技能定义和执行能力。

## 特性

- **Rhai 脚本技能**: 使用 Rhai 脚本语言定义灵活的技能逻辑
- **命令技能**: 基于 SKILL.md 模板的命令执行技能
- **文件操作技能**: 内置文件读取等常用技能
- **热加载**: 从目录动态加载技能
- **模块化设计**: 通过 feature 控制功能启用

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
agentkit-skills = "0.1.0"
```

## Feature 标志

| Feature | 说明 | 依赖 |
|---------|------|------|
| `rhai-skills` | 启用 Rhai 脚本技能支持 | `rhai` |
| `full` | 启用所有功能 | `rhai-skills` |

### 使用示例

```toml
# 仅基础功能（不含 Rhai）
agentkit-skills = "0.1.0"

# 启用 Rhai 脚本支持
agentkit-skills = { version = "0.1.0", features = ["rhai-skills"] }

# 完整功能
agentkit-skills = { version = "0.1.0", features = ["full"] }
```

## 使用示例

### 基础使用

```rust
use agentkit_skills::skills::*;

// 从目录加载技能
let skills = load_skills_from_dir("skills").await?;

// 转换为 tools
let tools = skills.as_tools();
```

### 使用 Rhai 脚本技能

需要启用 `rhai-skills` feature：

```rust
use agentkit_skills::skills::*;

// 创建 Rhai 工具调用器
let invoker = rhai_stdlib_registrar(|name, args| {
    // 实现工具调用逻辑
    Ok(json!({"result": "success"}))
});

// 加载技能（支持 SKILL.rhai）
let skills = load_skills_from_dir_with_rhai("skills", Some(invoker)).await?;
```

### 技能目录结构

```
skills/
├── weather/
│   ├── SKILL.md          # 命令技能定义
│   └── meta.yaml         # 可选的元数据
├── file_analyzer/
│   ├── SKILL.rhai        # Rhai 脚本技能
│   └── SKILL.md          # 技能描述
└── custom_skill/
    ├── SKILL.rhai
    └── meta.yaml
```

### SKILL.md 示例

```markdown
---
name: weather
description: 查询天气信息
version: 1.0.0
---

查询指定城市的天气信息。

```bash
curl -s "wttr.in/{location}?format=3"
```
```

### SKILL.rhai 示例

```rhai
// SKILL.rhai - Rhai 脚本技能
let location = ctx.input["location"];
let result = call_tool("http_get", #{ url: `wttr.in/${location}` });

if is_error(result) {
    #{ success: false, error: result["error"] }
} else {
    #{ success: true, weather: result["body"] }
}
```

## 模块结构

```
agentkit-skills
├── rhai_skills      # Rhai 脚本技能（需要 rhai-skills feature）
├── command_skills   # 命令技能
├── file_skills      # 文件操作技能
└── registry         # 技能注册表和加载逻辑
```

## 核心类型

### Skill trait

所有技能必须实现的基础 trait：

```rust
#[async_trait]
pub trait Skill {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn categories(&self) -> &'static [ToolCategory];
    fn input_schema(&self) -> Value;
    async fn run_value(&self, input: Value) -> Result<Value, SkillError>;
}
```

### SkillRegistry

技能注册表，用于管理多个技能：

```rust
let registry = SkillRegistry::new()
    .register(CommandSkill::new("weather".into(), None, "curl ...".into()))
    .register(FileReadSkill::new());

// 转换为 tools
let tools = registry.as_tools();
```

### RhaiSkill

Rhai 脚本技能（需要 `rhai-skills` feature）：

```rust
let skill = RhaiSkill::new(
    "my_script".into(),
    Some("执行自定义脚本".into()),
    script_source.into(),
    Some(rhai_registrar),
);
```

## 与 agentkit 集成

如果你使用 `agentkit` 主库，可以通过 feature 启用 skills：

```toml
[dependencies]
agentkit = { version = "0.1.0", features = ["skills", "rhai-skills"] }
```

然后在代码中使用：

```rust
use agentkit::skills::*;

// 加载技能
let skills = load_skills_from_dir("skills").await?;
```

## 依赖说明

### 最小依赖（无 Rhai）

- `agentkit-core`
- `tokio`
- `serde` / `serde_json` / `serde_yaml`
- `tracing`

### 完整依赖（含 Rhai）

额外依赖：
- `rhai` (带 `serde` 和 `sync` 特性)

## 性能考虑

- **Rhai 技能**: 脚本执行是同步的，对于耗时操作建议在宿主函数中使用 `block_in_place`
- **命令技能**: 使用异步文件 I/O 和进程执行
- **文件技能**: 支持读取大小限制，避免内存溢出

## 安全建议

1. **Rhai 脚本**: 限制可用函数，避免任意代码执行
2. **命令技能**: 验证命令模板，防止命令注入
3. **文件技能**: 限制访问路径，使用沙箱目录

## 许可证

MIT License
