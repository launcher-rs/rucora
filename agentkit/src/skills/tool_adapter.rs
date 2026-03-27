//! Skill 到 Tool 的适配器
//!
//! 借鉴 zeroclaw 项目的 Skill 设计，将 Skill 包装成 Tool trait

use agentkit_core::tool::{Tool, ToolCategory};
use agentkit_core::error::ToolError;
use serde_json::Value;
use async_trait::async_trait;
use std::sync::Arc;
use crate::skills::{SkillDefinition, SkillExecutor};

/// Skill 工具适配器
/// 
/// 参考 zeroclaw 项目的 Skill 设计：
/// - Skill 可以定义多个 tools
/// - 每个 tool 有 name, description, kind, command
/// - 支持 shell/http/script 等多种类型
pub struct SkillTool {
    /// Skill 定义
    skill: SkillDefinition,
    /// Skill 执行器
    executor: Arc<SkillExecutor>,
    /// Skill 目录路径
    skill_path: std::path::PathBuf,
}

impl SkillTool {
    /// 创建新的 Skill 工具
    pub fn new(
        skill: SkillDefinition,
        executor: Arc<SkillExecutor>,
        skill_path: std::path::PathBuf,
    ) -> Self {
        Self {
            skill,
            executor,
            skill_path,
        }
    }
    
    /// 获取 Skill 名称
    pub fn skill_name(&self) -> &str {
        &self.skill.name
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
        // 使用 skill 的 input_schema，如果没有则返回空对象
        self.skill.input_schema.clone()
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        // 执行 Skill
        match self.executor.execute(&self.skill, &self.skill_path, &input).await {
            Ok(result) => {
                if result.success {
                    // 返回成功结果
                    Ok(result.data.unwrap_or_else(|| Value::Object(serde_json::Map::new())))
                } else {
                    // 返回错误
                    Err(ToolError::Message(
                        result.error.unwrap_or_else(|| "Skill 执行失败".to_string())
                    ))
                }
            }
            Err(e) => {
                Err(ToolError::Message(format!("Skill 执行错误：{}", e)))
            }
        }
    }
}

/// 将 Skills 转换为 Tools
/// 
/// 参考 zeroclaw 的 load_skills 函数：
/// - 支持 skill.name 和文件夹名不一致
/// - 检查实现文件是否存在
/// - 返回 Arc<dyn Tool> 以便注册到 Agent
pub fn skills_to_tools(
    skills: &[SkillDefinition],
    executor: Arc<SkillExecutor>,
    skills_dir: &std::path::Path,
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    
    for skill in skills {
        // 查找 Skill 目录（支持 skill.name 和文件夹名不一致）
        let mut skill_path = skills_dir.join(&skill.name);
        if !skill_path.exists() {
            // 尝试使用文件夹名作为 skill 名称
            skill_path = skills_dir.join(skill.name.split('-').next().unwrap_or(&skill.name));
        }
        
        if skill_path.exists() {
            // 检查是否有实现文件（参考 zeroclaw 的审计逻辑）
            let has_implementation = skill_path.join("SKILL.py").exists()
                || skill_path.join("SKILL.js").exists()
                || skill_path.join("SKILL.sh").exists();
            
            if has_implementation {
                let tool = SkillTool::new(skill.clone(), executor.clone(), skill_path);
                tools.push(Arc::new(tool) as Arc<dyn Tool>);
            }
        }
    }
    
    tools
}

/// 生成 Skills 提示词（参考 zeroclaw 的 skills_to_prompt）
pub fn skills_to_prompt(skills: &[SkillDefinition], _workspace_dir: &std::path::Path) -> String {
    if skills.is_empty() {
        return String::new();
    }
    
    let mut prompt = String::from("<available_skills>\n");
    
    for skill in skills {
        prompt.push_str(&format!(
            "  <skill>\n    <name>{}</name>\n    <description>{}</description>\n  </skill>\n",
            xml_escape(&skill.name),
            xml_escape(&skill.description)
        ));
    }
    
    prompt.push_str("</available_skills>\n");
    prompt
}

/// XML 转义（参考 zeroclaw 的 XML 转义逻辑）
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
