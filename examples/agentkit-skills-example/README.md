# Agent + Skills 完整示例

## 运行方式

```bash
cd D:\Desktop\ocr\agentkit
export OPENAI_API_KEY=sk-your-key
# 或使用 Ollama
export OPENAI_BASE_URL=http://localhost:11434

cargo run -p agentkit-skills-example
```

## 功能演示

### 1. Skills 加载
从 `skills/` 目录自动加载所有 Skills

### 2. Full/Compact 两种提示词模式

**Full 模式**:
- 包含所有 skill 的详细说明
- 包含 instructions
- 适合少量 skills

**Compact 模式**:
- 只包含 skill 摘要（name, description, location）
- 添加 read_skill 工具说明
- 适合大量 skills，保持上下文紧凑

### 3. read_skill 工具
读取 skill 的详细信息：
- 优先读取配置文件（TOML > YAML > JSON）
- 如果没有配置文件，读取 SKILL.md
- 返回完整内容

### 4. Agent 自动调用 Skills
Agent 根据用户问题自动选择合适的 Skill 执行

## 运行输出示例

```
╔═══════════════════════════════════════════════════════════╗
║         Agent + Skills 完整示例                           ║
╚═══════════════════════════════════════════════════════════╝

1. 加载 Skills...
✓ 加载了 2 个 Skills

已加载的 Skills:
  - weather-query: 查询指定城市的当前天气情况
  - rhai_min: 最小 Rhai 脚本技能示例

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
示例 A: Full 模式（包含所有详细信息）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. 构建 Full 模式系统提示词...
✓ 提示词长度：1234 字符

提示词预览:
  ## Available Skills
  
  Skill instructions are preloaded below.
  
  <available_skills>
    <skill>
      <name>weather-query</name>
      <description>查询指定城市的当前天气情况</description>
      ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
示例 B: Compact 模式（简洁模式 + read_skill 工具）
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. 构建 Compact 模式系统提示词...
✓ 提示词长度：567 字符

提示词预览:
  ## Available Skills
  
  Skill summaries are preloaded. Call `read_skill(name)` for full instructions.
  
  <available_skills>
    <skill>
      <name>weather-query</name>
      <description>查询指定城市的当前天气情况</description>
      ...

4. 演示 read_skill 工具...
读取 'weather-query' 技能的详细信息...

✓ 读取成功:
  内容预览:
  === SKILL.toml ===
  [skill]
  name = "weather-query"
  description = "查询指定城市的当前天气情况"
  version = "1.0.0"
  ...
```

## 系统提示词对比

### Full 模式

```xml
## Available Skills

Skill instructions are preloaded below.

<available_skills>
  <skill>
    <name>weather-query</name>
    <description>查询指定城市的当前天气情况</description>
    <location>skills/weather/SKILL.md</location>
    <instructions>
      <instruction>使用 wttr.in API 查询天气</instruction>
    </instructions>
  </skill>
</available_skills>
```

### Compact 模式

```xml
## Available Skills

Skill summaries are preloaded. Call `read_skill(name)` for full instructions.

<available_skills>
  <skill>
    <name>weather-query</name>
    <description>查询指定城市的当前天气情况</description>
    <location>skills/weather/SKILL.md</location>
  </skill>
</available_skills>

<callable_tools>
  <tool>
    <name>read_skill</name>
    <description>Read full skill file by name</description>
    <parameters>
      <name>skill_name</name>
      <type>string</type>
    </parameters>
  </tool>
</callable_tools>
```

## 依赖

- OPENAI_API_KEY 环境变量
- Python 3 (用于执行 Python Skills)

## 参考

- [zeroclaw 项目 Skills 设计](temp/zeroclaw/src/skills/mod.rs)
- [Skills 规范文档](docs/skills_specification.md)
- [Skills 实现总结](SKILLS_FINAL_SUMMARY.md)
