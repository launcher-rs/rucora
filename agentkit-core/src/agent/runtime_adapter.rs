//! RuntimeAdapter（运行时适配器）抽象
//!
//! 本模块提供跨平台运行时抽象，使相同的 Agent 代码可以在不同平台运行：
//! - Native（原生操作系统）
//! - Docker（容器环境）
//! - WASM（WebAssembly）
//! - Serverless（无服务器函数）
//!
//! 参考实现: zeroclaw `RuntimeAdapter` trait

use std::path::{Path, PathBuf};
use std::process::Stdio;

/// 运行时平台类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlatform {
    /// 原生操作系统
    Native,
    /// Docker 容器
    Docker,
    /// WebAssembly
    Wasm,
    /// 无服务器函数（如 AWS Lambda, Azure Functions）
    Serverless,
    /// 嵌入式设备
    Embedded,
    /// 浏览器环境
    Browser,
}

impl std::fmt::Display for RuntimePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimePlatform::Native => write!(f, "native"),
            RuntimePlatform::Docker => write!(f, "docker"),
            RuntimePlatform::Wasm => write!(f, "wasm"),
            RuntimePlatform::Serverless => write!(f, "serverless"),
            RuntimePlatform::Embedded => write!(f, "embedded"),
            RuntimePlatform::Browser => write!(f, "browser"),
        }
    }
}

/// 运行时能力声明
///
/// 描述当前运行时支持的功能
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeCapabilities {
    /// 是否有 shell 访问权限
    pub has_shell_access: bool,
    /// 是否有文件系统访问权限
    pub has_filesystem_access: bool,
    /// 是否支持网络访问
    pub has_network_access: bool,
    /// 是否支持长时间运行任务
    pub supports_long_running: bool,
    /// 是否支持多线程
    pub supports_multithreading: bool,
    /// 是否支持动态加载
    pub supports_dynamic_loading: bool,
    /// 最大文件大小限制（字节，0 表示无限制）
    pub max_file_size: u64,
    /// 最大内存限制（字节，0 表示无限制）
    pub max_memory: u64,
}

impl RuntimeCapabilities {
    /// 创建完整能力的运行时
    pub fn full() -> Self {
        Self {
            has_shell_access: true,
            has_filesystem_access: true,
            has_network_access: true,
            supports_long_running: true,
            supports_multithreading: true,
            supports_dynamic_loading: true,
            max_file_size: 0,
            max_memory: 0,
        }
    }

    /// 创建受限能力的运行时（如 WASM）
    pub fn restricted() -> Self {
        Self {
            has_shell_access: false,
            has_filesystem_access: false,
            has_network_access: false,
            supports_long_running: false,
            supports_multithreading: false,
            supports_dynamic_loading: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_memory: 128 * 1024 * 1024,   // 128MB
        }
    }

    /// 创建容器环境的运行时
    pub fn container() -> Self {
        Self {
            has_shell_access: true,
            has_filesystem_access: true,
            has_network_access: true,
            supports_long_running: true,
            supports_multithreading: true,
            supports_dynamic_loading: false, // 通常容器中不鼓励动态加载
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_memory: 512 * 1024 * 1024,   // 512MB
        }
    }
}

/// 运行时适配器 trait
///
/// 抽象不同运行时的平台能力，使 Agent 代码可以跨平台运行。
#[async_trait::async_trait]
pub trait RuntimeAdapter: Send + Sync {
    /// 获取运行时名称
    fn name(&self) -> &str;

    /// 获取运行时平台类型
    fn platform(&self) -> RuntimePlatform;

    /// 获取运行时能力
    fn capabilities(&self) -> RuntimeCapabilities;

    /// 获取存储路径
    ///
    /// 返回运行时允许持久化存储的路径
    fn storage_path(&self) -> PathBuf;

    /// 获取临时目录路径
    fn temp_path(&self) -> PathBuf;

    /// 获取内存预算（字节，0 表示无限制）
    fn memory_budget(&self) -> u64 {
        self.capabilities().max_memory
    }

    /// 检查是否有 shell 访问权限
    fn has_shell_access(&self) -> bool {
        self.capabilities().has_shell_access
    }

    /// 检查是否有文件系统访问权限
    fn has_filesystem_access(&self) -> bool {
        self.capabilities().has_filesystem_access
    }

    /// 检查是否支持网络访问
    fn has_network_access(&self) -> bool {
        self.capabilities().has_network_access
    }

    /// 检查是否支持长时间运行
    fn supports_long_running(&self) -> bool {
        self.capabilities().supports_long_running
    }

