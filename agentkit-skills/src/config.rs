//! Skill 配置模块
//!
//! 支持多种配置文件格式：YAML, TOML, JSON
//! 支持按需加载、配置验证、配置合并等功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Skill 配置（支持 skill.yaml, skill.toml, skill.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillConfig {
    /// 技能基本信息
    #[serde(default)]
    pub skill: SkillMeta,

    /// 输入 Schema
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,

    /// 输出 Schema
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,

    /// 执行配置
    #[serde(default)]
    pub execution: Option<ExecutionConfig>,

    /// 触发器配置
    #[serde(default)]
    pub triggers: Vec<String>,

    /// 权限配置
    #[serde(default)]
    pub permissions: Option<PermissionsConfig>,

    /// 依赖配置
    #[serde(default)]
    pub dependencies: Option<DependenciesConfig>,

    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 技能元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillMeta {
    /// 技能名称
    pub name: String,

    /// 技能描述
    #[serde(default)]
    pub description: String,

    /// 版本号
    #[serde(default = "default_version")]
    pub version: String,

    /// 作者
    #[serde(default)]
    pub author: Option<String>,

    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// 执行配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionConfig {
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// 工作目录
    #[serde(default)]
    pub work_dir: Option<String>,

    /// 重试次数
    #[serde(default)]
    pub retries: u32,

    /// 是否缓存结果
    #[serde(default)]
    pub cache: bool,
}

fn default_timeout() -> u64 {
    30
}

/// 权限配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    /// 网络访问权限
    #[serde(default)]
    pub network: bool,

    /// 文件系统访问权限
    #[serde(default)]
    pub filesystem: bool,

    /// 允许的命令列表
    #[serde(default)]
    pub commands: Vec<String>,

    /// 允许的域名白名单
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// 禁止的域名黑名单
    #[serde(default)]
    pub denied_domains: Vec<String>,
}

/// 依赖配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependenciesConfig {
    /// 需要的二进制文件
    #[serde(default)]
    pub bins: Vec<String>,

    /// 需要的环境变量
    #[serde(default)]
    pub env: Vec<String>,

    /// 需要的 Python 包
    #[serde(default)]
    pub python_packages: Vec<String>,
}

/// 配置加载选项（按需加载）
#[derive(Debug, Clone, Default)]
pub struct ConfigLoadOptions {
    /// 是否加载基本信息
    pub load_basic: bool,
    /// 是否加载输入输出 Schema
    pub load_schema: bool,
    /// 是否加载执行配置
    pub load_execution: bool,
    /// 是否加载触发器
    pub load_triggers: bool,
    /// 是否加载权限配置
    pub load_permissions: bool,
    /// 是否加载依赖配置
    pub load_dependencies: bool,
    /// 是否加载元数据
    pub load_metadata: bool,
}

impl ConfigLoadOptions {
    /// 创建只加载基本信息的选项（用于 LLM 调用）
    pub fn for_llm() -> Self {
        Self {
            load_basic: true,
            load_schema: true,
            ..Default::default()
        }
    }

    /// 创建只加载执行配置的选项（用于技能执行）
    pub fn for_execution() -> Self {
        Self {
            load_basic: true,
            load_execution: true,
            load_permissions: true,
            load_dependencies: true,
            ..Default::default()
        }
    }

    /// 创建只加载注册信息的选项（用于技能注册）
    pub fn for_registration() -> Self {
        Self {
            load_basic: true,
            load_triggers: true,
            ..Default::default()
        }
    }

    /// 创建只加载搜索信息的选项（用于技能搜索）
    pub fn for_search() -> Self {
        Self {
            load_basic: true,
            load_triggers: true,
            ..Default::default()
        }
    }

    /// 创建加载所有信息的选项
    pub fn full() -> Self {
        Self {
            load_basic: true,
            load_schema: true,
            load_execution: true,
            load_triggers: true,
            load_permissions: true,
            load_dependencies: true,
            load_metadata: true,
        }
    }
}

/// 配置验证错误
#[derive(Debug, Clone)]
pub struct ConfigError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ConfigError {}

