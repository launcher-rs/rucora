//! Shell 工具模块
//!
//! 提供系统命令执行功能，支持超时和安全限制

use async_trait::async_trait;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;

/// 获取 Shell 工具描述
fn get_shell_description() -> &'static str {
    if cfg!(target_os = "windows") {
        "执行系统命令。当前平台：Windows。请使用 Windows 命令：dir (列表), cd (切换目录), type (查看文件), findstr (搜索), copy (复制), move (移动), del (删除), mkdir (创建目录)。命令必须与当前操作系统兼容。"
    } else if cfg!(target_os = "macos") {
        "执行系统命令。当前平台：macOS。请使用 macOS 命令：ls (列表), cd (切换目录), cat (查看文件), grep (搜索), cp (复制), mv (移动), rm (删除), mkdir (创建目录)。命令必须与当前操作系统兼容。"
    } else if cfg!(target_os = "linux") {
        "执行系统命令。当前平台：Linux。请使用 Linux 命令：ls (列表), cd (切换目录), cat (查看文件), grep (搜索), cp (复制), mv (移动), rm (删除), mkdir (创建目录)。命令必须与当前操作系统兼容。"
    } else {
        "执行系统命令。请使用适合当前平台的命令。"
    }
}

/// Shell 命令执行的超时时间（秒）
pub const SHELL_TIMEOUT_SECS: u64 = 60;
/// 最大输出大小（1MB），防止内存溢出
pub const MAX_OUTPUT_BYTES: usize = 1_048_576;

/// 默认禁止的命令列表
const FORBIDDEN_COMMANDS: &[&str] = &[
    "rm -rf",
    "rm -fr",
    "del /f/s/q", // 强制删除
    "format",
    "mkfs",
    "diskpart", // 磁盘操作
    "shutdown",
    "reboot",
    "halt", // 系统操作
    "wget",
    "curl", // 网络下载（可用受限版本替代）
];

/// 默认禁止的危险操作符
const DANGEROUS_OPERATORS: &[&str] = &[
    "|", "||", "&&", ";", ">", ">>", "<", "<<<", // 管道和重定向
    "`", "$(", "${", // 命令替换
    "\n", "\r", // 多行命令
    "\\", // 续行符
];

/// Shell 工具：执行系统命令。
///
/// 支持超时和输出限制，防止命令执行时间过长或输出过大。
///
/// # 安全机制
///
/// - 命令黑名单检查
/// - 危险操作符检测
/// - 路径遍历防护
/// - 安全的环境变量（清除敏感变量）
/// - 超时和输出大小限制
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
///   "timeout": 60,  // 可选，超时时间（秒）
///   "working_dir": "/path/to/dir"  // 可选，工作目录
/// }
/// ```
pub struct ShellTool {
    /// 允许的命令白名单（如果为空，则只检查黑名单）
    allowed_commands: HashSet<String>,
    /// 额外的禁止命令列表
    forbidden_commands: HashSet<String>,
}

impl ShellTool {
    /// 创建一个新的 ShellTool 实例（使用默认安全配置）。
    pub fn new() -> Self {
        Self {
            allowed_commands: HashSet::new(),
            forbidden_commands: HashSet::new(),
        }
    }

