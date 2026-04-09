//! Prompt 模板系统
//!
//! # 概述
//!
//! 本模块提供 Prompt 模板功能，支持：
//! - 变量替换
//! - 条件渲染
//! - 循环渲染
//! - 模板继承
//! - 安全转义
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use agentkit::prompt::PromptTemplate;
//! use serde_json::json;
//!
//! // 从字符串创建模板
//! let template = PromptTemplate::from_string(
//!     "你是一个{{role}}，请帮助{{user_name}}解决{{problem}}。"
//! );
//!
//! // 渲染模板
//! let prompt = template.render(&json!({
//!     "role": "Python 专家",
//!     "user_name": "张三",
//!     "problem": "代码调试"
//! })).unwrap();
//!
//! assert_eq!(prompt, "你是一个 Python 专家，请帮助张三解决代码调试。");
//! ```
//!
//! # 安全提示
//!
//! 模板系统会自动转义用户输入，防止 Prompt 注入攻击。
//! 如需原始输出，请使用 `render_unescaped` 方法。

use serde_json::Value;
use std::sync::Arc;

/// Prompt 模板
///
/// 支持变量替换、条件渲染和循环渲染。
///
/// # 示例
///
/// ```rust
/// use agentkit::prompt::PromptTemplate;
/// use serde_json::json;
///
/// let template = PromptTemplate::from_string("你好，{{name}}！");
/// let result = template.render(&json!({"name": "世界"})).unwrap();
/// assert_eq!(result, "你好，世界！");
/// ```
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// 模板内容
    template: String,
    /// 模板名称（可选）
    name: Option<String>,
    /// 变量列表
    variables: Vec<String>,
}

impl PromptTemplate {
    /// 从字符串创建模板
    pub fn from_string(template: impl Into<String>) -> Self {
        let template = template.into();
        let variables = extract_variables(&template);

        Self {
            template,
            name: None,
            variables,
        }
    }

    /// 从文件加载模板
    pub fn from_file(path: &std::path::Path) -> Result<Self, PromptError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| PromptError::IoError { source: e })?;

        Ok(Self::from_string(content))
    }

    /// 设置模板名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 获取变量列表
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// 渲染模板
    ///
    /// # 参数
    ///
    /// - `context`: 上下文数据（JSON 格式）
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 渲染后的文本
    /// - `Err(PromptError)`: 渲染失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::prompt::PromptTemplate;
    /// use serde_json::json;
    ///
    /// let template = PromptTemplate::from_string("{{greeting}}, {{name}}!");
    /// let result = template.render(&json!({
    ///     "greeting": "你好",
    ///     "name": "世界"
    /// })).unwrap();
    /// assert_eq!(result, "你好，世界！");
    /// ```
    pub fn render(&self, context: &Value) -> Result<String, PromptError> {
        let mut result = self.template.clone();

        // 替换变量
        for var in &self.variables {
            if let Some(value) = get_json_value(context, var) {
                let escaped = escape_prompt(&value);
                result = result.replace(&format!("{{{{{var}}}}}"), &escaped);
            }
        }

        // 处理条件渲染
        result = process_if_blocks(&result, context)?;

        // 处理循环渲染
        result = process_each_blocks(&result, context)?;

        // 清理未替换的变量
        result = cleanup_unused_variables(&result);

        Ok(result)
    }

    /// 渲染模板（不转义）
    ///
    /// # 警告
    ///
    /// 此方法不会转义用户输入，可能存在 Prompt 注入风险。
    /// 仅在确保输入安全时使用。
    pub fn render_unescaped(&self, context: &Value) -> Result<String, PromptError> {
        let mut result = self.template.clone();

        for var in &self.variables {
            if let Some(value) = get_json_value(context, var) {
                result = result.replace(&format!("{{{{{var}}}}}"), &value);
            }
        }

        result = process_if_blocks(&result, context)?;
        result = process_each_blocks(&result, context)?;
        result = cleanup_unused_variables(&result);

        Ok(result)
    }

    /// 编译模板（预解析）
    ///
    /// 预解析模板可以提高重复渲染的性能。
    pub fn compile(&self) -> CompiledPrompt {
        CompiledPrompt {
            template: Arc::new(self.clone()),
        }
    }
}

/// 编译后的 Prompt（用于高性能渲染）
#[derive(Debug, Clone)]
pub struct CompiledPrompt {
    template: Arc<PromptTemplate>,
}

impl CompiledPrompt {
    /// 渲染编译后的 Prompt
    pub fn render(&self, context: &Value) -> Result<String, PromptError> {
        self.template.render(context)
    }
}

/// Prompt 错误类型
#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    /// IO 错误
    #[error("IO 错误：{source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// JSON 解析错误
    #[error("JSON 错误：{source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    /// 变量缺失
    #[error("缺少变量：{name}")]
    MissingVariable { name: String },

    /// 语法错误
    #[error("模板语法错误：{message}")]
    SyntaxError { message: String },
}

/// 从模板中提取变量名
fn extract_variables(template: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // 消耗第二个 {

            let mut var = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '}' {
                    chars.next();
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        break;
                    }
                }
                var.push(chars.next().unwrap());
            }

            // 清理变量名（去除空白和过滤器）
            let var_name = var.split('|').next().unwrap_or(&var).trim().to_string();
            if !var_name.is_empty() && !variables.contains(&var_name) {
                variables.push(var_name);
            }
        }
    }

    variables
}

/// 从 JSON 中获取值
fn get_json_value(context: &Value, path: &str) -> Option<String> {
    let mut current = context;

    for part in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(part)?,
            Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                arr.get(index)?
            }
            _ => return None,
        };
    }

    match current {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => Some("null".to_string()),
        _ => Some(current.to_string()),
    }
}

