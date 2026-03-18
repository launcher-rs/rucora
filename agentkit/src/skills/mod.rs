//! Skills（技能）相关实现。
//!
//! 本模块包含具体的技能实现，是对 agentkit-core 中技能类型定义的具体化。
//!
//! 设计理念：
//! - Skill 是对 Tool/Provider/Memory 的组合封装，提供更高层次的抽象
//! - 每个技能都是独立的可执行单元，具有明确的输入输出
//! - 技能应该专注于解决特定领域的问题
//!
//! 使用示例：
//! ```rust
//! use agentkit::skills::EchoSkill;
//!
//! let skill = EchoSkill::new();
//! let ctx = SkillContext { input: json!("hello") };
//! let result = skill.run(ctx).await?;
//! ```

use agentkit_core::{
    error::SkillError,
    skill::types::{SkillContext, SkillOutput},
};
use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::Path;
use std::{collections::HashMap, sync::Arc};
use tokio::fs;
use tracing::{debug, info};

pub use agentkit_core::skill::Skill;

/// Skill 注册表：集中管理所有可用 skills。
///
/// 在运行前（启动阶段）注册所有 skills，然后在运行时把它们转换成 `ToolRegistry`
/// 交给 `ToolCallingAgent`。
#[derive(Default, Clone)]
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

/// 基于 `SKILL.md` 的通用命令型 skill：执行命令并返回 stdout/stderr。
///
/// 这里不为每个 skill 写一个专用 struct（例如 WeatherSkill），而是让 SKILL.md 驱动执行流程。
pub struct CommandSkill {
    pub name: String,
    pub description: Option<String>,
    pub command_template: String,
}

impl CommandSkill {
    pub fn new(name: String, description: Option<String>, command_template: String) -> Self {
        Self {
            name,
            description,
            command_template,
        }
    }

    fn render_command(&self, input: &Value) -> Result<String, SkillError> {
        let location = input
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("London");

        let mode = input
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("today");

        let location_q = location.trim().replace(' ', "+");

        // 支持 weather 这类常见模式：mode=full 时用 ?T，否则用 format=3。
        // 若模板未包含这些占位符，则只替换 {location}。
        let mut cmd = self.command_template.clone();
        cmd = cmd.replace("{location}", &location_q);
        cmd = cmd.replace("{mode}", mode);
        Ok(cmd)
    }
}

#[async_trait]
impl Skill for CommandSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::System]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "地点/城市名，例如：北京 / Beijing / New York"},
                "mode": {"type": "string", "description": "可选：today(今天)/full(完整)"}
            }
        })
    }

    async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
        debug!(skill.name = %self.name, skill.template = %self.command_template, "command_skill.start");

        let cmd = self.render_command(&input)?;
        info!(skill.name = %self.name, cmd.command = %cmd, "command_skill.exec");

        // Skill 内部通过 tool 来执行命令（符合“skills 执行时调用工具执行命令”的约束）。
        let tool = crate::tools::CmdExecTool::new();
        let out = tool
            .call(json!({"command": cmd}))
            .await
            .map_err(|e| SkillError::Message(format!("cmd_exec tool 执行失败: {}", e)))?;

        Ok(json!({
            "skill": self.name,
            "tool": "cmd_exec",
            "result": out
        }))
    }
}

impl SkillRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// 注册一个 skill。
    ///
    /// - key 为 `skill.name()`
    /// - 同名注册会覆盖
    pub fn register<S: Skill + 'static>(mut self, skill: S) -> Self {
        self.skills
            .insert(skill.name().to_string(), Arc::new(skill));
        self
    }

    /// 将当前注册的 skills 暴露为 `ToolRegistry`，用于 tool-calling。
    ///
    /// 这是“Step2/3：提取 schema → 注入 LLM 上下文”的关键连接点：
    /// `ToolCallingAgent` 会把 `ToolRegistry.definitions()` 发送给 provider。
    pub fn as_tool_registry(&self) -> agentkit_runtime::ToolRegistry {
        let mut reg = agentkit_runtime::ToolRegistry::new();

        for skill in self.skills.values() {
            reg = reg.register(SkillTool::new(skill.clone()));
        }

        reg
    }
}