    /// 构建 shell 命令
    ///
    /// 根据运行时环境构建适当的命令执行器
    ///
    /// # Errors
    ///
    /// 当运行时不支持 shell 访问时返回 [`RuntimeError`]。
    fn build_shell_command(
        &self,
        command: &str,
        working_dir: Option<&Path>,
    ) -> Result<tokio::process::Command, RuntimeError>;

    /// 读取文件
    ///
    /// 根据运行时限制读取文件内容
    async fn read_file(&self, path: &Path) -> Result<String, RuntimeError>;

    /// 写入文件
    ///
    /// 根据运行时限制写入文件内容
    async fn write_file(&self, path: &Path, content: &str) -> Result<(), RuntimeError>;

    /// 检查文件是否存在
    async fn file_exists(&self, path: &Path) -> bool;

    /// 获取文件大小
    async fn file_size(&self, path: &Path) -> Result<u64, RuntimeError>;

    /// 列出目录内容
    async fn list_directory(&self, path: &Path) -> Result<Vec<tokio::fs::DirEntry>, RuntimeError>;

    /// 创建目录
    async fn create_directory(&self, path: &Path) -> Result<(), RuntimeError>;

    /// 执行 shell 命令
    ///
    /// 在支持 shell 的运行时中执行命令
    async fn execute_shell(
        &self,
        command: &str,
        working_dir: Option<&Path>,
        timeout_secs: Option<u64>,
    ) -> Result<ShellResult, RuntimeError>;

    /// 获取环境变量
    fn get_env(&self, key: &str) -> Option<String>;

    /// 设置环境变量（如果运行时支持）
    ///
    /// # Errors
    ///
    /// 当运行时不支持设置环境变量时返回 [`RuntimeError`]。
    fn set_env(&self, key: &str, value: &str) -> Result<(), RuntimeError>;

    /// 记录日志
    fn log(&self, level: LogLevel, message: &str);

