//! Skill 到 Tool 的适配器

use crate::{SkillDefinition, SkillExecutor, SkillsPromptMode};
use rucora_core::error::ToolError;
use rucora_core::tool::{Tool, ToolCategory};
use async_trait::async_trait;
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

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
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

/// 将 Skills 转换为 Tools
pub fn skills_to_tools(
    skills: &[SkillDefinition],
    executor: Arc<SkillExecutor>,
    skills_dir: &Path,
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

    for skill in skills {
        // 查找 skill 目录（支持 skill.name 和文件夹名不一致）
        let mut skill_path = skills_dir.join(&skill.name);
        if !skill_path.exists() {
            // 尝试使用文件夹名作为 skill 名称（去掉连字符后的第一部分）
            skill_path = skills_dir.join(skill.name.split('-').next().unwrap_or(&skill.name));
        }

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
    if let Some(ref location) = skill.homepage {
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

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
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
    // 查找 skill 目录
    let mut skill_path = skills_dir.join(skill_name);
    if !skill_path.exists() {
        skill_path = skills_dir.join(skill_name.split('-').next().unwrap_or(skill_name));
    }

    if !skill_path.exists() {
        return Err(format!("Skill not found: {skill_name}").into());
    }

    // 优先级：skill.yaml > skill.toml > skill.json > SKILL.md
    for config_file in &["skill.yaml", "skill.toml", "skill.json"] {
        let path = skill_path.join(config_file);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            return Ok(format!("=== {config_file} ===\n{content}"));
        }
    }

    // 读取 SKILL.md
    let md_path = skill_path.join("SKILL.md");
    if md_path.exists() {
        let content = std::fs::read_to_string(&md_path)?;
        return Ok(format!("=== SKILL.md ===\n{content}"));
    }

    Err(format!("No skill file found in {skill_path:?}").into())
}
