//! Git 工具模块。
//!
//! 提供 Git 操作功能，支持常见命令和安全检查。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// 允许的 Git 命令白名单
const ALLOWED_COMMANDS: &[&str] = &[
    "status", "log", "diff", "show", "branch", "remote", "config", "rev-parse",
    "add", "commit", "checkout", "stash", "reset", "revert", "merge", "rebase",
    "pull", "push", "fetch", "clone", "init", "describe", "tag",
];

/// 写入命令列表
const WRITE_COMMANDS: &[&str] = &[
    "add", "commit", "checkout", "stash", "reset", "revert", "merge", "rebase",
    "pull", "push", "fetch", "clone", "init",
];

/// 禁止的危险参数
const FORBIDDEN_ARGS: &[&str] = &[
    "--exec", "--upload-pack", "--receive-pack", "--pager", "--editor",
    "--no-verify", "--no-gpg-sign", "-c",
];

/// Git 工具：执行 Git 操作。
///
/// 安全限制：
/// - 仅允许白名单中的命令
/// - 禁止危险参数（防止命令注入）
/// - 限制工作目录范围
/// - 自动检测只读/写入操作
///
/// 输入格式：
/// ```json
/// {
///   "command": "status",
///   "args": ["--porcelain"]
/// }
/// ```
///
/// 支持命令：
/// - 只读：status, log, diff, show, branch, remote, config, rev-parse
/// - 写入：add, commit, checkout, stash, reset, revert, merge, rebase, pull, push, fetch, clone, init
pub struct GitTool {
    /// 允许的 Git 仓库根目录（可选，限制操作范围）
    allowed_roots: Option<Vec<PathBuf>>,
    /// 是否允许写入操作
    allow_write: bool,
}

impl GitTool {
    /// 创建一个新的 GitTool 实例。
    pub fn new() -> Self {
        Self {
            allowed_roots: None,
            allow_write: true,
        }
    }

    /// 设置允许的 Git 仓库根目录
    pub fn with_allowed_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.allowed_roots = Some(roots);
        self
    }

    /// 设置是否允许写入操作
    pub fn with_allow_write(mut self, allow: bool) -> Self {
        self.allow_write = allow;
        self
    }

    /// 检查命令是否是写入操作
    fn is_write_command(&self, command: &str) -> bool {
        WRITE_COMMANDS.contains(&command)
    }

    /// 验证 Git 命令是否允许
    fn validate_command(&self, command: &str) -> Result<(), ToolError> {
        let cmd_lower = command.to_lowercase();

        // 检查是否在白名单中
        if !ALLOWED_COMMANDS.contains(&cmd_lower.as_str()) {
            return Err(ToolError::Message(format!(
                "不支持的 Git 命令：{}（允许的命令：{:?}）",
                command, ALLOWED_COMMANDS
            )));
        }

        // 检查是否允许写入操作
        if self.is_write_command(&cmd_lower) && !self.allow_write {
            return Err(ToolError::Message(format!(
                "Git 写入操作已被禁用：{}",
                command
            )));
        }

        Ok(())
    }

    /// 验证路径是否安全
    fn validate_path(&self, path: &str) -> Result<PathBuf, ToolError> {
        let path = Path::new(path);

        // 解析为绝对路径
        let canonical_path = if path.is_absolute() {
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
                .canonicalize()
                .unwrap_or_else(|_| path.to_path_buf())
        };

        // 如果配置了允许的根目录，检查路径是否在其中
        if let Some(allowed_roots) = &self.allowed_roots {
            let is_allowed = allowed_roots.iter().any(|root| {
                canonical_path.starts_with(root)
            });
            if !is_allowed {
                return Err(ToolError::Message(format!(
                    "Git 仓库路径不在允许的范围内（允许的根目录：{:?}）",
                    allowed_roots
                )));
            }
        }

        // 禁止访问系统敏感路径
        let path_str = canonical_path.to_string_lossy().to_lowercase();
        let forbidden_prefixes = [
            "/etc/", "/proc/", "/sys/", "/dev/", "/boot/", "/bin/", "/sbin/",
            "c:\\windows\\", "c:\\program files",
        ];
        for prefix in &forbidden_prefixes {
            if path_str.starts_with(prefix) {
                return Err(ToolError::Message(format!(
                    "禁止在系统敏感路径执行 Git 操作：{}",
                    canonical_path.display()
                )));
            }
        }

        Ok(canonical_path)
    }

    /// 清理 Git 参数，防止命令注入
    fn sanitize_args(&self, args: &[String]) -> Result<Vec<String>, ToolError> {
        let mut result = Vec::with_capacity(args.len());

        for arg in args {
            let arg_lower = arg.to_lowercase();

            // 检查禁止的参数
            for forbidden in FORBIDDEN_ARGS {
                if arg_lower.starts_with(&forbidden.to_lowercase()) {
                    return Err(ToolError::Message(format!(
                        "禁止使用 Git 参数：{}（存在安全风险）",
                        arg
                    )));
                }
            }

            // 检查命令注入特征
            if arg.contains("$(") || arg.contains('`') || arg.contains('|')
                || arg.contains(';') || arg.contains("&&") || arg.contains("||")
                || arg.contains('>') || arg.contains('<') || arg.contains('\n')
                || arg.contains('\r')
            {
                return Err(ToolError::Message(format!(
                    "参数包含危险字符，可能存在注入风险：{}",
                    arg
                )));
            }

            // 检查路径遍历
            if arg.contains("..\\") || arg.contains("../") {
                // 允许 git diff HEAD~2..HEAD 这样的用法，但不允许路径遍历
                if arg.starts_with("..") || arg.ends_with("..") || arg.contains("/../") || arg.contains("\\..\\") {
                    return Err(ToolError::Message(format!(
                        "参数包含路径遍历：{}",
                        arg
                    )));
                }
            }

            result.push(arg.clone());
        }

        Ok(result)
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "git"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("执行 Git 操作（有安全限制：命令白名单、参数检查、路径限制）")
    }

    /// 返回工具分类。
    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::System]
    }

    /// 返回输入参数的 JSON Schema。
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Git 命令，如 status、log、add、commit"
                },
                "args": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "命令参数列表"
                },
                "path": {
                    "type": "string",
                    "description": "Git 仓库路径，默认为当前目录",
                    "default": "."
                }
            },
            "required": ["command"]
        })
    }

    /// 执行 Git 命令。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'command' 字段".to_string()))?;

        // 验证命令
        self.validate_command(command)?;

        let path_str = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let work_dir = self.validate_path(path_str)?;

        // 解析参数
        let args_vec: Vec<String> = input
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // 清理参数
        let sanitized_args = self.sanitize_args(&args_vec)?;

        // 构建并执行命令
        let output = tokio::process::Command::new("git")
            .arg(command)
            .args(&sanitized_args)
            .current_dir(&work_dir)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .env("USERPROFILE", std::env::var("USERPROFILE").unwrap_or_default())
            .output()
            .await
            .map_err(|e| ToolError::Message(format!("Git 命令执行失败：{}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let is_write = self.is_write_command(&command.to_lowercase());

        Ok(json!({
            "command": command,
            "args": sanitized_args,
            "work_dir": work_dir.display().to_string(),
            "stdout": stdout,
            "stderr": stderr,
            "success": output.status.success(),
            "is_write_operation": is_write
        }))
    }
}
