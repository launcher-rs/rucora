//! 文件工具共享配置
//!
//! 用于提取 FileReadTool、FileWriteTool 和 FileEditTool 的公共逻辑

use agentkit_core::error::ToolError;
use std::path::{Path, PathBuf};

/// 允许的文件扩展名白名单
const ALLOWED_EXTENSIONS: &[&str] = &[
    "txt", "md", "rst", "rs", "py", "js", "ts", "jsx", "tsx", "json", "yaml", "yml", "toml", "cfg",
    "ini", "sh", "bash", "zsh", "html", "css", "scss", "less", "xml", "csv",
];

/// 禁止访问的路径前缀
const FORBIDDEN_PATH_PREFIXES: &[&str] = &[
    "/etc/",
    "/proc/",
    "/sys/",
    "/dev/",
    "/boot/",
    "/bin/",
    "/sbin/",
    "/usr/bin/",
    "/usr/sbin/",
    "C:\\Windows\\",
    "C:\\Program Files\\",
    "C:\\Program Files (x86)\\",
];

/// 文件工具的共享配置
#[derive(Clone)]
pub struct FileToolConfig {
    /// 允许的工作目录（可选，限制文件访问范围）
    pub allowed_dirs: Option<Vec<PathBuf>>,
    /// 最大文件大小（字节）
    pub max_file_size: u64,
}

impl FileToolConfig {
    /// 创建默认配置
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

    /// 验证路径是否安全（用于读取）
    pub fn validate_path_for_read(&self, path: &str) -> Result<PathBuf, ToolError> {
        self.validate_path(path, false)
    }

    /// 验证路径是否安全（用于写入）
    pub fn validate_path_for_write(&self, path: &str) -> Result<PathBuf, ToolError> {
        self.validate_path(path, true)
    }

    /// 验证路径的共享逻辑
    fn validate_path(&self, path: &str, is_write: bool) -> Result<PathBuf, ToolError> {
        let path = Path::new(path);

        // 检查是否为绝对路径且包含禁止前缀
        if let Some(path_str) = path.to_str() {
            let path_lower = path_str.to_lowercase();
            for prefix in FORBIDDEN_PATH_PREFIXES {
                if path_lower.starts_with(&prefix.to_lowercase()) {
                    return Err(ToolError::Message(format!(
                        "禁止访问系统敏感路径：{path_str}"
                    )));
                }
            }
        }

        // 检查扩展名
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&ext_lower.as_str()) {
                return Err(ToolError::Message(format!(
                    "不支持的文件类型：{ext}（允许的类型：{ALLOWED_EXTENSIONS:?}）"
                )));
            }
        } else {
            return Err(ToolError::Message("文件必须包含扩展名".to_string()));
        }

        // 如果配置了允许的目录，检查路径是否在其中
        if let Some(allowed_dirs) = &self.allowed_dirs {
            if is_write {
                // 写入时检查父目录
                let parent = path.parent().unwrap_or(path);
                let canonical_path = parent
                    .canonicalize()
                    .unwrap_or_else(|_| parent.to_path_buf());
                let is_allowed = allowed_dirs
                    .iter()
                    .any(|dir| canonical_path.starts_with(dir) || dir.starts_with(&canonical_path));
                if !is_allowed {
                    return Err(ToolError::Message(format!(
                        "文件路径不在允许的工作目录内（允许的目录：{allowed_dirs:?}）"
                    )));
                }
            } else {
                // 读取时检查文件本身
                let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                let is_allowed = allowed_dirs
                    .iter()
                    .any(|dir| canonical_path.starts_with(dir));
                if !is_allowed {
                    return Err(ToolError::Message(format!(
                        "文件路径不在允许的工作目录内（允许的目录：{allowed_dirs:?}）"
                    )));
                }
            }
        }

        Ok(path.to_path_buf())
    }

    /// 检查文件大小
    pub fn check_file_size(&self, size: u64, operation: &str) -> Result<(), ToolError> {
        if size > self.max_file_size {
            return Err(ToolError::Message(format!(
                "{}过大（{} 字节），超过限制（{} 字节）",
                operation, size, self.max_file_size
            )));
        }
        Ok(())
    }
}

impl Default for FileToolConfig {
    fn default() -> Self {
        Self::new()
    }
}
