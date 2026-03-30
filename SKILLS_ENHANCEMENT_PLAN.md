# Skills 系统完善计划

## 已完成的工作

### 1. 核心功能 ✅
- ✅ SkillLoader - 从目录加载 Skills
- ✅ SkillExecutor - 执行 Python/JavaScript/Shell 脚本
- ✅ SkillTool 适配器 - 将 Skill 包装成 Tool trait
- ✅ skills_to_tools - 批量转换 Skills 为 Tools
- ✅ Agent 注册 - 使用 `.tools()` 方法批量注册

### 2. 配置文件支持 ✅
- ✅ 支持 SKILL.md 中的 YAML Frontmatter
- ✅ 支持 SKILL.toml（通过 toml 库）
- ✅ 支持 SKILL.yaml（通过 serde_yaml 库）
- ✅ 支持 SKILL.json（通过 serde_json 库）

### 3. 示例代码 ✅
- ✅ agentkit-skills-example - 完整的 Agent + Skills 示例

## 待完成的工作（参考 zeroclaw）

### 1. SkillsPromptMode 提示词模式
```rust
pub enum SkillsPromptMode {
    Full,     // 完整模式：包含所有 skill 详细说明
    Compact,  // 简洁模式：只包含摘要，通过 read_skill 获取详情
}
```

### 2. skills_to_prompt_with_mode 方法
根据模式构建不同的系统提示词：
- **Full 模式**: 包含所有 skill 的详细说明和工具
- **Compact 模式**: 只包含 skill 摘要，添加 read_skill 工具说明

### 3. read_skill 工具
读取 skill 的详细信息：
```rust
pub fn read_skill(skill_name: &str, skills_dir: &Path) -> Result<String, Error> {
    // 1. 查找 skill 目录
    // 2. 读取配置文件（TOML/YAML/JSON 优先）
    // 3. 如果没有配置文件，读取 SKILL.md
    // 4. 返回完整内容
}
```

### 4. build_system_prompt 方法
集成 skills 到系统提示词：
```rust
fn build_system_prompt(
    base_prompt: &str,
    skills: &[SkillDefinition],
    workspace_dir: &Path,
    mode: SkillsPromptMode,
) -> String {
    let mut prompt = base_prompt.to_string();
    prompt.push_str(&skills_to_prompt_with_mode(skills, workspace_dir, mode));
    prompt
}
```

## 实现流程

```
1. 加载 Skills
   ↓
2. 收集 skill 简要描述（name, description）
   ↓
3. 根据模式构建系统提示词
   ├─ Full: 包含所有详细信息
   └─ Compact: 只包含摘要 + read_skill 工具说明
   ↓
4. 创建 Agent 并注册 skills
   ↓
5. Agent 运行时
   ├─ 如果需要 skill 详细信息 → 调用 read_skill 工具
   └─ 如果需要执行 skill → 调用 skill 对应的工具
```

## 下一步

1. 完善 tool_adapter.rs 中的 skills_to_prompt_with_mode 函数
2. 实现 read_skill 工具
3. 在 Agent 中集成 build_system_prompt 方法
4. 添加 SkillsPromptMode 配置选项
5. 更新示例代码演示不同模式的使用

## 参考 zeroclaw 的关键设计

1. **多格式配置支持** - TOML/YAML/JSON 优先，MD 作为后备
2. **提示词模式** - Full/Compact 两种模式，适应不同场景
3. **按需加载** - Compact 模式下通过 read_skill 工具获取详细信息
4. **位置渲染** - 根据 workspace_dir 渲染 skill 相对路径
5. **XML 转义** - 安全的提示词生成
