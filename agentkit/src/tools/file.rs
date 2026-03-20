//! 文件工具模块。
//!
//! 提供文件读写和编辑功能，带有安全限制。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// 允许的文件扩展名白名单
const ALLOWED_EXTENSIONS: &[&str] = &[
    "txt", "md", "rst", "rs", "py", "js", "ts", "jsx", "tsx", "json", "yaml", "yml", "toml",
    "cfg", "ini", "sh", "bash", "zsh", "html", "css", "scss", "less", "xml", "csv",
];

/// 禁止访问的路径前缀
const FORBIDDEN_PATH_PREFIXES: &[&str] = &[
    "/etc/", "/proc/", "/sys/", "/dev/", "/boot/", "/bin/", "/sbin/", "/usr/bin/", "/usr/sbin/",
    "C:\\Windows\\", "C:\\Program Files\\", "C:\\Program Files (x86)\\",
];

/// 文件读取工具：读取文件内容。
///
/// 安全限制：
/// - 仅允许读取白名单扩展名的文件
/// - 禁止访问系统敏感路径
/// - 支持配置允许的工作目录
///
/// 输入格式：
/// ```json
/// {
///   "path": "/path/to/file"
/// }
/// ```
pub struct FileReadTool {
    /// 允许的工作目录（可选，限制文件访问范围）
    allowed_dirs: Option<Vec<PathBuf>>,
    /// 最大文件大小（字节），默认 1MB
    max_file_size: u64,
}

impl FileReadTool {
    /// 创建一个新的 FileReadTool 实例。
    pub fn new() -> Self {
        Self {
            allowed_dirs: None,
            max_file_size: 1024 * 1024, // 1MB
        }
    }

    /// 设置允许的工作目录
    pub fn with_allowed_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.allowed_dirs = Some(dirs);
        self
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// 验证路径是否安全
    fn validate_path(&self, path: &str) -> Result<PathBuf, ToolError> {
        let path = Path::new(path);

        // 检查是否为绝对路径且包含禁止前缀
        if let Some(path_str) = path.to_str() {
            let path_lower = path_str.to_lowercase();
            for prefix in FORBIDDEN_PATH_PREFIXES {
                if path_lower.starts_with(&prefix.to_lowercase()) {
                    return Err(ToolError::Message(format!(
                        "禁止访问系统敏感路径：{}",
                        path_str
                    )));
                }
            }
        }

        // 检查扩展名
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&ext_lower.as_str()) {
                return Err(ToolError::Message(format!(
                    "不支持的文件类型：{}（允许的类型：{:?}）",
                    ext, ALLOWED_EXTENSIONS
                )));
            }
        } else {
            return Err(ToolError::Message("文件必须包含扩展名".to_string()));
        }

        // 如果配置了允许的目录，检查路径是否在其中
        if let Some(allowed_dirs) = &self.allowed_dirs {
            let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let is_allowed = allowed_dirs.iter().any(|dir| {
                canonical_path.starts_with(dir)
            });
            if !is_allowed {
                return Err(ToolError::Message(format!(
                    "文件路径不在允许的工作目录内（允许的目录：{:?}）",
                    allowed_dirs
                )));
            }
        }

        Ok(path.to_path_buf())
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "file_read"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("读取文件内容（有安全限制：仅允许特定扩展名，禁止系统路径）")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径（相对路径或绝对路径）"
                }
            },
            "required": ["path"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'path' 字段".to_string()))?;

        let path = self.validate_path(path_str)?;

        // 检查文件大小
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| ToolError::Message(format!("无法获取文件信息：{}", e)))?;

        if metadata.len() > self.max_file_size {
            return Err(ToolError::Message(format!(
                "文件过大（{} 字节），超过限制（{} 字节）",
                metadata.len(),
                self.max_file_size
            )));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Message(format!("读取文件失败：{}", e)))?;

        Ok(json!({
            "path": path_str,
            "content": content,
            "size": content.len()
        }))
    }
}

