//! Skill（技能）核心定义模块

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Skill 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub input_schema: Value,
    #[serde(default)]
    pub output_schema: Value,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl SkillDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: default_version(),
            author: None,
            tags: Vec::new(),
            timeout: default_timeout(),
            input_schema: Value::Null,
            output_schema: Value::Null,
            homepage: None,
            metadata: None,
        }
    }
    
    pub fn to_tool_description(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.input_schema
            }
        })
    }
    
    pub fn validate_input(&self, input: &Value) -> Result<(), String> {
        if self.input_schema.is_null() {
            return Ok(());
        }
        
        if let Some(_props) = self.input_schema.get("properties") {
            if let Some(required) = self.input_schema.get("required").and_then(|v| v.as_array()) {
                for req_field in required {
                    if let Some(field_name) = req_field.as_str() {
                        if input.get(field_name).is_none() {
                            return Err(format!("缺少必需字段：{}", field_name));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Skill 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub execution_time_ms: Option<u64>,
}

impl SkillResult {
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            execution_time_ms: None,
        }
    }
    
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "success": false,
                "error": "序列化失败"
            })
        })
    }
}

/// Skill 执行上下文
#[derive(Debug, Clone)]
pub struct SkillContext {
    pub input: Value,
    pub definition: SkillDefinition,
    pub env: std::collections::HashMap<String, String>,
    pub work_dir: Option<std::path::PathBuf>,
}

impl SkillContext {
    pub fn new(input: Value, definition: SkillDefinition) -> Self {
        Self {
            input,
            definition,
            env: std::env::vars().collect(),
            work_dir: None,
        }
    }
    
    pub fn with_work_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.work_dir = Some(dir.into());
        self
    }
}
