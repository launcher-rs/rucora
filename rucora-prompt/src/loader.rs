//! Prompt 文件加载器

use super::template::PromptTemplate;
use serde::Deserialize;
use std::path::PathBuf;

/// 从文件加载 prompt
///
/// 支持 .toml 格式：
/// ```toml
/// name = "my_prompt"
/// description = "描述"
/// [system]
/// content = "你是助手"
/// [user]
/// template = "问题: {{question}}"
/// ```
pub fn load(path: &str) -> Option<PromptTemplate> {
    // 1. 如果是绝对路径或相对路径，直接加载
    if is_file_path(path) {
        return load_file(path);
    }

    // 2. 尝试作为内置名称，从 prompts 目录加载
    if let Some(p) = load_from_prompts_dir(path) {
        return Some(p);
    }

    // 3. 尝试作为相对路径
    if let Some(p) = load_file(path) {
        return Some(p);
    }

    None
}

/// 从 prompts 目录加载（内置 prompt 文件）
fn load_from_prompts_dir(name: &str) -> Option<PromptTemplate> {
    // prompts/agent/simple.toml -> agent/simple
    let parts: Vec<&str> = name.split('/').collect();

    if parts.len() == 2 {
        let (category, stem) = (parts[0], parts[1]);

        // 构建路径: prompts/{category}/{stem}.toml
        let relative = format!("prompts/{}/{}.toml", category, stem);

        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let full_path = PathBuf::from(manifest_dir).join(&relative);
            if full_path.exists() {
                return load_file(&full_path.to_string_lossy());
            }
        }
    }

    // 尝试直接拼接 prompts 目录
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let base_dir = PathBuf::from(&manifest_dir);

        let full_path = base_dir.join("prompts").join(format!("{}.toml", name));
        if full_path.exists() {
            return load_file(&full_path.to_string_lossy());
        }

        // 尝试按类别搜索
        let prompts_dir = base_dir.join("prompts");
        if prompts_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&prompts_dir) {
                for entry in entries.flatten() {
                    let subdir = entry.path();
                    if subdir.is_dir() {
                        let file_path = subdir.join(format!("{}.toml", name));
                        if file_path.exists() {
                            return load_file(&file_path.to_string_lossy());
                        }
                    }
                }
            }
        }
    }

    None
}

/// 加载文件
fn load_file(path: &str) -> Option<PromptTemplate> {
    let content = std::fs::read_to_string(path).ok()?;
    load_from_str(&content)
}

/// 从字符串加载
pub fn load_from_str(content: &str) -> Option<PromptTemplate> {
    let config: TomlPrompt = toml::from_str(content).ok()?;
    Some(PromptTemplate {
        name: config.name,
        system: config.system.content,
        template: config.user.template,
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
    system: SystemSection,
    /// User prompt 模板
    #[serde(rename = "user")]
    user: UserSection,
}

#[derive(Deserialize)]
struct SystemSection {
    content: String,
}

#[derive(Deserialize)]
struct UserSection {
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
                    if let Some(tmpl) = load_file(&path.to_string_lossy()) {
                        results.insert(name.to_string(), tmpl);
                    }
                }
            }
        }
    }

    results
}

/// 列出所有可用的 prompt 文件
pub fn list_prompt_files() -> Vec<String> {
    let mut files = Vec::new();

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let prompts_dir = PathBuf::from(manifest_dir).join("prompts");

        if prompts_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&prompts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // 直接的 .toml 文件
                    if path.is_file() && path.extension().map_or(false, |e| e == "toml") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            files.push(name.to_string());
                        }
                    }

                    // 子目录中的文件
                    if path.is_dir() {
                        let category = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        if let Ok(subentries) = std::fs::read_dir(&path) {
                            for subentry in subentries.flatten() {
                                let subpath = subentry.path();
                                if subpath.extension().map_or(false, |e| e == "toml") {
                                    let stem =
                                        subpath.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                    files.push(format!("{}/{}", category, stem));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
name = "test"
description = "test desc"

[system]
content = "你是测试助手"

[user]
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
        assert!(is_file_path("path/to/file"));
        assert!(!is_file_path("tool"));
        assert!(!is_file_path("search"));
    }
}