/// 文件写入工具：写入文件内容。
///
/// 安全限制：
/// - 仅允许写入白名单扩展名的文件
/// - 禁止访问系统敏感路径
/// - 支持配置允许的工作目录
///
/// 适用场景：
/// - 写入文件内容
///
/// 输入格式：
/// ```json
/// {
///   "path": "/path/to/file",
///   "content": "文件内容"
/// }
/// ```
pub struct FileWriteTool {
    /// 允许的工作目录（可选，限制文件访问范围）
    allowed_dirs: Option<Vec<PathBuf>>,
    /// 最大文件大小（字节），默认 1MB
    max_file_size: u64,
}

impl FileWriteTool {
    /// 创建一个新的 FileWriteTool 实例。
    pub fn new() -> Self {
        Self {
            allowed_dirs: None,
            max_file_size: 1024 * 1024, // 1MB
        }
    }

    /// 设置允许的工作目录
    pub fn with_allowed_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.allowed_dirs = Some(dirs);
        self
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// 验证路径是否安全
    fn validate_path(&self, path: &str) -> Result<PathBuf, ToolError> {
        let path = Path::new(path);

        // 检查是否为绝对路径且包含禁止前缀
        if let Some(path_str) = path.to_str() {
            let path_lower = path_str.to_lowercase();
            for prefix in FORBIDDEN_PATH_PREFIXES {
                if path_lower.starts_with(&prefix.to_lowercase()) {
                    return Err(ToolError::Message(format!(
                        "禁止访问系统敏感路径：{}",
                        path_str
                    )));
                }
            }
        }

        // 检查扩展名
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&ext_lower.as_str()) {
                return Err(ToolError::Message(format!(
                    "不支持的文件类型：{}（允许的类型：{:?}）",
                    ext, ALLOWED_EXTENSIONS
                )));
            }
        } else {
            return Err(ToolError::Message("文件必须包含扩展名".to_string()));
        }

        // 如果配置了允许的目录，检查路径是否在其中
        if let Some(allowed_dirs) = &self.allowed_dirs {
            let parent = path.parent().unwrap_or(path);
            let canonical_path = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
            let is_allowed = allowed_dirs.iter().any(|dir| {
                canonical_path.starts_with(dir) || dir.starts_with(&canonical_path)
            });
            if !is_allowed {
                return Err(ToolError::Message(format!(
                    "文件路径不在允许的工作目录内（允许的目录：{:?}）",
                    allowed_dirs
                )));
            }
        }

        Ok(path.to_path_buf())
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "file_write"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("写入文件内容（有安全限制：仅允许特定扩展名，禁止系统路径）")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径"
                },
                "content": {
                    "type": "string",
                    "description": "文件内容"
                }
            },
            "required": ["path", "content"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'path' 字段".to_string()))?;

        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'content' 字段".to_string()))?;

        // 检查内容大小
        if content.len() as u64 > self.max_file_size {
            return Err(ToolError::Message(format!(
                "内容过大（{} 字节），超过限制（{} 字节）",
                content.len(),
                self.max_file_size
            )));
        }

        let path = self.validate_path(path_str)?;

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| ToolError::Message(format!("写入文件失败：{}", e)))?;

        Ok(json!({
            "path": path_str,
            "success": true,
            "bytes_written": content.len()
        }))
    }
}

/// 文件编辑工具：通过精确替换编辑文件。
///
/// 使用 old_string → new_string 的精确替换方式来编辑文件内容。
/// old_string 必须在文件中精确匹配出现一次（0 次=未找到，多次=歧义）。
/// new_string 可以为空以删除匹配的文本。
///
/// 安全限制：
/// - 仅允许编辑白名单扩展名的文件
/// - 禁止访问系统敏感路径
/// - 支持配置允许的工作目录
///
/// 输入格式：
/// ```json
/// {
///   "path": "文件路径",
///   "old_string": "要替换的原文",
///   "new_string": "新文本"
/// }
/// ```
pub struct FileEditTool {
    /// 允许的工作目录（可选，限制文件访问范围）
    allowed_dirs: Option<Vec<PathBuf>>,
    /// 最大文件大小（字节），默认 1MB
    max_file_size: u64,
}

