//! Glob 文件搜索工具
//!
//! 使用 glob 模式搜索文件，支持通配符和递归搜索

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::Path;

/// Glob 文件搜索工具
///
/// 支持使用 glob 模式搜索文件，例如：
/// - `**/*.rs` - 所有 Rust 文件
/// - `src/**/mod.rs` - src 目录下的所有 mod.rs
/// - `*.txt` - 当前目录下的所有 txt 文件
pub struct GlobSearchTool {
    /// 允许的最大结果数
    max_results: usize,
    /// 允许搜索的根目录（None 表示当前目录）
    allowed_root: Option<std::path::PathBuf>,
}

impl GlobSearchTool {
    /// 创建新的 Glob 搜索工具
    pub fn new() -> Self {
        Self {
            max_results: 1000,
            allowed_root: None,
        }
    }

    /// 设置最大结果数
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// 设置允许搜索的根目录
    pub fn with_allowed_root<P: AsRef<Path>>(mut self, root: P) -> Self {
        self.allowed_root = Some(root.as_ref().to_path_buf());
        self
    }

    /// 验证路径安全性
    fn is_path_allowed(&self, path: &Path) -> bool {
        // 检查路径遍历攻击
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            return false;
        }

        // 如果设置了根目录，检查路径是否在根目录下
        if let Some(ref root) = self.allowed_root {
            let canonical_root = match std::fs::canonicalize(root) {
                Ok(p) => p,
                Err(_) => return false,
            };
            let canonical_path = match std::fs::canonicalize(path) {
                Ok(p) => p,
                Err(_) => return false,
            };
            canonical_path.starts_with(canonical_root)
        } else {
            true
        }
    }
}

impl Default for GlobSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobSearchTool {
    fn name(&self) -> &str {
        "glob_search"
    }

    fn description(&self) -> Option<&str> {
        Some(
            "使用 glob 模式搜索文件。支持通配符: \
             * (匹配任意字符), ** (递归匹配目录), ? (匹配单个字符)。 \
             示例: '**/*.rs' (所有 Rust 文件), 'src/**/mod.rs' (所有 mod.rs)"
        )
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob 搜索模式，例如 '**/*.rs', '*.txt', 'src/**/*.json'"
                },
                "path": {
                    "type": "string",
                    "description": "搜索的起始路径（可选，默认为当前目录）"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少 'pattern' 参数".to_string()))?;

        // 安全检查：禁止绝对路径和路径遍历
        if pattern.starts_with('/') || pattern.starts_with('\\') {
            return Err(ToolError::Message(
                "不允许使用绝对路径，请使用相对路径".to_string()
            ));
        }
        if pattern.contains("..") {
            return Err(ToolError::Message(
                "不允许路径遍历 (..)".to_string()
            ));
        }

        // 构建完整路径
        let search_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .map(Path::new);

        let full_pattern = if let Some(base_path) = search_path {
            if !self.is_path_allowed(base_path) {
                return Err(ToolError::Message(
                    "搜索路径不在允许的范围内".to_string()
                ));
            }
            base_path.join(pattern)
        } else {
            std::path::PathBuf::from(pattern)
        };

        // 执行 glob 搜索
        let mut matches = Vec::new();
        match glob::glob(full_pattern.to_string_lossy().as_ref()) {
            Ok(entries) => {
                for entry in entries.take(self.max_results) {
                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                matches.push(path.to_string_lossy().to_string());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Glob 匹配错误: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(ToolError::Message(format!(
                    "无效的 glob 模式: {}",
                    e
                )));
            }
        }

        // 排序结果
        matches.sort();

        Ok(json!({
            "pattern": pattern,
            "matches": matches,
            "count": matches.len(),
            "truncated": matches.len() >= self.max_results
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_search_creation() {
        let tool = GlobSearchTool::new();
        assert_eq!(tool.name(), "glob_search");
    }

    #[test]
    fn test_path_security() {
        let tool = GlobSearchTool::new();

        // 绝对路径应该被拒绝
        assert!(!tool.is_path_allowed(Path::new("/etc/passwd")));

        // 路径遍历应该被拒绝
        assert!(!tool.is_path_allowed(Path::new("../secret.txt")));
        assert!(!tool.is_path_allowed(Path::new("foo/../../secret.txt")));

        // 正常路径应该被允许
        assert!(tool.is_path_allowed(Path::new("src/main.rs")));
        assert!(tool.is_path_allowed(Path::new("./README.md")));
    }
}
