//! Shell 工具模块。
//!
//! 提供系统命令执行功能，支持超时和安全限制。

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::timeout;

/// Shell 命令执行的超时时间（秒）
pub const SHELL_TIMEOUT_SECS: u64 = 60;
/// 最大输出大小（1MB），防止内存溢出
pub const MAX_OUTPUT_BYTES: usize = 1_048_576;

/// Shell 工具：执行系统命令。
///
/// 支持超时和输出限制，防止命令执行时间过长或输出过大。
///
/// 适用场景：
/// - 执行系统命令
/// - 运行脚本
///
/// 输入格式：
/// ```json
/// {
///   "command": "要执行的命令",
///   "args": ["命令参数"],
///   "timeout": 60 // 可选，超时时间（秒）
/// }
/// ```
pub struct ShellTool;

impl ShellTool {
    /// 创建一个新的 ShellTool 实例。
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellTool {
    /// 返回工具名称。
    fn name(&self) -> &str {
        "shell"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some("执行系统命令")
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
                    "description": "要执行的命令"
                },
                "args": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "命令参数"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时时间（秒）"
                }
            },
            "required": ["command", "args"]
        })
    }

    /// 执行工具的核心逻辑。
    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'command' 字段".to_string()))?;

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(SHELL_TIMEOUT_SECS);

        // 执行命令并设置超时
        let result = timeout(
            Duration::from_secs(timeout_secs),
            execute_shell_command(command),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = truncate_output(String::from_utf8_lossy(&output.stdout).to_string());
                let stderr = truncate_output(String::from_utf8_lossy(&output.stderr).to_string());

                Ok(json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.status.code().unwrap_or(-1),
                    "success": output.status.success()
                }))
            }
            Ok(Err(e)) => Err(ToolError::Message(format!("命令执行失败: {}", e))),
            Err(_) => Err(ToolError::Message(format!(
                "命令执行超时（超过 {} 秒）",
                timeout_secs
            ))),
        }
    }
}

/// 执行 shell 命令（内部函数）
pub async fn execute_shell_command(command: &str) -> Result<std::process::Output, std::io::Error> {
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("cmd");
    #[cfg(target_os = "windows")]
    cmd.args(["/C", command]);

    #[cfg(not(target_os = "windows"))]
    let mut cmd = std::process::Command::new("sh");
    #[cfg(not(target_os = "windows"))]
    cmd.args(["-c", command]);

    // 只保留安全的环境变量，防止敏感信息泄露
    cmd.env_clear();
    let safe_env_vars = [
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "TMPDIR",
        "TEMP",
        "SystemRoot",
    ];
    for var in &safe_env_vars {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, val);
        }
    }

    tokio::task::spawn_blocking(move || cmd.output())
        .await
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("任务执行失败: {}", e))
        })?
}

/// 截断输出内容，防止内存溢出
pub fn truncate_output(mut output: String) -> String {
    if output.len() > MAX_OUTPUT_BYTES {
        // 找到有效的 UTF-8 边界
        let mut boundary = MAX_OUTPUT_BYTES;
        while boundary > 0 && !output.is_char_boundary(boundary) {
            boundary -= 1;
        }
        output.truncate(boundary);
        output.push_str("\n... [输出已截断，超过 1MB 限制]");
    }
    output
}