    /// 获取当前时间戳
    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

/// 运行时错误
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// 操作不被支持
    NotSupported(String),
    /// 权限不足
    PermissionDenied(String),
    /// 资源限制
    ResourceLimit(String),
    /// IO 错误
    IoError(String),
    /// 超时
    Timeout(String),
    /// 其他错误
    Other(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::NotSupported(msg) => write!(f, "操作不被支持: {msg}"),
            RuntimeError::PermissionDenied(msg) => write!(f, "权限不足: {msg}"),
            RuntimeError::ResourceLimit(msg) => write!(f, "资源限制: {msg}"),
            RuntimeError::IoError(msg) => write!(f, "IO 错误: {msg}"),
            RuntimeError::Timeout(msg) => write!(f, "超时: {msg}"),
            RuntimeError::Other(msg) => write!(f, "错误: {msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Shell 执行结果
#[derive(Debug, Clone)]
pub struct ShellResult {
    /// 退出码
    pub exit_code: i32,
    /// 标准输出
    pub stdout: String,
    /// 标准错误
    pub stderr: String,
    /// 执行时长（毫秒）
    pub duration_ms: u64,
}

/// 原生运行时适配器
pub struct NativeRuntimeAdapter {
    name: String,
    storage_path: PathBuf,
    capabilities: RuntimeCapabilities,
}

impl NativeRuntimeAdapter {
    /// 创建新的原生运行时适配器
    pub fn new() -> Self {
        // 使用标准目录或当前目录作为存储路径
        let storage_path = PathBuf::from("./.agentkit");

        Self {
            name: "native".to_string(),
            storage_path,
            capabilities: RuntimeCapabilities::full(),
        }
    }

    /// 设置自定义存储路径
    pub fn with_storage_path(mut self, path: impl AsRef<Path>) -> Self {
        self.storage_path = path.as_ref().to_path_buf();
        self
    }

    /// 设置自定义能力
    pub fn with_capabilities(mut self, capabilities: RuntimeCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }
}

impl Default for NativeRuntimeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RuntimeAdapter for NativeRuntimeAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn platform(&self) -> RuntimePlatform {
        RuntimePlatform::Native
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        self.capabilities
    }

    fn storage_path(&self) -> PathBuf {
        self.storage_path.clone()
    }

    fn temp_path(&self) -> PathBuf {
        std::env::temp_dir().join("agentkit")
    }

    fn build_shell_command(
        &self,
        command: &str,
        working_dir: Option<&Path>,
    ) -> Result<tokio::process::Command, RuntimeError> {
        if !self.has_shell_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持 shell 访问".to_string(),
            ));
        }

        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(command);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(command);
            c
        };

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        Ok(cmd)
    }

    async fn read_file(&self, path: &Path) -> Result<String, RuntimeError> {
        if !self.has_filesystem_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持文件系统访问".to_string(),
            ));
        }

        // 检查文件大小限制
        if let Ok(metadata) = tokio::fs::metadata(path).await {
            let size = metadata.len();
            let max_size = self.capabilities.max_file_size;
            if max_size > 0 && size > max_size {
                return Err(RuntimeError::ResourceLimit(format!(
                    "文件大小 {size} 超过限制 {max_size}"
                )));
            }
        }

        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<(), RuntimeError> {
        if !self.has_filesystem_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持文件系统访问".to_string(),
            ));
        }

        // 检查内容大小限制
        let content_size = content.len() as u64;
        let max_size = self.capabilities.max_file_size;
        if max_size > 0 && content_size > max_size {
            return Err(RuntimeError::ResourceLimit(format!(
                "内容大小 {content_size} 超过限制 {max_size}"
            )));
        }

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| RuntimeError::IoError(e.to_string()))?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn file_exists(&self, path: &Path) -> bool {
        if !self.has_filesystem_access() {
            return false;
        }
        tokio::fs::metadata(path).await.is_ok()
    }

    async fn file_size(&self, path: &Path) -> Result<u64, RuntimeError> {
        if !self.has_filesystem_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持文件系统访问".to_string(),
            ));
        }

        tokio::fs::metadata(path)
            .await
            .map(|m| m.len())
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn list_directory(&self, path: &Path) -> Result<Vec<tokio::fs::DirEntry>, RuntimeError> {
        if !self.has_filesystem_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持文件系统访问".to_string(),
            ));
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))?
        {
            entries.push(entry);
        }

        Ok(entries)
    }

    async fn create_directory(&self, path: &Path) -> Result<(), RuntimeError> {
        if !self.has_filesystem_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持文件系统访问".to_string(),
            ));
        }

        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn execute_shell(
        &self,
        command: &str,
        working_dir: Option<&Path>,
        timeout_secs: Option<u64>,
    ) -> Result<ShellResult, RuntimeError> {
        if !self.has_shell_access() {
            return Err(RuntimeError::NotSupported(
                "当前运行时不支持 shell 访问".to_string(),
            ));
        }

        let mut cmd = self.build_shell_command(command, working_dir)?;

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let start = std::time::Instant::now();

        let output = if let Some(timeout) = timeout_secs {
            tokio::time::timeout(tokio::time::Duration::from_secs(timeout), cmd.output())
                .await
                .map_err(|_| RuntimeError::Timeout("命令执行超时".to_string()))?
                .map_err(|e| RuntimeError::IoError(e.to_string()))?
        } else {
            cmd.output()
                .await
                .map_err(|e| RuntimeError::IoError(e.to_string()))?
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ShellResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
        })
    }

    fn get_env(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn set_env(&self, key: &str, value: &str) -> Result<(), RuntimeError> {
        // SAFETY: 我们信任调用者不会传入无效的键或值
        unsafe {
            std::env::set_var(key, value);
        }
        Ok(())
    }

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Trace => tracing::trace!("{}", message),
            LogLevel::Debug => tracing::debug!("{}", message),
            LogLevel::Info => tracing::info!("{}", message),
            LogLevel::Warn => tracing::warn!("{}", message),
            LogLevel::Error => tracing::error!("{}", message),
        }
    }
}

/// 受限运行时适配器（用于 WASM 等受限环境）
pub struct RestrictedRuntimeAdapter {
    name: String,
    storage_path: PathBuf,
}

impl RestrictedRuntimeAdapter {
    /// 创建新的受限运行时适配器
    pub fn new() -> Self {
        Self {
            name: "restricted".to_string(),
            storage_path: PathBuf::from("/tmp/agentkit"),
        }
    }
}

