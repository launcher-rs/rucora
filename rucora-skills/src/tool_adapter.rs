//! Skill 到 Tool 的适配器

use crate::{SkillConfig, SkillDefinition, SkillExecutor, SkillsPromptMode};
use async_trait::async_trait;
use rucora_core::error::ToolError;
use rucora_core::tool::{Tool, ToolCategory, types::ToolContext};
use serde_json::{Value, json};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Skill 工具适配器
pub struct SkillTool {
    skill: SkillDefinition,
    executor: Arc<SkillExecutor>,
    skill_path: PathBuf,
}

impl SkillTool {
    pub fn new(skill: SkillDefinition, executor: Arc<SkillExecutor>, skill_path: PathBuf) -> Self {
        let skill_path = if find_script_file(&skill_path).is_some() {
            skill_path
        } else if let Some(location) = skill.location.as_ref() {
            location.clone()
        } else {
            skill_path
        };

        Self {
            skill,
            executor,
            skill_path,
        }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        &self.skill.name
    }
    fn description(&self) -> Option<&str> {
        Some(&self.skill.description)
    }
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Custom("skill")]
    }
    fn input_schema(&self) -> Value {
        // 确保 input_schema 是完整的 JSON Schema 格式
        let mut schema = self.skill.input_schema.clone();

        // 如果是对象，确保有 type 字段
        if let Some(obj) = schema.as_object_mut() {
            if !obj.contains_key("type") {
                obj.insert("type".to_string(), json!("object"));
            }
        } else {
            // 如果 input_schema 不是对象，包装成对象
            schema = json!({
                "type": "object",
                "properties": self.skill.input_schema
            });
        }

        schema
    }

    async fn call(&self, input: Value, _context: &ToolContext) -> Result<Value, ToolError> {
        // 查找脚本文件
        let script_path = find_script_file(&self.skill_path);

        if let Some(path) = script_path {
            match self.executor.execute(&self.skill, &path, &input).await {
                Ok(result) => {
                    if result.success {
                        Ok(result
                            .data
                            .unwrap_or_else(|| Value::Object(serde_json::Map::new())))
                    } else {
                        Err(ToolError::Message(
                            result.error.unwrap_or_else(|| "Skill 执行失败".to_string()),
                        ))
                    }
                }
                Err(e) => Err(ToolError::Message(format!("Skill 执行错误：{e}"))),
            }
        } else {
            Err(ToolError::Message(format!(
                "未找到脚本实现：{:?}",
                self.skill_path
            )))
        }
    }
}

/// 查找 skill 目录中的脚本文件
fn find_script_file(skill_dir: &Path) -> Option<PathBuf> {
    // 优先级：Python > JavaScript > Shell
    let script_names = ["SKILL.py", "SKILL.js", "SKILL.sh"];

    for script_name in &script_names {
        let path = skill_dir.join(script_name);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[allow(clippy::needless_pass_by_value)]
/// 将 Skills 转换为 Tools
pub fn skills_to_tools(
    skills: &[SkillDefinition],
    executor: Arc<SkillExecutor>,
    skills_dir: &Path,
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

    for skill in skills {
        let Some(skill_path) = resolve_skill_dir(skill, skills_dir) else {
            continue;
        };

        // 检查是否有实现文件
        if let Some(_script_path) = find_script_file(&skill_path) {
            let tool = SkillTool::new(skill.clone(), executor.clone(), skill_path);
            tools.push(Arc::new(tool) as Arc<dyn Tool>);
        }
    }

    tools
}

/// XML 转义
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// 渲染 skill 位置
fn render_skill_location(
    skill: &SkillDefinition,
    workspace_dir: &Path,
    prefer_relative: bool,
) -> String {
    if let Some(location_path) = skill.location.as_ref() {
        if prefer_relative && let Ok(relative) = location_path.strip_prefix(workspace_dir) {
            return relative.display().to_string();
        }
        location_path.display().to_string()
    } else if let Some(ref location) = skill.homepage {
        let location_path = PathBuf::from(location);
        if prefer_relative && let Ok(relative) = location_path.strip_prefix(workspace_dir) {
            return relative.display().to_string();
        }
        location_path.display().to_string()
    } else {
        format!("skills/{}/SKILL.md", skill.name)
    }
}

/// 根据模式构建 Skills 提示词
pub fn skills_to_prompt_with_mode(
    skills: &[SkillDefinition],
    workspace_dir: &Path,
    mode: SkillsPromptMode,
) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut prompt = match mode {
        SkillsPromptMode::Full => String::from(
            "## Available Skills\n\n\
             Skill instructions are preloaded below. Follow these instructions directly.\n\n\
             <available_skills>\n",
        ),
        SkillsPromptMode::Compact => String::from(
            "## Available Skills\n\n\
             Skill summaries are preloaded. Call `read_skill(name)` for full instructions.\n\n\
             <available_skills>\n",
        ),
    };

    for skill in skills {
        let location = render_skill_location(
            skill,
            workspace_dir,
            matches!(mode, SkillsPromptMode::Compact),
        );
        let _ = writeln!(prompt, "  <skill>");
        let _ = writeln!(prompt, "    <name>{}</name>", xml_escape(&skill.name));
        let _ = writeln!(
            prompt,
            "    <description>{}</description>",
            xml_escape(&skill.description)
        );
        let _ = writeln!(prompt, "    <location>{}</location>", xml_escape(&location));
        let _ = writeln!(prompt, "  </skill>");
    }

    let _ = writeln!(prompt, "</available_skills>");

    if matches!(mode, SkillsPromptMode::Compact) {
        let _ = write!(prompt, "\n<callable_tools>\n");
        let _ = writeln!(prompt, "  <tool>");
        let _ = writeln!(prompt, "    <name>read_skill</name>");
        let _ = writeln!(
            prompt,
            "    <description>Read full skill file by name</description>"
        );
        let _ = writeln!(prompt, "    <parameters>");
        let _ = writeln!(prompt, "      <name>skill_name</name>");
        let _ = writeln!(prompt, "      <type>string</type>");
        let _ = writeln!(prompt, "    </parameters>");
        let _ = writeln!(prompt, "  </tool>");
        let _ = writeln!(prompt, "</callable_tools>");
    }

    prompt
}

/// read_skill 工具
pub struct ReadSkillTool {
    skills_dir: PathBuf,
}

impl ReadSkillTool {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }
}

