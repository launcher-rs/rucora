//! 技能注册表和加载逻辑。
//!
//! 本模块提供技能注册表管理和从目录加载技能的功能。

use agentkit_core::{
    error::{SkillError, ToolError},
    skill::Skill,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

#[cfg(feature = "rhai-skills")]
use crate::skills::command_skills::{
    CommandSkill, SkillManifest, SkillMetaYaml, extract_primary_command_template,
    parse_skill_md_frontmatter, validate_manifest,
};
#[cfg(feature = "rhai-skills")]
use crate::skills::file_skills::FileReadSkill;
#[cfg(feature = "rhai-skills")]
use crate::skills::rhai_skills::{
    RhaiEngineRegistrar, RhaiSkill, get_global_rhai_engine_registrar,
};
#[cfg(feature = "rhai-skills")]
use tokio::fs;
#[cfg(feature = "rhai-skills")]
use tracing::info;

/// Skill 注册表：集中管理所有可用 skills。
///
/// 在运行前（启动阶段）注册所有 skills，然后在运行时把它们转换成 `ToolRegistry`
/// 交给 `ToolCallingAgent`。
#[derive(Default, Clone)]
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
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
    pub fn as_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.skills
            .values()
            .cloned()
            .map(|skill| Arc::new(SkillTool::new(skill)) as Arc<dyn Tool>)
            .collect()
    }

    /// 获取已注册的技能数量。
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// 检查是否为空。
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

/// 从工作区 `skills/` 目录加载 skills。
///
/// 约定：每个 skill 是一个子目录，包含 `SKILL.md` 或 `SKILL.rhai`。
///
/// - SKILL.md -> `CommandSkill`
/// - SKILL.rhai -> `RhaiSkill` (需要 `rhai-skills` feature)
pub async fn load_skills_from_dir(_dir: impl AsRef<Path>) -> Result<SkillRegistry, SkillError> {
    #[cfg(feature = "rhai-skills")]
    {
        load_skills_from_dir_inner(_dir.as_ref(), get_global_rhai_engine_registrar()).await
    }
    #[cfg(not(feature = "rhai-skills"))]
    {
        // 无 Rhai 支持时，返回空注册表
        tracing::warn!(
            "skills loaded without rhai-skills feature - SKILL.rhai files will be ignored"
        );
        Ok(SkillRegistry::new())
    }
}

/// 从工作区 `skills/` 目录加载 skills（带 Rhai 注册器）。
#[cfg(feature = "rhai-skills")]
pub async fn load_skills_from_dir_with_rhai(
    dir: impl AsRef<Path>,
    rhai_registrar: Option<RhaiEngineRegistrar>,
) -> Result<SkillRegistry, SkillError> {
    load_skills_from_dir_inner(dir.as_ref(), rhai_registrar).await
}

#[cfg(feature = "rhai-skills")]
async fn load_skills_from_dir_inner(
    dir: &Path,
    rhai_registrar: Option<RhaiEngineRegistrar>,
) -> Result<SkillRegistry, SkillError> {
    let mut registry = SkillRegistry::new();

    info!(skills.dir = %dir.display(), "skills.load.start");

    let mut rd = fs::read_dir(dir)
        .await
        .map_err(|e| SkillError::Message(format!("读取 skills 目录失败：{}", e)))?;

    while let Some(entry) = rd
        .next_entry()
        .await
        .map_err(|e| SkillError::Message(format!("遍历 skills 目录失败：{}", e)))?
    {
        let path = entry.path();
        let ty = entry
            .file_type()
            .await
            .map_err(|e| SkillError::Message(format!("读取目录项类型失败：{}", e)))?;

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
                .map_err(|e| SkillError::Message(format!("读取 SKILL.rhai 失败：{}", e)))?;

            // 读取 meta.yaml / SKILL.md 作为"简短描述"来源
            let meta_yaml_path = path.join("meta.yaml");
            let meta: SkillMetaYaml = if fs::metadata(&meta_yaml_path).await.is_ok() {
                let meta_str = fs::read_to_string(&meta_yaml_path)
                    .await
                    .map_err(|e| SkillError::Message(format!("读取 meta.yaml 失败：{}", e)))?;
                serde_yaml::from_str(&meta_str)
                    .map_err(|e| SkillError::Message(format!("解析 meta.yaml 失败：{}", e)))?
            } else {
                SkillMetaYaml::default()
            };

            let skill_md = path.join("SKILL.md");
            let (name_from_md, desc_from_md) = if fs::metadata(&skill_md).await.is_ok() {
                let md = fs::read_to_string(&skill_md)
                    .await
                    .map_err(|e| SkillError::Message(format!("读取 SKILL.md 失败：{}", e)))?;
                parse_skill_md_frontmatter(&md)
            } else {
                (None, None)
            };

            let default_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("rhai_skill")
                .to_string();

            let manifest = meta_yaml_to_manifest(
                name_from_md.unwrap_or(default_name),
                desc_from_md.or_else(|| Some("Rhai 脚本技能".to_string())),
                meta,
            );
            validate_manifest(&manifest, &path)?;

            let name = manifest.name.clone();
            let description = manifest.description.clone();

            info!(skills.name = %name, skills.kind = "rhai", "skills.load.register");
            registry = registry.register(RhaiSkill::new(
                name,
                description,
                script,
                rhai_registrar.clone(),
            ));

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
            .map_err(|e| SkillError::Message(format!("读取 SKILL.md 失败：{}", e)))?;

        let (name, description) = parse_skill_md_frontmatter(&md);
        let command_template = extract_primary_command_template(&md);

        debug!(
            skills.skill_md = %skill_md.display(),
            skills.name = name.as_deref().unwrap_or(""),
            skills.description = description.as_deref().unwrap_or(""),
            "skills.load.frontmatter"
        );

        if let Some(name) = name {
            if let Some(tpl) = command_template {
                info!(skills.name = %name, skills.kind = "command", "skills.load.register");
                let manifest = SkillManifest {
                    name: name.clone(),
                    description: description.clone(),
                    version: "0.1.0".to_string(),
                    triggers: Vec::new(),
                    capabilities: Vec::new(),
                    requires_bins: Vec::new(),
                    requires_env: Vec::new(),
                    permissions: Vec::new(),
                };
                validate_manifest(&manifest, &path)?;
                registry = registry.register(CommandSkill::new(name, description, tpl));
            }
        }
    }

    // 注册内置技能
    registry = registry.register(FileReadSkill::new());

    info!(skills.count = registry.skills.len(), "skills.load.done");
    Ok(registry)
}

/// Skill 到 Tool 的通用适配器。
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
        debug!(skill.name = %skill_name, "skill_tool.call.start");

        let start = std::time::Instant::now();
        let out = self
            .skill
            .run_value(input)
            .await
            .map_err(|e| ToolError::Message(e.to_string()))?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            skill.name = %skill_name,
            skill.elapsed_ms = elapsed_ms,
            "skill_tool.call.done"
        );

        Ok(out)
    }
}