    /// 设置允许的命令白名单。
    ///
    /// 如果设置了白名单，只有白名单中的命令可以执行。
    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands.into_iter().collect();
        self
    }

    /// 添加额外的禁止命令。
    pub fn with_forbidden_commands(mut self, commands: Vec<String>) -> Self {
        self.forbidden_commands = commands.into_iter().collect();
        self
    }

    /// 检查命令是否安全
    fn validate_command(&self, command: &str) -> Result<(), ToolError> {
        let cmd_lower = command.to_lowercase();

        // 检查是否在禁止列表中
        for forbidden in FORBIDDEN_COMMANDS {
            if cmd_lower.contains(forbidden) {
                return Err(ToolError::Message(format!(
                    "命令包含禁止的操作：{forbidden}"
                )));
            }
        }

        // 检查额外的禁止命令
        for forbidden in &self.forbidden_commands {
            if cmd_lower.contains(forbidden) {
                return Err(ToolError::Message(format!(
                    "命令包含禁止的操作：{forbidden}"
                )));
            }
        }

        // 检查危险操作符
        for operator in DANGEROUS_OPERATORS {
            if command.contains(operator) {
                return Err(ToolError::Message(format!(
                    "命令包含危险操作符：{operator}"
                )));
            }
        }

        // 如果设置了白名单，检查命令是否在白名单中
        if !self.allowed_commands.is_empty() {
            let cmd_name = command.split_whitespace().next().unwrap_or(command);
            if !self.allowed_commands.contains(cmd_name) {
                return Err(ToolError::Message(format!(
                    "命令 {cmd_name} 不在允许的白名单中"
                )));
            }
        }

        // 检查路径遍历
        if command.contains("..") {
            return Err(ToolError::Message(
                "命令包含路径遍历（..），这是不安全的".to_string(),
            ));
        }

        Ok(())
    }

    /// 检查工作目录是否安全
    fn validate_working_dir(&self, dir: &str) -> Result<(), ToolError> {
        let path = Path::new(dir);

        // 检查路径遍历
        if dir.contains("..") {
            return Err(ToolError::Message(
                "工作目录包含路径遍历（..），这是不安全的".to_string(),
            ));
        }

        // 检查目录是否存在
        if !path.exists() {
            return Err(ToolError::Message(format!("工作目录不存在：{dir}")));
        }

        if !path.is_dir() {
            return Err(ToolError::Message(format!("工作目录路径不是目录：{dir}")));
        }

        Ok(())
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> Option<&str> {
        Some(get_shell_description())
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::System]
    }

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
                    "items": {"type": "string"},
                    "description": "命令参数列表"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时时间（秒），默认 60 秒",
                    "default": 60
                },
                "working_dir": {
                    "type": "string",
                    "description": "工作目录（可选）"
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少必需的 'command' 字段".to_string()))?;

        let args: Vec<String> = input
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(SHELL_TIMEOUT_SECS);

        // 验证命令安全性
        self.validate_command(command)?;

        // 处理工作目录
        let working_dir = input.get("working_dir").and_then(|v| v.as_str());

        if let Some(dir) = working_dir {
            self.validate_working_dir(dir)?;
        }

        // 构建完整命令
        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        // 执行命令
        let result = execute_shell_command(&full_command, timeout_secs, working_dir).await?;

        Ok(json!({
            "command": full_command,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
            "success": result.exit_code == 0,
            "truncated": result.truncated
        }))
    }
}

/// 命令执行结果
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub truncated: bool,
}

/// 执行 shell 命令
pub async fn execute_shell_command(
    command: &str,
    timeout_secs: u64,
    working_dir: Option<&str>,
) -> Result<CommandResult, ToolError> {
    let timeout_duration = Duration::from_secs(timeout_secs);

    // 根据操作系统选择 shell
    let (shell, shell_arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut cmd = tokio::process::Command::new(shell);
    cmd.arg(shell_arg).arg(command);

    // 设置工作目录
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    // 清除敏感环境变量
    cmd.env_remove("AWS_SECRET_ACCESS_KEY");
    cmd.env_remove("AZURE_CLIENT_SECRET");
    cmd.env_remove("GCP_SERVICE_ACCOUNT_KEY");

    // 执行命令（带超时）
    let output = timeout(timeout_duration, cmd.output())
        .await
        .map_err(|_| ToolError::Message(format!("命令执行超时（{timeout_secs} 秒）")))?
        .map_err(|e| ToolError::Message(format!("命令执行失败：{e}")))?;

    let exit_code = output.status.code().unwrap_or(-1);

    // 处理输出（截断过长的输出）
    let (stdout, stdout_truncated) = truncate_output(&output.stdout);
    let (stderr, stderr_truncated) = truncate_output(&output.stderr);

    Ok(CommandResult {
        stdout,
        stderr,
        exit_code,
        truncated: stdout_truncated || stderr_truncated,
    })
}

/// 截断输出
pub fn truncate_output(output: &[u8]) -> (String, bool) {
    if output.len() > MAX_OUTPUT_BYTES {
        let truncated = String::from_utf8_lossy(&output[..MAX_OUTPUT_BYTES]);
        (format!("{truncated}... [截断]"), true)
    } else {
        (String::from_utf8_lossy(output).to_string(), false)
    }
}
