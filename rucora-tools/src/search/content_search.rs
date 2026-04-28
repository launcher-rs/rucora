//! 内容搜索工具
//!
//! 在文件内容中搜索指定文本，支持正则表达式

use async_trait::async_trait;
use regex::Regex;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde_json::{Value, json};
use std::path::Path;

/// 内容搜索工具
///
/// 在文件内容中搜索指定文本或正则表达式模式
pub struct ContentSearchTool {
    /// 允许的最大文件大小（字节）
    max_file_size: u64,
    /// 每个文件最大匹配数
    max_matches_per_file: usize,
    /// 允许的最大结果总数
    max_total_matches: usize,
}

/// 单个匹配结果
#[derive(Debug, Clone, serde::Serialize)]
struct MatchResult {
    /// 文件路径
    file: String,
    /// 行号
    line: usize,
    /// 行内容
    content: String,
    /// 匹配的起始列
    start_col: usize,
    /// 匹配的结束列
    end_col: usize,
}

impl ContentSearchTool {
    /// 创建新的内容搜索工具
    pub fn new() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_matches_per_file: 100,
            max_total_matches: 1000,
        }
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// 在单个文件中搜索
    fn search_in_file(
        &self,
        file_path: &Path,
        pattern: &Regex,
    ) -> Result<Vec<MatchResult>, ToolError> {
        let mut matches = Vec::new();

        // 检查文件大小
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| ToolError::Message(format!("无法读取文件元数据: {e}")))?;

        if metadata.len() > self.max_file_size {
            return Err(ToolError::Message(format!(
                "文件 {} 过大 ({} > {} bytes)",
                file_path.display(),
                metadata.len(),
                self.max_file_size
            )));
        }

        // 读取文件内容
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| ToolError::Message(format!("无法读取文件: {e}")))?;

        // 逐行搜索
        for (line_num, line) in content.lines().enumerate() {
            if matches.len() >= self.max_matches_per_file {
                break;
            }

            for mat in pattern.find_iter(line) {
                matches.push(MatchResult {
                    file: file_path.to_string_lossy().to_string(),
                    line: line_num + 1,
                    content: line.to_string(),
                    start_col: mat.start(),
                    end_col: mat.end(),
                });

                if matches.len() >= self.max_matches_per_file {
                    break;
                }
            }
        }

        Ok(matches)
    }
}

impl Default for ContentSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ContentSearchTool {
    fn name(&self) -> &str {
        "content_search"
    }

    fn description(&self) -> Option<&str> {
        Some(
            "在文件内容中搜索文本或正则表达式模式。 \
             支持在指定目录递归搜索，返回匹配的文件路径、行号和内容。 \
             示例: 搜索所有包含 'TODO' 的 Rust 文件",
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
                    "description": "搜索模式（正则表达式），例如 'TODO|FIXME', 'fn main', 'class\\s+\\w+'"
                },
                "path": {
                    "type": "string",
                    "description": "搜索的起始路径（可选，默认为当前目录）"
                },
                "glob": {
                    "type": "string",
                    "description": "文件过滤模式（可选），例如 '*.rs', '*.py', '*.js'"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "是否区分大小写（默认为 true）",
                    "default": true
                }
            },
            "required": ["pattern"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let pattern_str = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少 'pattern' 参数".to_string()))?;

        let case_sensitive = input
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // 编译正则表达式
        let regex_builder = if case_sensitive {
            Regex::new(pattern_str)
        } else {
            Regex::new(&format!("(?i){pattern_str}"))
        };

        let pattern =
            regex_builder.map_err(|e| ToolError::Message(format!("无效的正则表达式: {e}")))?;

        // 获取搜索路径
        let search_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .map_or_else(|| Path::new("."), Path::new);

        // 获取文件过滤模式
        let glob_pattern = input.get("glob").and_then(|v| v.as_str()).unwrap_or("*");

        // 构建完整的 glob 模式
        let full_glob = if glob_pattern.contains('/') {
            glob_pattern.to_string()
        } else {
            format!("**/{glob_pattern}")
        };

        // 执行搜索
        let mut all_matches = Vec::new();
        let mut files_searched = 0;
        let mut files_with_matches = 0;

        let glob_path = search_path.join(&full_glob);
        match glob::glob(glob_path.to_string_lossy().as_ref()) {
            Ok(entries) => {
                for entry in entries {
                    if all_matches.len() >= self.max_total_matches {
                        break;
                    }

                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                files_searched += 1;

                                match self.search_in_file(&path, &pattern) {
                                    Ok(mut matches) => {
                                        if !matches.is_empty() {
                                            files_with_matches += 1;
                                            all_matches.append(&mut matches);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("搜索文件 {} 时出错: {}", path.display(), e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Glob 匹配错误: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(ToolError::Message(format!("无效的 glob 模式: {e}")));
            }
        }

        // 限制结果数量
        let truncated = all_matches.len() > self.max_total_matches;
        all_matches.truncate(self.max_total_matches);

        Ok(json!({
            "pattern": pattern_str,
            "glob": glob_pattern,
            "files_searched": files_searched,
            "files_with_matches": files_with_matches,
            "total_matches": all_matches.len(),
            "truncated": truncated,
            "matches": all_matches
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_search_creation() {
        let tool = ContentSearchTool::new();
        assert_eq!(tool.name(), "content_search");
    }

    #[test]
    fn test_regex_compilation() {
        // 有效的正则表达式
        let regex = Regex::new(r"TODO|FIXME");
        assert!(regex.is_ok());

        // 无效的正则表达式
        let regex = Regex::new(r"\[invalid");
        assert!(regex.is_err());
    }
}
