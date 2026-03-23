//! 命令技能实现。
//!
//! 本模块提供基于命令模板的技能实现，通过解析 SKILL.md 文件中的命令模板来执行操作。

#[cfg(feature = "rhai-skills")]
use agentkit_core::{error::SkillError, tool::ToolCategory};
#[cfg(feature = "rhai-skills")]
use async_trait::async_trait;
#[cfg(feature = "rhai-skills")]
use serde::Deserialize;
#[cfg(feature = "rhai-skills")]
use serde_json::{json, Value};
#[cfg(feature = "rhai-skills")]
use std::path::Path;
#[cfg(feature = "rhai-skills")]
use tracing::debug;

#[cfg(feature = "rhai-skills")]
pub use agentkit_core::skill::Skill;

/// SKILL.md 文件中的 YAML frontmatter 元数据结构。
#[cfg(feature = "rhai-skills")]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SkillMetaYaml {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub requires: Option<SkillRequires>,

    #[serde(default)]
    pub permissions: Vec<String>,
}

/// 技能依赖声明结构。
#[cfg(feature = "rhai-skills")]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SkillRequires {
    #[serde(default)]
    pub bins: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
}

/// 技能清单：描述技能的基本信息和依赖。
#[cfg(feature = "rhai-skills")]
#[derive(Debug, Clone)]
pub struct SkillManifest {
    /// 技能名称（建议唯一）。
    pub name: String,
    /// 描述（可选）。
    pub description: Option<String>,
    /// 版本（建议语义化版本）。
    pub version: String,
    /// 触发词（可选，用于路由/检索）。
    pub triggers: Vec<String>,
    /// 能力声明（可选，用于提示/权限/依赖判断）。
    pub capabilities: Vec<String>,
    /// 外部二进制依赖（例如 curl）。
    pub requires_bins: Vec<String>,
    /// 运行所需环境变量（用于提醒/校验）。
    pub requires_env: Vec<String>,
    /// 权限声明（为后续 policy/审计预留）。
    pub permissions: Vec<String>,
}

/// 验证技能清单的有效性。
///
/// 说明：这里先做最小校验，确保技能至少具备可识别的 name/version。
/// 后续如果引入更严格的依赖/权限模型，可在此扩展。
#[cfg(feature = "rhai-skills")]
pub fn validate_manifest(manifest: &SkillManifest, location: &Path) -> Result<(), SkillError> {
    if manifest.name.trim().is_empty() {
        return Err(SkillError::Message(format!(
            "skill manifest 校验失败：name 为空（dir={}）",
            location.display()
        )));
    }
    if manifest.version.trim().is_empty() {
        return Err(SkillError::Message(format!(
            "skill manifest 校验失败：version 为空（skill={} dir={}）",
            manifest.name,
            location.display()
        )));
    }
    Ok(())
}

/// 将 YAML 元数据转换为技能清单。
#[cfg(feature = "rhai-skills")]
pub fn meta_yaml_to_manifest(
    default_name: String,
    desc_from_md: Option<String>,
    meta: SkillMetaYaml,
) -> SkillManifest {
    let name = meta.name.clone().unwrap_or(default_name);
    let version = meta.version.clone().unwrap_or_else(|| "0.1.0".to_string());
    let description = format_meta_description(desc_from_md, &meta);
    let requires_bins = meta
        .requires
        .as_ref()
        .map(|r| r.bins.clone())
        .unwrap_or_default();
    let requires_env = meta
        .requires
        .as_ref()
        .map(|r| r.env.clone())
        .unwrap_or_default();
    SkillManifest {
        name,
        description,
        version,
        triggers: meta.triggers,
        capabilities: meta.capabilities,
        requires_bins,
        requires_env,
        permissions: meta.permissions,
    }
}

/// 格式化技能描述，合并来自 frontmatter 的额外信息。
#[cfg(feature = "rhai-skills")]
fn format_meta_description(base: Option<String>, meta: &SkillMetaYaml) -> Option<String> {
    let mut desc = base.or_else(|| meta.description.clone());

    let mut extra: Vec<String> = Vec::new();
    if !meta.triggers.is_empty() {
        extra.push(format!("触发词：{}", meta.triggers.join(", ")));
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

/// 基于 `SKILL.md` 的通用命令型 skill：执行命令并返回 stdout/stderr。
///
/// 这里不为每个 skill 写一个专用 struct（例如 WeatherSkill），而是让 SKILL.md 驱动执行流程。
#[cfg(feature = "rhai-skills")]
pub struct CommandSkill {
    pub name: String,
    pub description: Option<String>,
    pub command_template: String,
}

#[cfg(feature = "rhai-skills")]
impl CommandSkill {
    /// 创建新的命令技能实例。
    pub fn new(name: String, description: Option<String>, command_template: String) -> Self {
        Self {
            name,
            description,
            command_template,
        }
    }

    /// 根据输入渲染命令模板。
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

#[cfg(feature = "rhai-skills")]
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
        debug!(skill.name = %self.name, cmd.command = %cmd, "command_skill.exec");

        // Skill 内部通过 tool 来执行命令（符合"skills 执行时调用工具执行命令"的约束）。
        let tool = crate::tools::CmdExecTool::new();
        let out = tool
            .call(json!({"command": cmd}))
            .await
            .map_err(|e| SkillError::Message(format!("cmd_exec tool 执行失败：{}", e)))?;

        Ok(json!({
            "skill": self.name,
            "tool": "cmd_exec",
            "result": out
        }))
    }
}

/// 解析 SKILL.md 文件的 YAML frontmatter。
///
/// 返回 (name, description) 元组。
#[cfg(feature = "rhai-skills")]
pub fn parse_skill_md_frontmatter(md: &str) -> (Option<String>, Option<String>) {
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

/// 从 SKILL.md 内容中提取主要的命令模板。
///
/// 约定：从第一个 ```bash 代码块中提取第一条以 curl 开头的命令。
#[cfg(feature = "rhai-skills")]
pub fn extract_primary_command_template(md: &str) -> Option<String> {
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
            let mut cmd = t.to_string();
            cmd = cmd.replace("wttr.in/London?format=3", "wttr.in/{location}?format=3");
            cmd = cmd.replace("wttr.in/London?T", "wttr.in/{location}?T");
            cmd = cmd.replace("wttr.in/London", "wttr.in/{location}");
            return Some(cmd);
        }
    }

    None
}
