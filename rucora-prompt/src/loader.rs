//! Prompt 文件加载器

use super::template::PromptTemplate;
use serde::Deserialize;

/// 从文件加载 prompt
///
/// 支持 .toml 格式：
/// ```toml
/// name = "my_prompt"
/// system = "你是助手"
/// template = "问题: {{question}}"
/// ```
pub fn load(path: &str) -> Option<PromptTemplate> {
    // 检查是否为文件路径
    if !is_file_path(path) {
        return None;
    }

    // 读取文件
    let content = std::fs::read_to_string(path).ok()?;

    // 解析 TOML
    let config: TomlPrompt = toml::from_str(&content).ok()?;

    Some(PromptTemplate {
        name: config.name,
        system: config.system,
        template: config.template,
    })
}

/// 从字符串加载
pub fn load_from_str(content: &str) -> Option<PromptTemplate> {
    let config: TomlPrompt = toml::from_str(content).ok()?;
    Some(PromptTemplate {
        name: config.name,
        system: config.system,
        template: config.template,
    })
}

/// 是否为文件路径
fn is_file_path(name: &str) -> bool {
    name.contains('/') || name.contains('\\') || name.ends_with(".toml")
}

/// TOML 配置文件结构
#[derive(Deserialize)]
struct TomlPrompt {
    /// 模板名称
    name: String,
    /// System prompt
    #[serde(rename = "system")]
    system: String,
    /// User prompt 模板
    #[serde(rename = "template")]
    template: String,
}

/// 加载目录所有 .toml 文件
pub fn load_dir(dir: &str) -> std::collections::HashMap<String, PromptTemplate> {
    let mut results = std::collections::HashMap::new();

    let entries = std::fs::read_dir(dir).ok();
    if let Some(entries) = entries {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(tmpl) = load(&path.to_string_lossy()) {
                        results.insert(name.to_string(), tmpl);
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
name = "test"
system = "你是测试助手"
template = "问题: {{question}}"
"#;
        let tmpl = load_from_str(toml_str).unwrap();
        assert_eq!(tmpl.name, "test");
        assert_eq!(tmpl.system, "你是测试助手");
        assert_eq!(tmpl.template, "问题: {{question}}");
    }

    #[test]
    fn test_is_file_path() {
        assert!(is_file_path("config/prompts/test.toml"));
        assert!(is_file_path("test.toml"));
        assert!(!is_file_path("tool"));
        assert!(!is_file_path("search"));
    }
}