impl FileEditTool {
    /// 创建一个新的 FileEditTool 实例。
    pub fn new() -> Self {
        Self {
            allowed_dirs: None,
            max_file_size: 1024 * 1024, // 1MB
        }
    }

    /// 设置允许的工作目录
    pub fn with_allowed_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.allowed_dirs = Some(dirs);
        self
    }

    /// 设置最大文件大小
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// 验证路径是否安全
    fn validate_path(&self, path: &str) -> Result<PathBuf, ToolError> {
        let path = Path::new(path);

        // 检查是否为绝对路径且包含禁止前缀
        if let Some(path_str) = path.to_str() {
            let path_lower = path_str.to_lowercase();
            for prefix in FORBIDDEN_PATH_PREFIXES {
                if path_lower.starts_with(&prefix.to_lowercase()) {
                    return Err(ToolError::Message(format!(
                        "禁止访问系统敏感路径：{}",
                        path_str
                    )));
                }
            }
        }

        // 检查扩展名
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&ext_lower.as_str()) {
                return Err(ToolError::Message(format!(
                    "不支持的文件类型：{}（允许的类型：{:?}）",
                    ext, ALLOWED_EXTENSIONS
                )));
            }
        } else {
            return Err(ToolError::Message("文件必须包含扩展名".to_string()));
        }

        // 如果配置了允许的目录，检查路径是否在其中
        if let Some(allowed_dirs) = &self.allowed_dirs {
            let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let is_allowed = allowed_dirs.iter().any(|dir| {
                canonical_path.starts_with(dir)
            });
            if !is_allowed {
                return Err(ToolError::Message(format!(
                    "文件路径不在允许的工作目录内（允许的目录：{:?}）",
                    allowed_dirs
                )));
            }
        }

        Ok(path.to_path_buf())
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "file_edit"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("通过精确字符串替换编辑文件内容（有安全限制）")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径"
                },
                "old_string": {
                    "type": "string",
                    "description": "要查找并替换的精确文本（必须在文件中精确出现一次）"
                },
                "new_string": {
                    "type": "string",
                    "description": "替换后的文本（空字符串表示删除）"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    /// 执行文件编辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'path' 字段".to_string()))?;

        let old_string = input
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'old_string' 字段".to_string()))?;

        let new_string = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'new_string' 字段".to_string()))?;

        if old_string.is_empty() {
            return Err(ToolError::Message("old_string 不能为空".to_string()));
        }

        let path = self.validate_path(path_str)?;

        // 读取文件内容
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Message(format!("读取文件失败：{}", e)))?;

        // 检查匹配次数
        let matches = content.matches(old_string).count();
        if matches == 0 {
            return Err(ToolError::Message(format!(
                "未找到匹配文本：{}",
                old_string
            )));
        }
        if matches > 1 {
            return Err(ToolError::Message(format!(
                "找到 {} 处匹配，匹配歧义。请提供更精确的唯一匹配文本",
                matches
            )));
        }

        // 执行替换
        let new_content = content.replacen(old_string, new_string, 1);

        // 检查新内容大小
        if new_content.len() as u64 > self.max_file_size {
            return Err(ToolError::Message(format!(
                "编辑后文件过大（{} 字节），超过限制（{} 字节）",
                new_content.len(),
                self.max_file_size
            )));
        }

        // 写回文件
        tokio::fs::write(&path, new_content)
            .await
            .map_err(|e| ToolError::Message(format!("写入文件失败：{}", e)))?;

        Ok(json!({
            "success": true,
            "path": path_str,
            "replacements": 1
        }))
    }
}
