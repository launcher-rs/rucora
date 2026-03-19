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
use rhai::{Engine, Scope};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{OnceLock, RwLock};
use std::{collections::HashMap, sync::Arc};
use tokio::fs;
use tracing::{debug, info};

pub use agentkit_core::skill::Skill;

static GLOBAL_RHAI_REGISTRAR: OnceLock<RwLock<Option<RhaiEngineRegistrar>>> = OnceLock::new();

fn global_rhai_registrar_cell() -> &'static RwLock<Option<RhaiEngineRegistrar>> {
    GLOBAL_RHAI_REGISTRAR.get_or_init(|| RwLock::new(None))
}

pub fn set_global_rhai_engine_registrar(registrar: Option<RhaiEngineRegistrar>) {
    let cell = global_rhai_registrar_cell();
    let mut w = cell.write().expect("global rhai registrar lock poisoned");
    *w = registrar;
}

fn get_global_rhai_engine_registrar() -> Option<RhaiEngineRegistrar> {
    let cell = global_rhai_registrar_cell();
    let r = cell.read().expect("global rhai registrar lock poisoned");
    r.clone()
}

/// Rhai 引擎的“宿主函数注册器”。
///
/// 说明：
/// - blockcell 的 SKILL.rhai 脚本里通常会调用 `call_tool(...)`、`browse(...)` 等宿主函数。
/// - 这些函数不是 Rhai 自带的，需要宿主程序在创建 `Engine` 时注册。
/// - agentkit 在 core/skills 层不强行绑定具体工具集，因此通过该接口把注册权交给上层。
///
/// 你可以在应用启动时：
/// - 实现一个 registrar：向 `Engine` 注册你希望脚本可用的函数
/// - 再调用 `load_skills_from_dir_with_rhai(...)` 加载脚本 skills
pub type RhaiEngineRegistrar = Arc<dyn Fn(&mut Engine) + Send + Sync>;

#[derive(Debug, Clone, Default, Deserialize)]
struct SkillMetaYaml {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    triggers: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
}

fn format_meta_description(base: Option<String>, meta: &SkillMetaYaml) -> Option<String> {
    let mut desc = base.or_else(|| meta.description.clone());

    let mut extra: Vec<String> = Vec::new();
    if !meta.triggers.is_empty() {
        extra.push(format!("触发词: {}", meta.triggers.join(", ")));
    }
    if !meta.capabilities.is_empty() {
        extra.push(format!("capabilities: {}", meta.capabilities.join(", ")));
    }

    if !extra.is_empty() {
        let extra_str = extra.join("\n");
        desc = Some(match desc {
            Some(d) if !d.is_empty() => format!("{}\n{}", d, extra_str),
            _ => extra_str,
        });
    }

    desc
}

/// 从工作区 `skills/` 目录加载 skills（支持 SKILL.md / SKILL.rhai）。
///
/// - SKILL.md -> `CommandSkill`
/// - SKILL.rhai -> `RhaiSkill`
///
/// 该版本提供 `rhai_registrar` 参数，用于注册自定义 Rhai 宿主函数。
pub async fn load_skills_from_dir_with_rhai(
    dir: impl AsRef<Path>,
    rhai_registrar: Option<RhaiEngineRegistrar>,
) -> Result<SkillRegistry, SkillError> {
    load_skills_from_dir_inner(dir.as_ref(), rhai_registrar).await
}

/// Skill 注册表：集中管理所有可用 skills。
///
/// 在运行前（启动阶段）注册所有 skills，然后在运行时把它们转换成 `ToolRegistry`
/// 交给 `ToolCallingAgent`。
#[derive(Default, Clone)]
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

/// 基于 `SKILL.rhai` 的脚本型 skill。
///
/// 设计目标：参考 blockcell 的 skills 形态，让每个 skill 用一段 Rhai 脚本来描述“怎么做”。
///
/// 约定：脚本运行时会注入 `ctx`：
/// - `ctx.user_input`：用户原始输入文本
/// - `ctx.input`：本次 tool call 的 JSON input（serde_json::Value）
///
/// 脚本返回值：
/// - 推荐返回一个 map（例如 `#{ success: true, instruction: "..." }`）
/// - host 会尽量把返回值转成 JSON Value 作为 tool result 回传
pub struct RhaiSkill {
    pub name: String,
    pub description: Option<String>,
    pub script_source: String,
    pub rhai_registrar: Option<RhaiEngineRegistrar>,
}

impl RhaiSkill {
    pub fn new(
        name: String,
        description: Option<String>,
        script_source: String,
        rhai_registrar: Option<RhaiEngineRegistrar>,
    ) -> Self {
        Self {
            name,
            description,
            script_source,
            rhai_registrar,
        }
    }
}

