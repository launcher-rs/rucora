//! Git 工具模块。
//!
//! 提供 Git 操作功能，支持常见命令和安全检查。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};

/// Git 工具：执行 Git 操作。
///
/// 支持常见的 Git 命令，如 status、log、add、commit 等。
/// 自动检测只读操作和写入操作，提供基本的参数安全检查。
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
/// - 只读：status, log, diff, show, branch, remote
/// - 写入：add, commit, checkout, stash
pub struct GitTool;

impl GitTool {
    /// 创建一个新的 GitTool 实例。
    pub fn new() -> Self {
        Self
    }

    /// 检查命令是否需要写入权限
    fn requires_write_access(&self, command: &str) -> bool {
        matches!(
            command,
            "add" | "commit" | "checkout" | "stash" | "reset" | "revert" | "merge" | "rebase"
        )
    }

    /// 检查命令是否是只读操作
    fn is_read_only(&self, command: &str) -> bool {
        matches!(
            command,
            "status" | "log" | "diff" | "show" | "branch" | "remote" | "config" | "rev-parse"
        )
    }

    /// 清理 git 参数，防止命令注入
    fn sanitize_git_args(&self, args: &str) -> Result<Vec<String>, ToolError> {
        let mut result = Vec::new();
        for arg in args.split_whitespace() {
            let arg_lower = arg.to_lowercase();
            // 阻止危险的 git 选项
            if arg_lower.starts_with("--exec=")
                || arg_lower.starts_with("--upload-pack=")
                || arg_lower.starts_with("--receive-pack=")
                || arg_lower.starts_with("--pager=")
                || arg_lower.starts_with("--editor=")
                || arg_lower == "--no-verify"
                || arg_lower.contains("$(")
                || arg_lower.contains('`')
                || arg.contains('|')
                || arg.contains(';')
                || arg.contains('>')
                || arg_lower == "-c"
                || arg_lower.starts_with("-c=")
            {
                return Err(ToolError::Message(format!(
                    "阻止潜在危险的 git 参数: {}",
                    arg
                )));
            }
            result.push(arg.to_string());
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
        Some("执行 Git 操作，支持 status、log、add、commit 等命令")
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
                    "type": "string",
                    "description": "命令参数，空格分隔"
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

        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let args_str = input.get("args").and_then(|v| v.as_str()).unwrap_or("");

        // 参数安全检查
        let sanitized_args = self.sanitize_git_args(args_str)?;

        // 检查操作类型
        let is_write = self.requires_write_access(command);
        let is_read = self.is_read_only(command);

        if !is_write && !is_read {
            return Err(ToolError::Message(format!(
                "不支持的 Git 命令: {}",
                command
            )));
        }

        // 构建并执行命令
        let output = tokio::process::Command::new("git")
            .arg(command)
            .args(&sanitized_args)
            .current_dir(path)
            .output()
            .await
            .map_err(|e| ToolError::Message(format!("Git 命令执行失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(json!({
            "stdout": stdout,
            "stderr": stderr,
            "success": output.status.success(),
            "is_write_operation": is_write
        }))
    }
}