/// 从工作区 `skills/` 目录加载 skills。
///
/// 约定：每个 skill 是一个子目录，包含 `SKILL.md`，其开头为 YAML frontmatter：
///
/// ```text
/// ---
/// name: weather
/// description: ...
/// ---
/// ```
///
/// 当前实现先支持 `weather` skill；后续可以按 name 扩展更多 skill 的具体执行实现。
pub async fn load_skills_from_dir(dir: impl AsRef<Path>) -> Result<SkillRegistry, SkillError> {
    let dir = dir.as_ref();
    let mut registry = SkillRegistry::new();

    info!(skills.dir = %dir.display(), "skills.load.start");

    let mut rd = fs::read_dir(dir)
        .await
        .map_err(|e| SkillError::Message(format!("读取 skills 目录失败: {}", e)))?;

    while let Some(entry) = rd
        .next_entry()
        .await
        .map_err(|e| SkillError::Message(format!("遍历 skills 目录失败: {}", e)))?
    {
        let path = entry.path();
        let ty = entry
            .file_type()
            .await
            .map_err(|e| SkillError::Message(format!("读取目录项类型失败: {}", e)))?;

        if !ty.is_dir() {
            continue;
        }

        debug!(skills.dir_entry = %path.display(), "skills.load.found_dir");

        let skill_md = path.join("SKILL.md");
        if fs::metadata(&skill_md).await.is_err() {
            debug!(skills.dir_entry = %path.display(), "skills.load.skip_no_skill_md");
            continue;
        }

        debug!(skills.skill_md = %skill_md.display(), "skills.load.read_skill_md");

        let md = fs::read_to_string(&skill_md)
            .await
            .map_err(|e| SkillError::Message(format!("读取 SKILL.md 失败: {}", e)))?;

        let (name, description) = parse_skill_md_frontmatter(&md);
        let command_template = extract_primary_command_template(&md);

        debug!(
            skills.skill_md = %skill_md.display(),
            skills.name = name.as_deref().unwrap_or(""),
            skills.description = description.as_deref().unwrap_or(""),
            "skills.load.frontmatter"
        );

        debug!(
            skills.skill_md = %skill_md.display(),
            skills.command_template = command_template.as_deref().unwrap_or(""),
            "skills.load.command_template"
        );

        if let Some(name) = name {
            if let Some(tpl) = command_template {
                info!(skills.name = %name, "skills.load.register");
                registry = registry.register(CommandSkill::new(name, description, tpl));
            } else {
                debug!(skills.name = %name, "skills.load.skip_no_command_template");
            }
        }
    }

    info!(skills.count = registry.skills.len(), "skills.load.done");

    Ok(registry)
}

fn parse_skill_md_frontmatter(md: &str) -> (Option<String>, Option<String>) {
    // 超轻量解析：只读取第一段 --- ... ---
    let mut lines = md.lines();
    if lines.next().map(|l| l.trim()) != Some("---") {
        return (None, None);
    }

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;

    for line in lines {
        let t = line.trim();
        if t == "---" {
            break;
        }
        if let Some((k, v)) = t.split_once(':') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').to_string();
            match key {
                "name" => name = Some(val),
                "description" => description = Some(val),
                _ => {}
            }
        }
    }

    (name, description)
}

