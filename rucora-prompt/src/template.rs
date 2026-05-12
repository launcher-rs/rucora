//! PromptTemplate 结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板名称
    pub name: String,
    /// System prompt
    pub system: String,
    /// User prompt 模板
    pub template: String,
}

impl PromptTemplate {
    /// 渲染模板，替换 {{variable}}
    pub fn render(&self, vars: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }

    /// 渲染并返回消息数组
    pub fn messages(&self, vars: &HashMap<String, String>) -> Vec<Message> {
        vec![
            Message::system(&self.system),
            Message::user(&self.render(vars)),
        ]
    }

    /// 创建新模板
    pub fn new(name: &str, system: &str, template: &str) -> Self {
        Self {
            name: name.to_string(),
            system: system.to_string(),
            template: template.to_string(),
        }
    }
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: Role::System,
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: Role::User,
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: Role::Assistant,
            content: content.to_string(),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// 渲染错误
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("缺少变量: {0}")]
    MissingVar(String),
}

/// 严格渲染（缺少变量返回错误）
pub fn render_strict(
    template: &str,
    vars: &HashMap<String, String>,
) -> Result<String, RenderError> {
    let mut result = template.to_string();

    // 提取所有变量
    let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
    for cap in re.captures_iter(template) {
        if let Some(var) = cap.get(1) {
            let var_name = var.as_str();
            if !vars.contains_key(var_name) {
                return Err(RenderError::MissingVar(var_name.to_string()));
            }
            result = result.replace(
                &format!("{{{{{}}}}}", var_name),
                vars.get(var_name).unwrap(),
            );
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render() {
        let tmpl = PromptTemplate::new("test", "你是助手", "你好, {{name}}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        assert_eq!(tmpl.render(&vars), "你好, World!");
    }

    #[test]
    fn test_messages() {
        let tmpl = PromptTemplate::new("test", "系统", "问题: {{q}}");
        let mut vars = HashMap::new();
        vars.insert("q".to_string(), "什么是AI?".to_string());

        let msgs = tmpl.messages(&vars);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "系统");
        assert_eq!(msgs[1].content, "问题: 什么是AI?");
    }
}