#[async_trait]
impl Skill for RhaiSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        // 这里不强制 schema：不同脚本的入参结构可能不同。
        // 统一给一个 object，以便 LLM 可以自由传参。
        json!({
            "type": "object",
            "description": "SKILL.rhai 脚本输入（由脚本自行解析 ctx.input / ctx.user_input）"
        })
    }

    async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
        debug!(skill.name = %self.name, skill.kind = "rhai", "rhai_skill.start");

        // 说明：Rhai 引擎本身是同步执行的。
        // 这里先用最小实现：直接在当前线程运行脚本。
        // 如果后续脚本变重，可以考虑 spawn_blocking。
        let mut engine = Engine::new();
        if let Some(reg) = &self.rhai_registrar {
            reg(&mut engine);
        }

        // 注入 ctx
        // - ctx.user_input：如果外部没有传入，就用空字符串
        // - ctx.input：本次调用参数
        let mut ctx_map = rhai::Map::new();
        let user_input = input
            .get("user_input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        ctx_map.insert("user_input".into(), user_input.into());

        let dyn_input = rhai::serde::to_dynamic(input.clone())
            .map_err(|e| SkillError::Message(format!("rhai: input 转 dynamic 失败: {}", e)))?;
        ctx_map.insert("input".into(), dyn_input);

        let ctx_dynamic: rhai::Dynamic = ctx_map.into();

        let mut scope = Scope::new();
        scope.push_dynamic("ctx", ctx_dynamic);

        let result = engine
            .eval_with_scope::<rhai::Dynamic>(&mut scope, &self.script_source)
            .map_err(|e| SkillError::Message(format!("rhai 脚本执行失败: {}", e)))?;

        // 返回值尽量转成 JSON
        let out: Value = rhai::serde::from_dynamic(&result)
            .unwrap_or_else(|_| json!({"success": true, "result": result.to_string()}));
        Ok(out)
    }
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

    /// 将当前注册的 skills 暴露为 tools 列表，便于上层 runtime 自行组装。
    ///
    /// 说明：
    /// - `agentkit-runtime` 是可替换的，因此 `agentkit` 这个聚合 crate 不应强依赖默认 runtime。
    /// - 这里返回 `Vec<Arc<dyn Tool>>`（core 层 trait 对象），让上层自行决定如何构造 registry。
    pub fn as_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.skills
            .values()
            .cloned()
            .map(|skill| Arc::new(SkillTool::new(skill)) as Arc<dyn Tool>)
            .collect()
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
    load_skills_from_dir_inner(dir.as_ref(), get_global_rhai_engine_registrar()).await
}

async fn load_skills_from_dir_inner(
    dir: &Path,
    rhai_registrar: Option<RhaiEngineRegistrar>,
) -> Result<SkillRegistry, SkillError> {
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

        // 1) 优先加载 SKILL.rhai（blockcell 风格）
        let skill_rhai = path.join("SKILL.rhai");
        if fs::metadata(&skill_rhai).await.is_ok() {
            debug!(skills.skill_rhai = %skill_rhai.display(), "skills.load.read_skill_rhai");
            let script = fs::read_to_string(&skill_rhai)
                .await
                .map_err(|e| SkillError::Message(format!("读取 SKILL.rhai 失败: {}", e)))?;

            // 读取 meta.yaml / SKILL.md 作为“简短描述”来源（供大模型/tool schema 使用）
            let meta_yaml_path = path.join("meta.yaml");
            let meta: SkillMetaYaml = if fs::metadata(&meta_yaml_path).await.is_ok() {
                let meta_str = fs::read_to_string(&meta_yaml_path)
                    .await
                    .map_err(|e| SkillError::Message(format!("读取 meta.yaml 失败: {}", e)))?;
                serde_yaml::from_str(&meta_str)
                    .map_err(|e| SkillError::Message(format!("解析 meta.yaml 失败: {}", e)))?
            } else {
                SkillMetaYaml::default()
            };

            let skill_md = path.join("SKILL.md");
            let (name_from_md, desc_from_md) = if fs::metadata(&skill_md).await.is_ok() {
                let md = fs::read_to_string(&skill_md)
                    .await
                    .map_err(|e| SkillError::Message(format!("读取 SKILL.md 失败: {}", e)))?;
                parse_skill_md_frontmatter(&md)
            } else {
                (None, None)
            };

            // 默认用目录名作为 skill name
            let default_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("rhai_skill")
                .to_string();

            let name = meta.name.clone().or(name_from_md).unwrap_or(default_name);

            let description = format_meta_description(
                desc_from_md.or_else(|| Some("Rhai 脚本技能".to_string())),
                &meta,
            );

            info!(skills.name = %name, skills.kind = "rhai", "skills.load.register");
            registry = registry.register(RhaiSkill::new(
                name,
                description,
                script,
                rhai_registrar.clone(),
            ));

            // 一个目录里既有 SKILL.rhai 又有 SKILL.md 时，优先 rhai。
            continue;
        }

        // 2) 兼容 SKILL.md（命令模板）
        let skill_md = path.join("SKILL.md");
        if fs::metadata(&skill_md).await.is_err() {
            debug!(skills.dir_entry = %path.display(), "skills.load.skip_no_skill_file");
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
                info!(skills.name = %name, skills.kind = "command", "skills.load.register");
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
