# Skills 配置文件统一命名

## 修改内容

### 1. 配置文件名称统一 ✅
- 所有配置文件统一命名为 `skill.yaml`
- 支持的配置文件优先级：`skill.yaml` > `skill.toml` > `skill.json` > `SKILL.md`

### 2. 已创建的 skill.yaml 文件
- ✅ calculator/skill.yaml
- ✅ datetime/skill.yaml
- ✅ weather-query/skill.yaml
- ✅ rhai_min/skill.yaml

## 配置文件优先级

```rust
pub async fn load_skill(&self, skill_dir: &Path) -> Result<SkillDefinition, SkillLoadError> {
    // 配置文件优先级：skill.yaml > skill.toml > skill.json > SKILL.md
    let definition = if skill_dir.join("skill.yaml").exists() {
        // 读取 skill.yaml
        serde_yaml::from_str(&content)?
    } else if skill_dir.join("skill.toml").exists() {
        // 读取 skill.toml
        toml::from_str(&content)?
    } else if skill_dir.join("skill.json").exists() {
        // 读取 skill.json
        serde_json::from_str(&content)?
    } else {
        // 读取 SKILL.md
        parse_skill_md(&content)?
    };
    
    Ok(definition)
}
```

## skill.yaml 格式示例

```yaml
name: calculator
description: 执行数学表达式计算
version: 1.0.0
author: AgentKit Team
tags:
  - utility
  - math
timeout: 10
input_schema:
  type: object
  properties:
    expression:
      type: string
      description: 数学表达式（支持 +, -, *, /, ^, sqrt 等）
  required:
    - expression
output_schema:
  type: object
  properties:
    success:
      type: boolean
    result:
      type: number
    expression:
      type: string
```

## 文件结构

```
skills/
├── calculator/
│   ├── skill.yaml      ← 配置文件（优先读取）
│   ├── SKILL.md        ← 文档（后备）
│   └── SKILL.py        ← 实现
├── datetime/
│   ├── skill.yaml
│   ├── SKILL.md
│   └── SKILL.py
├── weather/
│   ├── skill.yaml
│   ├── SKILL.md
│   └── SKILL.py
└── rhai_min/
    ├── skill.yaml
    ├── SKILL.md
    └── SKILL.rhai
```

## 优势

1. **统一命名** - 所有配置文件都叫 `skill.yaml`，易于记忆
2. **格式清晰** - YAML 格式比 Markdown Frontmatter 更清晰
3. **向后兼容** - 仍然支持 SKILL.md 作为后备
4. **灵活选择** - 支持 YAML/TOML/JSON 多种格式

## 验证结果

```bash
cargo check --workspace
# ✅ Finished
```

## 下一步

- [ ] 更新文档说明新的配置文件格式
- [ ] 将 SKILL.md 中的详细说明移到 skill.yaml 的注释或单独文档
- [ ] 添加配置文件模板
