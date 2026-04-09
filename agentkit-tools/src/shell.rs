//! Shell 工具模块。
//!
//! 提供系统命令执行功能，支持超时和安全限制。
//!
//! # 安全特性
//!
//! - 命令白名单/黑名单机制
//! - 危险操作符检测
//! - 路径遍历防护
//! - 资源限制（超时、输出大小）
//! - 安全的环境变量

use agentkit_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use async_trait::async_trait;
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
            forbidden_commands: FORBIDDEN_COMMANDS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// 设置允许的命令白名单。
    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands.into_iter().collect();
        self
    }

    /// 添加额外的禁止命令。
    pub fn with_forbidden_commands(mut self, commands: Vec<String>) -> Self {
        self.forbidden_commands.extend(commands);
        self
    }

    /// 验证命令安全性。
    fn validate_command(&self, command: &str, args: &[String]) -> Result<(), ToolError> {
        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        // 检查白名单（如果设置了）
        if !self.allowed_commands.is_empty() {
            let cmd_name = command.split_whitespace().next().unwrap_or(command);
            if !self.allowed_commands.contains(cmd_name) {
                return Err(ToolError::PolicyDenied {
                    rule_id: "shell.whitelist".to_string(),
                    reason: format!("命令 '{}' 不在白名单中", cmd_name),
                });
            }
        }

        // 检查黑名单
        for forbidden in &self.forbidden_commands {
            if full_command.contains(forbidden) {
                return Err(ToolError::PolicyDenied {
                    rule_id: "shell.blacklist".to_string(),
                    reason: format!("命令包含禁止的操作: {}", forbidden),
                });
            }
        }

        // 检查危险操作符
        for op in DANGEROUS_OPERATORS {
            if command.contains(op) || args.iter().any(|a| a.contains(op)) {
                return Err(ToolError::PolicyDenied {
                    rule_id: "shell.dangerous_operators".to_string(),
                    reason: format!("命令包含危险操作符: {}", op),
                });
            }
        }

        // 检查路径遍历攻击
        if command.contains("..") || args.iter().any(|a| a.contains("..")) {
            return Err(ToolError::PolicyDenied {
                rule_id: "shell.path_traversal".to_string(),
                reason: "命令可能包含路径遍历攻击".to_string(),
            });
        }

        // 检查环境变量泄露风险
        let env_patterns = ["PASSWORD", "SECRET", "TOKEN", "API_KEY", "CREDENTIAL"];
        for pattern in &env_patterns {
            if command.contains(pattern) || args.iter().any(|a| a.contains(pattern)) {
                return Err(ToolError::PolicyDenied {
                    rule_id: "shell.env_leak".to_string(),
                    reason: "命令可能泄露敏感环境变量".to_string(),
                });
            }
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
    /// 返回工具名称。
    fn name(&self) -> &str {
        "shell"
    }

    /// 返回工具描述。
    fn description(&self) -> Option<&str> {
        Some(get_shell_description())
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
                    "description": "要执行的命令（注意：必须与当前操作系统兼容。Windows 使用 dir/findstr/type 等命令，Linux/Mac 使用 ls/grep/cat 等命令）"
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
                    "description": "超时时间（秒），默认 60 秒"
                },
                "working_dir": {
                    "type": "string",
                    "description": "工作目录（可选）"
                }
            },
            "required": ["command"]
        })
    }

    /// 执行工具的核心逻辑。
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
                    .filter_map(|x| x.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // 安全验证
        self.validate_command(command, &args)?;

        let timeout_secs = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(SHELL_TIMEOUT_SECS);

        let working_dir = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 验证工作目录
        if let Some(ref dir) = working_dir {
            let path = Path::new(dir);
            if !path.exists() || !path.is_dir() {
                return Err(ToolError::Message(format!(
                    "工作目录不存在或不是目录: {}",
                    dir
                )));
            }
            // 检查路径遍历
            if dir.contains("..") {
                return Err(ToolError::PolicyDenied {
                    rule_id: "shell.path_traversal".to_string(),
                    reason: "工作目录可能包含路径遍历".to_string(),
                });
            }
        }

        // 执行命令并设置超时
        let result = timeout(
            Duration::from_secs(timeout_secs),
            execute_shell_command(command, &args, working_dir.as_deref()),
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
///
/// # 参数
///
/// - `command`: 要执行的命令
/// - `args`: 命令的参数列表
/// - `working_dir`: 工作目录（可选）
///
/// # 安全性
///
/// - 清除所有环境变量，只保留安全的基础变量
/// - 使用 spawn_blocking 避免阻塞 async 运行时
pub async fn execute_shell_command(
    command: &str,
    args: &[String],
    working_dir: Option<&str>,
) -> Result<std::process::Output, std::io::Error> {
    let mut cmd = if cfg!(target_os = "windows") {
        // Windows 上使用 cmd /c 执行命令
        // 注意：args 会被附加到命令后面，作为 $0, $1 等位置参数
        let mut c = std::process::Command::new("cmd");
        c.arg("/C");

        // 在 Windows 上，需要正确构建完整命令
        if args.is_empty() {
            c.arg(command);
        } else {
            // 构建完整命令字符串
            let full_cmd = format!("{} {}", command, args.join(" "));
            c.arg(full_cmd);
        }
        c
    } else if cfg!(any(target_os = "linux", target_os = "macos")) {
        // Linux/macOS 使用 sh -c 执行命令
        // sh -c "command" [args...] - args 会作为 $0, $1 等传递
        let mut c = std::process::Command::new("sh");
        c.arg("-c");

        if args.is_empty() {
            c.arg(command);
        } else {
            // 构建完整命令字符串
            let full_cmd = format!("{} {}", command, args.join(" "));
            c.arg(full_cmd);
        }
        c
    } else {
        // 其他系统直接执行命令
        let mut c = std::process::Command::new(command);
        if !args.is_empty() {
            c.args(args);
        }
        c
    };

    // 设置工作目录
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    // 只保留安全的环境变量，防止敏感信息泄露
    cmd.env_clear();
    let safe_env_vars = [
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "TMPDIR",
        "TEMP",
        "TMP",
        "SystemRoot",
        "USERPROFILE",
    ];
    for var in &safe_env_vars {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, val);
        }
    }

    tokio::task::spawn_blocking(move || cmd.output())
        .await
        .map_err(|e| std::io::Error::other(format!("任务执行失败: {}", e)))?
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