#[async_trait]
impl Tool for ReadSkillTool {
    fn name(&self) -> &str {
        "read_skill"
    }
    fn description(&self) -> Option<&str> {
        Some("Read full skill file by name")
    }
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Custom("skill")]
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "skill_name": {
                    "type": "string",
                    "description": "The name of the skill to read"
                }
            },
            "required": ["skill_name"]
        })
    }

    async fn call(&self, input: Value, _context: &ToolContext) -> Result<Value, ToolError> {
        let skill_name = input
            .get("skill_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("Missing skill_name parameter".to_string()))?;

        match read_skill(skill_name, &self.skills_dir) {
            Ok(content) => Ok(json!({ "success": true, "content": content })),
            Err(e) => Ok(json!({ "success": false, "error": e.to_string() })),
        }
    }
}

/// 读取 skill 详细信息
pub fn read_skill(
    skill_name: &str,
    skills_dir: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(skill_path) = find_skill_dir_by_name(skill_name, skills_dir) else {
        return Err(format!("Skill not found: {skill_name}").into());
    };

    // 优先级：skill.yaml > skill.toml > skill.json > SKILL.md > skill.md
    for config_file in &["skill.yaml", "skill.toml", "skill.json"] {
        let path = skill_path.join(config_file);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return Ok(format!("=== {config_file} ===\n{content}"));
        }
    }

    for md_file in ["SKILL.md", "skill.md"] {
        let md_path = skill_path.join(md_file);
        if !md_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&md_path)?;
        return Ok(format!("=== {md_file} ===\n{content}"));
    }

    Err(format!("No skill file found in {skill_path:?}").into())
}

fn resolve_skill_dir(skill: &SkillDefinition, skills_dir: &Path) -> Option<PathBuf> {
    if let Some(location) = skill.location.as_ref()
        && location.exists()
    {
        return Some(location.clone());
    }

    if let Some(homepage) = skill.homepage.as_ref() {
        let path = PathBuf::from(homepage);
        if path.exists() {
            return Some(path);
        }
    }

    find_skill_dir_by_name(&skill.name, skills_dir)
}

fn find_skill_dir_by_name(skill_name: &str, skills_dir: &Path) -> Option<PathBuf> {
    let direct_path = skills_dir.join(skill_name);
    if direct_path.exists() {
        return Some(direct_path);
    }

    let entries = std::fs::read_dir(skills_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if read_skill_name_from_dir(&path).as_deref() == Some(skill_name) {
            return Some(path);
        }
    }

    None
}

fn read_skill_name_from_dir(skill_dir: &Path) -> Option<String> {
    for config_file in ["skill.yaml", "skill.toml", "skill.json"] {
        let path = skill_dir.join(config_file);
        if !path.exists() {
            continue;
        }

        if let Some(name) = read_skill_name_from_config(&path) {
            return Some(name);
        }
    }

    for md_file in ["SKILL.md", "skill.md"] {
        let path = skill_dir.join(md_file);
        if !path.exists() {
            continue;
        }

        if let Some(name) = read_skill_name_from_md(&path) {
            return Some(name);
        }
    }

    None
}

fn read_skill_name_from_config(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "yaml" | "yml" => serde_yaml::from_str::<SkillDefinition>(&content)
            .map(|skill| skill.name)
            .or_else(|_| {
                serde_yaml::from_str::<SkillConfig>(&content).map(|config| config.skill.name)
            })
            .ok(),
        "toml" => toml::from_str::<SkillDefinition>(&content)
            .map(|skill| skill.name)
            .or_else(|_| toml::from_str::<SkillConfig>(&content).map(|config| config.skill.name))
            .ok(),
        "json" => serde_json::from_str::<SkillDefinition>(&content)
            .map(|skill| skill.name)
            .or_else(|_| {
                serde_json::from_str::<SkillConfig>(&content).map(|config| config.skill.name)
            })
            .ok(),
        _ => None,
    }
}

fn read_skill_name_from_md(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    if lines.next()? != "---" {
        return None;
    }

    let mut frontmatter = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }

    serde_yaml::from_str::<SkillDefinition>(&frontmatter)
        .map(|skill| skill.name)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_skills_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("系统时间应晚于 UNIX_EPOCH")
            .as_nanos();
        std::env::temp_dir().join(format!("rucora-skills-adapter-{name}-{nanos}"))
    }

    #[test]
    fn skills_to_tools_uses_loaded_location_when_name_differs_from_dir() {
        let skills_dir = temp_skills_dir("location");
        let skill_dir = skills_dir.join("weather");
        fs::create_dir_all(&skill_dir).expect("应能创建临时 skill 目录");
        fs::write(skill_dir.join("SKILL.py"), "print('{}')").expect("应能写入脚本文件");

        let mut skill = SkillDefinition::new("weather-query", "查询天气");
        skill.location = Some(skill_dir);

        let tools = skills_to_tools(&[skill], Arc::new(SkillExecutor::new()), &skills_dir);

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "weather-query");

        fs::remove_dir_all(&skills_dir).expect("应能清理临时 skills 目录");
    }
}
