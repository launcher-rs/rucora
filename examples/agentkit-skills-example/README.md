# Agent + Skills 完整集成示例

## 运行方式

```bash
cd D:\Desktop\ocr\agentkit
export OPENAI_API_KEY=sk-your-key
cargo run -p agentkit-skills-example
```

## 功能实现

### ✅ 已完成

1. **Skills 加载** - 从 `skills/` 目录加载所有 Skills
2. **Skill 到 Tool 转换** - 将 Skills 包装成 Tool trait
3. **Agent 注册** - 将 Skills 注册到 Agent
4. **自动调用** - Agent 根据用户问题自动调用 Skills

## 实现细节

### SkillTool 适配器

参考 zeroclaw 项目的 Skill 设计：

```rust
pub struct SkillTool {
    skill: SkillDefinition,
    executor: Arc<SkillExecutor>,
    skill_path: PathBuf,
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str { &self.skill.name }
    fn description(&self) -> Option<&str> { Some(&self.skill.description) }
    
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 执行 Skill 并返回结果
    }
}
```

### Skills 转换函数

```rust
pub fn skills_to_tools(
    skills: &[SkillDefinition],
    executor: Arc<SkillExecutor>,
    skills_dir: &Path,
) -> Vec<Arc<dyn Tool>>
```

### Agent 注册

```rust
let executor = Arc::new(SkillExecutor::new());
let tools = skills_to_tools(&skills, executor, skills_dir);

let agent = DefaultAgent::builder()
    .provider(provider)
    .model("qwen3.5:9b")
    .system_prompt("你是智能助手，可以使用工具帮助用户解决问题。")
    .tools(tools)  // 批量注册 Skills
    .build();
```

## 运行输出示例

```
╔═══════════════════════════════════════════════════════════╗
║         Agent + Skills 示例                               ║
╚═══════════════════════════════════════════════════════════╝

1. 加载 Skills...
✓ 加载了 2 个 Skills

2. 创建 Agent 并注册 Skills...
   转换了 2 个 Tools
   已注册 Tools:
     - rhai_min
     - weather-query
✓ Agent 创建成功

3. 测试对话...

用户：你好
助手：你好！有什么可以帮助你的？

用户：北京天气怎么样？
助手：[自动调用 weather-query Skill]
助手：北京现在晴朗，气温 25°C。
```

## 借鉴 zeroclaw 的优秀设计

1. **Skill 结构** - 包含 name, description, version, author, tags
2. **Tool 定义** - 每个 Skill 可以有多个 tools
3. **审计机制** - 检查实现文件是否存在
4. **路径解析** - 支持 skill.name 和文件夹名不一致
5. **XML 转义** - 安全的提示词生成

## 依赖

- OPENAI_API_KEY 环境变量
- Python 3 (用于执行 Python Skills)

## 下一步

1. 添加更多示例 Skills
2. 实现 Skill 缓存机制
3. 支持 Skill 热加载
4. 添加 Skill 测试框架