impl SkillConfig {
    /// 从文件加载配置（自动检测格式）
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Ok(serde_yaml::from_str(&content)?),
            "toml" => Ok(toml::from_str(&content)?),
            "json" => Ok(serde_json::from_str(&content)?),
            _ => Err(format!("Unsupported config format: {}", ext).into()),
        }
    }

    /// 从文件加载配置（按需加载）
    pub fn from_file_with_options(
        path: &Path,
        options: &ConfigLoadOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::from_file(path)?;
        config.apply_options(options);
        Ok(config)
    }

    /// 应用加载选项（清理不需要的字段）
    pub fn apply_options(&mut self, options: &ConfigLoadOptions) {
        if !options.load_schema {
            self.input_schema = None;
            self.output_schema = None;
        }

        if !options.load_execution {
            self.execution = None;
        }

        if !options.load_triggers {
            self.triggers.clear();
        }

        if !options.load_permissions {
            self.permissions = None;
        }

        if !options.load_dependencies {
            self.dependencies = None;
        }

        if !options.load_metadata {
            self.metadata.clear();
        }
    }

    /// 尝试从目录加载配置（按优先级尝试不同格式）
    pub fn from_dir(dir: &Path) -> Option<Self> {
        // 优先级：skill.yaml > skill.toml > skill.json
        let formats = ["skill.yaml", "skill.toml", "skill.json"];

        for format in &formats {
            let path = dir.join(format);
            if path.exists()
                && let Ok(config) = Self::from_file(&path) {
                    return Some(config);
                }
        }
        None
    }

    /// 从目录加载配置（按需加载）
    pub fn from_dir_with_options(dir: &Path, options: &ConfigLoadOptions) -> Option<Self> {
        let formats = ["skill.yaml", "skill.toml", "skill.json"];

        for format in &formats {
            let path = dir.join(format);
            if path.exists()
                && let Ok(mut config) = Self::from_file(&path) {
                    config.apply_options(options);
                    return Some(config);
                }
        }
        None
    }

    /// 合并另一个配置（当前配置优先）
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            skill: SkillMeta {
                name: if self.skill.name.is_empty() {
                    other.skill.name.clone()
                } else {
                    self.skill.name.clone()
                },
                description: if self.skill.description.is_empty() {
                    other.skill.description.clone()
                } else {
                    self.skill.description.clone()
                },
                version: if self.skill.version.is_empty() || self.skill.version == "0.1.0" {
                    other.skill.version.clone()
                } else {
                    self.skill.version.clone()
                },
                author: self
                    .skill
                    .author
                    .clone()
                    .or_else(|| other.skill.author.clone()),
                tags: {
                    let mut tags = self.skill.tags.clone();
                    tags.extend(other.skill.tags.iter().cloned());
                    tags.sort();
                    tags.dedup();
                    tags
                },
            },
            input_schema: self
                .input_schema
                .clone()
                .or_else(|| other.input_schema.clone()),
            output_schema: self
                .output_schema
                .clone()
                .or_else(|| other.output_schema.clone()),
            execution: self.execution.clone().or_else(|| other.execution.clone()),
            triggers: {
                let mut triggers = self.triggers.clone();
                triggers.extend(other.triggers.iter().cloned());
                triggers.sort();
                triggers.dedup();
                triggers
            },
            permissions: self
                .permissions
                .clone()
                .or_else(|| other.permissions.clone()),
            dependencies: self
                .dependencies
                .clone()
                .or_else(|| other.dependencies.clone()),
            metadata: {
                let mut metadata = self.metadata.clone();
                metadata.extend(other.metadata.iter().map(|(k, v)| (k.clone(), v.clone())));
                metadata
            },
        }
    }

    /// 获取技能的显示名称
    pub fn display_name(&self) -> &str {
        if self.skill.name.is_empty() {
            "Unknown"
        } else {
            &self.skill.name
        }
    }

    /// 获取技能的描述
    pub fn display_description(&self) -> &str {
        if self.skill.description.is_empty() {
            "No description provided"
        } else {
            &self.skill.description
        }
    }

    /// 检查配置是否有效
    pub fn validate(&self) -> Result<(), Vec<ConfigError>> {
        let mut errors = Vec::new();

        // 验证 name
        if self.skill.name.is_empty() {
            errors.push(ConfigError {
                field: "skill.name".to_string(),
                message: "is required".to_string(),
            });
        } else if self.skill.name.len() < 3 || self.skill.name.len() > 50 {
            errors.push(ConfigError {
                field: "skill.name".to_string(),
                message: "must be 3-50 characters".to_string(),
            });
        } else if !self
            .skill
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            errors.push(ConfigError {
                field: "skill.name".to_string(),
                message: "must contain only lowercase letters, digits, and hyphens".to_string(),
            });
        }

        // 验证 description
        if self.skill.description.is_empty() {
            errors.push(ConfigError {
                field: "skill.description".to_string(),
                message: "is required".to_string(),
            });
        } else if self.skill.description.len() > 500 {
            errors.push(ConfigError {
                field: "skill.description".to_string(),
                message: "must be less than 500 characters".to_string(),
            });
        }

        // 验证 version
        if !self.skill.version.is_empty() && self.skill.version != "0.1.0" {
            // 简单的语义化版本验证
            let parts: Vec<&str> = self.skill.version.split('.').collect();
            if parts.len() != 3 || !parts.iter().all(|p| p.parse::<u32>().is_ok()) {
                errors.push(ConfigError {
                    field: "skill.version".to_string(),
                    message: "must follow semantic versioning (e.g., 1.0.0)".to_string(),
                });
            }
        }

        // 验证 timeout
        if let Some(exec) = &self.execution {
            if exec.timeout == 0 {
                errors.push(ConfigError {
                    field: "execution.timeout".to_string(),
                    message: "must be greater than 0".to_string(),
                });
            } else if exec.timeout > 300 {
                errors.push(ConfigError {
                    field: "execution.timeout".to_string(),
                    message: "must be less than 300 seconds".to_string(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 检查是否有某个标签
    pub fn has_tag(&self, tag: &str) -> bool {
        self.skill.tags.iter().any(|t| t == tag)
    }

    /// 检查是否匹配触发器
    pub fn matches_trigger(&self, keyword: &str) -> bool {
        self.triggers
            .iter()
            .any(|t| t.contains(keyword) || keyword.contains(t))
    }

    /// 获取配置的摘要信息
    pub fn summary(&self) -> String {
        format!(
            "{} v{} - {}",
            self.display_name(),
            self.skill.version,
            self.display_description()
        )
    }

    /// 转换为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 从 JSON 字符串加载
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_minimal_config() {
        let config = SkillConfig {
            skill: SkillMeta {
                name: "test-skill".to_string(),
                description: "Test skill".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_name() {
        let config = SkillConfig {
            skill: SkillMeta {
                name: "".to_string(),
                description: "Test".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_merge_configs() {
        let config1 = SkillConfig {
            skill: SkillMeta {
                name: "skill1".to_string(),
                description: "Description 1".to_string(),
                tags: vec!["tag1".to_string()],
                ..Default::default()
            },
            triggers: vec!["trigger1".to_string()],
            ..Default::default()
        };

        let config2 = SkillConfig {
            skill: SkillMeta {
                name: "skill2".to_string(),
                description: "Description 2".to_string(),
                tags: vec!["tag2".to_string()],
                ..Default::default()
            },
            triggers: vec!["trigger2".to_string()],
            ..Default::default()
        };

        let merged = config1.merge(&config2);

        assert_eq!(merged.skill.name, "skill1");
        assert!(merged.skill.tags.contains(&"tag1".to_string()));
        assert!(merged.skill.tags.contains(&"tag2".to_string()));
        assert!(merged.triggers.contains(&"trigger1".to_string()));
        assert!(merged.triggers.contains(&"trigger2".to_string()));
    }

    #[test]
    fn test_has_tag() {
        let config = SkillConfig {
            skill: SkillMeta {
                name: "test".to_string(),
                description: "Test".to_string(),
                tags: vec!["weather".to_string(), "api".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(config.has_tag("weather"));
        assert!(config.has_tag("api"));
        assert!(!config.has_tag("unknown"));
    }

    #[test]
    fn test_matches_trigger() {
        let config = SkillConfig {
            skill: SkillMeta {
                name: "test".to_string(),
                description: "Test".to_string(),
                ..Default::default()
            },
            triggers: vec!["天气".to_string(), "天气查询".to_string()],
            ..Default::default()
        };

        assert!(config.matches_trigger("北京天气"));
        assert!(config.matches_trigger("天气查询"));
        assert!(!config.matches_trigger("计算"));
    }
}
