# AgentKit Skills 系统完善总结

## 参考 zeroclaw 项目实现的功能

### 1. 多格式配置支持 ✅
- SKILL.toml (优先)
- SKILL.yaml / SKILL.yml
- SKILL.json
- SKILL.md (后备)

### 2. SkillsPromptMode 提示词模式 ✅
```rust
pub enum SkillsPromptMode {
    Full,     // 完整模式：包含所有 skill 详细说明
    Compact,  // 简洁模式：只包含摘要 + read_skill 工具说明
}
```

### 3. skills_to_prompt_with_mode 方法 ✅
根据模式构建不同的系统提示词：
- **Full 模式**: 
  - 包含所有 skill 的详细说明
  - 包含 instructions
  - 适合少量 skills
- **Compact 模式**: 
  - 只包含 skill 摘要（name, description, location）
  - 添加 read_skill 工具说明
  - 适合大量 skills，保持上下文紧凑

### 4. read_skill 工具 ✅
读取 skill 的详细信息：
```rust
pub fn read_skill(skill_name: &str, skills_dir: &Path) -> Result<String, Error>
```

工作流程：
1. 查找 skill 目录
2. 优先读取配置文件（TOML > YAML > JSON）
3. 如果没有配置文件，读取 SKILL.md
4. 返回完整内容

### 5. ReadSkillTool 工具包装器 ✅
实现了 `Tool` trait，可以注册到 Agent：
- 接收 skill_name 参数
- 返回 skill 完整内容
- 错误处理

## 完整流程

```
1. 加载 Skills
   ↓
2. 收集 skill 简要描述
   ↓
3. 根据模式构建系统提示词
   ├─ Full: 包含所有详细信息
   └─ Compact: 只包含摘要 + read_skill 工具说明
   ↓
4. 创建 Agent 并注册
   ├─ Skills 转换的 Tools
   └─ read_skill 工具
   ↓
5. Agent 运行时
   ├─ 如果需要 skill 详细信息 → 调用 read_skill 工具
   └─ 如果需要执行 skill → 调用 skill 对应的工具
```

## 使用示例

```rust
use agentkit::skills::{
    SkillLoader, 
    skills_to_tools, 
    skills_to_prompt_with_mode, 
    SkillsPromptMode,
    ReadSkillTool
};

// 1. 加载 Skills
let mut loader = SkillLoader::new("skills/");
let skills = loader.load_from_dir().await?;

// 2. 构建系统提示词（Compact 模式）
let workspace_dir = std::env::current_dir()?;
let skills_prompt = skills_to_prompt_with_mode(
    &skills, 
    &workspace_dir, 
    SkillsPromptMode::Compact
);

// 3. 转换为 Tools
let executor = Arc::new(SkillExecutor::new());
let mut tools = skills_to_tools(&skills, executor, skills_dir);

// 4. 添加 read_skill 工具
tools.push(Arc::new(ReadSkillTool::new(skills_dir.to_path_buf())));

// 5. 构建系统提示词
let system_prompt = format!(
    "你是智能助手...\n\n\
     使用规则：\n\
     1. 先查看 <available_skills> 了解可用技能\n\
     2. 如果需要某个技能的详细说明，调用 read_skill 工具\n\
     3. 根据技能描述选择合适的技能执行\n\n\
     {}",
    skills_prompt
);

// 6. 创建 Agent
let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .system_prompt(system_prompt)
    .tools(tools)
    .build();
```

## 系统提示词示例（Compact 模式）

```xml
## Available Skills

Skill summaries are preloaded. Call `read_skill(name)` for full instructions.

<available_skills>
  <skill>
    <name>weather-query</name>
    <description>查询指定城市的当前天气情况</description>
    <location>skills/weather/SKILL.md</location>
  </skill>
  <skill>
    <name>rhai_min</name>
    <description>最小 Rhai 脚本技能示例</description>
    <location>skills/rhai_min/SKILL.md</location>
  </skill>
</available_skills>

<callable_tools>
  <tool>
    <name>read_skill</name>
    <description>Read full skill file by name. Use when you need detailed instructions.</description>
    <parameters>
      <name>skill_name</name>
      <type>string</type>
    </parameters>
  </tool>
</callable_tools>
```

## 关键设计亮点

1. **按需加载** - Compact 模式下，详细信息通过 read_skill 工具获取，保持上下文紧凑
2. **多格式支持** - 支持 TOML/YAML/JSON/MD 多种格式，自由度更高
3. **模式切换** - 根据场景选择 Full/Compact 模式
4. **位置渲染** - 根据 workspace_dir 渲染相对路径
5. **XML 转义** - 安全的提示词生成

## 编译验证

```bash
cd D:\Desktop\ocr\agentkit
cargo check -p agentkit-skills-example
# Finished
```

**✅ 编译成功！**

## 下一步

1. 测试完整运行流程
2. 添加更多示例 Skills
3. 实现 Skill 缓存机制
4. 添加 Skill 测试框架