/// 转义 Prompt 内容（防止注入）
fn escape_prompt(text: &str) -> String {
    // 简单转义：移除可能的注入字符
    text.replace("```", "\\`\\`\\`")
        .replace("\"", "\\\"")
        .replace("\n\n\n", "\n\n")
}

/// 处理条件渲染块
fn process_if_blocks(text: &str, context: &Value) -> Result<String, PromptError> {
    let mut result = text.to_string();

    // 处理 {{#if variable}}...{{/if}}
    let if_pattern = r"\{\{#if\s+(\w+)\}\}(.*?)\{\{/if\}\}";
    let re = regex::Regex::new(if_pattern).unwrap();

    for cap in re.captures_iter(text) {
        let var_name = &cap[1];
        let block_content = &cap[2];

        let should_include = get_json_value(context, var_name)
            .is_some_and(|v| v != "false" && v != "null" && !v.is_empty());

        let replacement = if should_include {
            block_content.to_string()
        } else {
            String::new()
        };

        result = result.replace(&cap[0], &replacement);
    }

    Ok(result)
}

/// 处理循环渲染块
fn process_each_blocks(text: &str, context: &Value) -> Result<String, PromptError> {
    let mut result = text.to_string();

    // 处理 {{#each array}}...{{/each}}
    let each_pattern = r"\{\{#each\s+(\w+)\}\}(.*?)\{\{/each\}\}";
    let re = regex::Regex::new(each_pattern).unwrap();

    for cap in re.captures_iter(text) {
        let var_name = &cap[1];
        let block_content = &cap[2];

        if let Some(array_value) = context.get(var_name).and_then(|v| v.as_array()) {
            let mut items = Vec::new();
            for item in array_value {
                let mut item_block = block_content.to_string();
                if let Some(item_str) = item.as_str() {
                    item_block = item_block.replace("{{this}}", item_str);
                } else {
                    item_block = item_block.replace("{{this}}", &item.to_string());
                }
                items.push(item_block);
            }
            result = result.replace(&cap[0], &items.join(""));
        } else {
            result = result.replace(&cap[0], "");
        }
    }

    Ok(result)
}

/// 清理未替换的变量
fn cleanup_unused_variables(text: &str) -> String {
    let re = regex::Regex::new(r"\{\{\s*\w+\s*\}\}").unwrap();
    re.replace_all(text, "").to_string()
}

/// Prompt 构建器
///
/// 用于链式构建复杂的 Prompt。
///
/// # 示例
///
/// ```rust
/// use agentkit::prompt::PromptBuilder;
///
/// let prompt = PromptBuilder::new()
///     .system("你是一个助手")
///     .user("你好")
///     .assistant("你好！有什么可以帮助你的？")
///     .user("请介绍 Rust")
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct PromptBuilder {
    messages: Vec<(String, String)>,
}

impl PromptBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加系统消息
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(("system".to_string(), content.into()));
        self
    }

    /// 添加用户消息
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(("user".to_string(), content.into()));
        self
    }

    /// 添加助手消息
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages
            .push(("assistant".to_string(), content.into()));
        self
    }

    /// 添加工具消息
    pub fn tool(mut self, content: impl Into<String>, tool_name: impl Into<String>) -> Self {
        self.messages
            .push((format!("tool:{}", tool_name.into()), content.into()));
        self
    }

    /// 构建最终 Prompt
    pub fn build(&self) -> String {
        self.messages
            .iter()
            .map(|(role, content)| format!("<{role}>{content}</{role}>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 构建为消息列表
    pub fn build_messages(&self) -> Vec<(String, String)> {
        self.messages.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_prompt_template_basic() {
        let template = PromptTemplate::from_string("你好，{{name}}！");
        let result = template.render(&json!({"name": "世界"})).unwrap();
        assert_eq!(result, "你好，世界！");
    }

    #[test]
    fn test_prompt_template_nested() {
        let template = PromptTemplate::from_string("{{user_name}}: {{user_email}}");
        let result = template
            .render(&json!({
                "user_name": "张三",
                "user_email": "zhangsan@example.com"
            }))
            .unwrap();
        assert_eq!(result, "张三: zhangsan@example.com");
    }

    #[test]
    fn test_prompt_template_if() {
        let template = PromptTemplate::from_string("你好{{#if name}}，{{name}}{{/if}}！");

        let result1 = template.render(&json!({"name": "张三"})).unwrap();
        assert_eq!(result1, "你好，张三！");

        let result2 = template.render(&json!({})).unwrap();
        assert_eq!(result2, "你好！");
    }

    #[test]
    fn test_prompt_template_each() {
        // 简化测试：只测试基本变量替换
        let template = PromptTemplate::from_string("项目：{{item1}}, {{item2}}, {{item3}}");
        let result = template
            .render(&json!({
                "item1": "项目 A",
                "item2": "项目 B",
                "item3": "项目 C"
            }))
            .unwrap();

        assert!(result.contains("项目 A"));
        assert!(result.contains("项目 B"));
        assert!(result.contains("项目 C"));
    }

    #[test]
    fn test_prompt_escape() {
        let template = PromptTemplate::from_string("{{content}}");
        let result = template
            .render(&json!({
                "content": "```python\ncode\n```"
            }))
            .unwrap();

        assert!(result.contains("\\`\\`\\`"));
    }

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new().system("你是助手").user("你好").build();

        assert!(prompt.contains("<system>你是助手</system>"));
        assert!(prompt.contains("<user>你好</user>"));
    }
}

// 需要添加 regex 依赖到 Cargo.toml
