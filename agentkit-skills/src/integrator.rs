//! Skills 与 Agent 自动集成模块

use crate::loader::SkillLoader;
use agentkit_core::skill::SkillDefinition;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use tracing::{debug, info};

/// Skills 自动集成器
pub struct SkillsAutoIntegrator {
    loader: SkillLoader,
    detected_tools: HashSet<String>,
}

impl SkillsAutoIntegrator {
    /// 创建新的集成器
    pub fn new(skills_dir: impl AsRef<Path>) -> Self {
        Self {
            loader: SkillLoader::new(skills_dir),
            detected_tools: HashSet::new(),
        }
    }

    /// 加载 Skills 并分析需要的工具
    pub async fn load_and_analyze(
        &mut self,
    ) -> Result<Vec<SkillDefinition>, Box<dyn std::error::Error>> {
        // 1. 加载 Skills
        let skills = self.loader.load_from_dir().await?;
        info!("加载了 {} 个 Skills", skills.len());

        // 2. 分析每个 Skill 需要的工具
        for skill in &skills {
            let required_tools = self.analyze_skill_requirements(skill);

            for tool_name in &required_tools {
                self.detected_tools.insert(tool_name.clone());
            }
        }

        if !self.detected_tools.is_empty() {
            info!("检测到需要的工具：{:?}", self.detected_tools);
        }

        Ok(skills)
    }

    /// 分析 Skill 需要的工具
    fn analyze_skill_requirements(&self, skill: &SkillDefinition) -> Vec<String> {
        let mut required_tools = Vec::new();

        // 从 metadata 中读取需要的工具
        if let Some(metadata) = &skill.metadata {
            if let Some(Value::Object(requires_map)) = metadata.get("requires") {
                // 检查需要的 bins（命令行工具）
                if let Some(Value::Array(bins_array)) = requires_map.get("bins") {
                    for bin in bins_array {
                        if let Some(bin_name) = bin.as_str() {
                            required_tools.push(format!("cmd_{}", bin_name));
                        }
                    }
                }
            }
        }

        debug!("Skill {} 需要的工具：{:?}", skill.name, required_tools);

        required_tools
    }

    /// 获取检测到的工具列表
    pub fn detected_tools(&self) -> &HashSet<String> {
        &self.detected_tools
    }

    /// 获取 Skill 加载器
    pub fn loader(&self) -> &SkillLoader {
        &self.loader
    }

    /// 获取所有 Skills 的工具描述（用于 LLM）
    pub fn to_tool_descriptions(&self) -> Vec<Value> {
        self.loader.to_tool_descriptions()
    }
}

/// Skill 工具适配器（简化版）
pub struct SkillToolAdapter {
    skill_name: String,
    description: String,
}

impl SkillToolAdapter {
    /// 创建新的适配器
    pub fn new(skill: &SkillDefinition) -> Self {
        Self {
            skill_name: skill.name.clone(),
            description: skill.description.clone(),
        }
    }

    /// 获取技能名称
    pub fn name(&self) -> &str {
        &self.skill_name
    }

    /// 获取描述
    pub fn description(&self) -> &str {
        &self.description
    }
}