impl Default for RestrictedRuntimeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RuntimeAdapter for RestrictedRuntimeAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn platform(&self) -> RuntimePlatform {
        RuntimePlatform::Wasm
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities::restricted()
    }

    fn storage_path(&self) -> PathBuf {
        self.storage_path.clone()
    }

    fn temp_path(&self) -> PathBuf {
        PathBuf::from("/tmp")
    }

    fn build_shell_command(
        &self,
        _command: &str,
        _working_dir: Option<&Path>,
    ) -> Result<tokio::process::Command, RuntimeError> {
        Err(RuntimeError::NotSupported(
            "受限运行时不支持 shell 命令".to_string(),
        ))
    }

    async fn read_file(&self, path: &Path) -> Result<String, RuntimeError> {
        // 在受限环境中，只允许读取特定路径
        if !path.starts_with(&self.storage_path) {
            return Err(RuntimeError::PermissionDenied(
                "只能访问存储目录内的文件".to_string(),
            ));
        }

        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<(), RuntimeError> {
        if !path.starts_with(&self.storage_path) {
            return Err(RuntimeError::PermissionDenied(
                "只能写入存储目录内的文件".to_string(),
            ));
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn file_exists(&self, path: &Path) -> bool {
        tokio::fs::metadata(path).await.is_ok()
    }

    async fn file_size(&self, path: &Path) -> Result<u64, RuntimeError> {
        tokio::fs::metadata(path)
            .await
            .map(|m| m.len())
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn list_directory(&self, _path: &Path) -> Result<Vec<tokio::fs::DirEntry>, RuntimeError> {
        Err(RuntimeError::NotSupported(
            "受限运行时不支持目录列表".to_string(),
        ))
    }

    async fn create_directory(&self, path: &Path) -> Result<(), RuntimeError> {
        if !path.starts_with(&self.storage_path) {
            return Err(RuntimeError::PermissionDenied(
                "只能在存储目录内创建目录".to_string(),
            ));
        }

        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| RuntimeError::IoError(e.to_string()))
    }

    async fn execute_shell(
        &self,
        _command: &str,
        _working_dir: Option<&Path>,
        _timeout_secs: Option<u64>,
    ) -> Result<ShellResult, RuntimeError> {
        Err(RuntimeError::NotSupported(
            "受限运行时不支持 shell 命令".to_string(),
        ))
    }

    fn get_env(&self, key: &str) -> Option<String> {
        // 受限环境中只允许访问特定环境变量
        if key.starts_with("AGENTKIT_") {
            std::env::var(key).ok()
        } else {
            None
        }
    }

    fn set_env(&self, _key: &str, _value: &str) -> Result<(), RuntimeError> {
        Err(RuntimeError::NotSupported(
            "受限运行时不支持设置环境变量".to_string(),
        ))
    }

    fn log(&self, level: LogLevel, message: &str) {
        // 受限环境中使用 console.log 或类似机制
        eprintln!("[{level:?}] {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_platform_display() {
        assert_eq!(RuntimePlatform::Native.to_string(), "native");
        assert_eq!(RuntimePlatform::Docker.to_string(), "docker");
        assert_eq!(RuntimePlatform::Wasm.to_string(), "wasm");
    }

    #[test]
    fn test_runtime_capabilities() {
        let full = RuntimeCapabilities::full();
        assert!(full.has_shell_access);
        assert!(full.has_filesystem_access);
        assert!(full.supports_long_running);

        let restricted = RuntimeCapabilities::restricted();
        assert!(!restricted.has_shell_access);
        assert!(!restricted.has_filesystem_access);
        assert!(!restricted.supports_long_running);

        let container = RuntimeCapabilities::container();
        assert!(container.has_shell_access);
        assert!(!container.supports_dynamic_loading);
    }

    #[test]
    fn test_runtime_error_display() {
        let err = RuntimeError::NotSupported("test".to_string());
        assert!(err.to_string().contains("不被支持"));

        let err = RuntimeError::PermissionDenied("test".to_string());
        assert!(err.to_string().contains("权限不足"));
    }

    #[tokio::test]
    async fn test_native_runtime_adapter() {
        let adapter = NativeRuntimeAdapter::new();

        assert_eq!(adapter.name(), "native");
        assert!(adapter.has_shell_access());
        assert!(adapter.has_filesystem_access());
        assert!(adapter.supports_long_running());

        // 测试文件操作
        let test_path = adapter.temp_path().join("test.txt");
        adapter.write_file(&test_path, "hello").await.unwrap();

        assert!(adapter.file_exists(&test_path).await);
        assert_eq!(adapter.file_size(&test_path).await.unwrap(), 5);

        let content = adapter.read_file(&test_path).await.unwrap();
        assert_eq!(content, "hello");

        // 清理
        tokio::fs::remove_file(&test_path).await.ok();
    }

    #[tokio::test]
    async fn test_restricted_runtime_adapter() {
        let adapter = RestrictedRuntimeAdapter::new();

        assert_eq!(adapter.name(), "restricted");
        assert!(!adapter.has_shell_access());
        assert!(!adapter.has_filesystem_access());

        // 尝试执行 shell 命令应该失败
        let result = adapter.execute_shell("echo hello", None, None).await;
        assert!(matches!(result, Err(RuntimeError::NotSupported(_))));
    }
}