fn extract_primary_command_template(md: &str) -> Option<String> {
    // 约定：从第一个 ```bash 代码块中提取第一条以 curl 开头的命令。
    // weather 的 SKILL.md 示例为：curl -s "wttr.in/London?format=3"
    let mut in_bash = false;

    for line in md.lines() {
        let t = line.trim();

        if t.starts_with("```bash") {
            in_bash = true;
            continue;
        }

        if in_bash && t.starts_with("```") {
            break;
        }

        if !in_bash {
            continue;
        }

        if t.starts_with("curl") {
            // 把示例中的 London 替换为 {location}。
            // 目前先覆盖 weather skill 的常见写法，后续可以做更通用的模板化。
            let mut cmd = t.to_string();
            cmd = cmd.replace("wttr.in/London?format=3", "wttr.in/{location}?format=3");
            cmd = cmd.replace("wttr.in/London?T", "wttr.in/{location}?T");
            cmd = cmd.replace("wttr.in/London", "wttr.in/{location}");
            return Some(cmd);
        }
    }

    None
}

/// Skill 到 Tool 的通用适配器。
///
/// 作用：让 skill 能以 tool 的形式被 `ToolCallingAgent` 调度。
/// - tool.name() == skill.name()
/// - tool.input_schema() == skill.input_schema()
/// - tool.call(args) 会转调 skill.run_value(args)
pub struct SkillTool {
    skill: Arc<dyn Skill>,
}

impl SkillTool {
    /// 创建 skill->tool 适配器。
    pub fn new(skill: Arc<dyn Skill>) -> Self {
        Self { skill }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        self.skill.name()
    }

    fn description(&self) -> Option<&str> {
        self.skill.description()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        self.skill.categories()
    }

    fn input_schema(&self) -> Value {
        self.skill.input_schema()
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let skill_name = self.skill.name();
        let input_str = input.to_string();
        let input_len = input_str.len();
        let input_snippet: String = input_str.chars().take(500).collect();

        debug!(
            skill.name = %skill_name,
            skill.input_len = input_len,
            skill.input_snippet = %input_snippet,
            "skill_tool.call.start"
        );

        let start = std::time::Instant::now();
        let out = self
            .skill
            .run_value(input)
            .await
            .map_err(|e| ToolError::Message(e.to_string()))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let out_str = out.to_string();
        let out_len = out_str.len();
        let out_snippet: String = out_str.chars().take(500).collect();

        debug!(
            skill.name = %skill_name,
            skill.output_len = out_len,
            skill.output_snippet = %out_snippet,
            skill.elapsed_ms = elapsed_ms,
            "skill_tool.call.done"
        );

        Ok(out)
    }
}

pub struct FileReadSkill {
    pub name: String,
    pub description: Option<String>,
    pub default_max_bytes: usize,
}

impl FileReadSkill {
    /// 创建一个读取本地文件的 skill。
    ///
    /// 默认：
    /// - name: `file_read_skill`（直接作为 tool name 暴露给 LLM）
    /// - default_max_bytes: 200_000（用于截断输出，避免一次读太大）
    pub fn new() -> Self {
        Self {
            name: "file_read_skill".to_string(),
            description: Some("读取本地文件内容".to_string()),
            default_max_bytes: 200_000,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub async fn run(&self, ctx: SkillContext) -> Result<SkillOutput, SkillError> {
        // 输入参数：
        // - path: 必填，本地文件路径
        // - max_bytes: 可选，最多读取字符数（用于截断）
        let path = ctx
            .input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::Message("缺少必需的 'path' 字段".to_string()))?
            .to_string();

        let max_bytes = ctx
            .input
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(self.default_max_bytes);

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| SkillError::Message(format!("读取文件失败: {}", e)))?;

        let truncated = content.len() > max_bytes;
        let out_content = if truncated {
            content.chars().take(max_bytes).collect::<String>()
        } else {
            content
        };

        Ok(SkillOutput {
            output: json!({
                "path": path,
                "content": out_content,
                "truncated": truncated,
                "max_bytes": max_bytes,
            }),
        })
    }
}

#[async_trait]
impl Skill for FileReadSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "本地文件路径"},
                "max_bytes": {"type": "integer", "description": "可选，最多读取字符数（用于截断）"}
            },
            "required": ["path"]
        })
    }

    async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
        let out = self.run(SkillContext { input }).await?;
        Ok(out.output)
    }
}

#[cfg(test)]
mod tests {}
