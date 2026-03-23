//! 统一工具加载器模块
//!
//! # 概述
//!
//! `ToolLoader` 提供统一的接口来加载和管理来自不同来源的工具。
//! 它简化了工具注册流程，支持链式调用和条件过滤。
//!
//! # 支持的工具来源
//!
//! - **Mcp**: 从 MCP（Model Context Protocol）服务器加载的工具
//! - **A2A**: 从 A2A（Agent-to-Agent）协议加载的工具
//! - **Custom**: 用户自定义工具（通过 `register()` 方法手动注册）
//!
//! # 关于内置工具和 Skills
//!
//! `agentkit-runtime` 本身不提供内置工具或 Skills 加载功能。
//! 如果需要使用内置工具（如 shell、file、http 等）或 Skills，
//! 请使用 `agentkit` crate 中的工具加载器，或手动注册实现 `Tool` trait 的类型。
//!
//! # 使用示例
//!
//! ## 基本使用
//!
//! ```rust,no_run
//! use agentkit::runtime::loader::ToolLoader;
//! use agentkit::runtime::ToolWrapper;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 手动注册自定义工具
//! let loader = ToolLoader::new()
//!     .register(MyCustomTool::new());
//!
//! let registry = loader.build();
//! # Ok(())
//! # }
//! ```
//!
//! ## 使用过滤器
//!
//! ```rust,no_run
//! use agentkit::runtime::loader::ToolLoader;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 只加载特定工具
//! let loader = ToolLoader::new()
//!     .include("shell")      // 包含 shell
//!     .include("file")       // 包含 file
//!     .exclude("dangerous")  // 排除 dangerous
//!     .register(MyCustomTool::new());
//!
//! let registry = loader.build();
//! # Ok(())
//! # }
//! ```
//!
//! ## 使用 MCP 工具
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! use agentkit::runtime::loader::ToolLoader;
//!
//! # #[cfg(feature = "mcp")]
//! # async fn example(client: rmcp::client::Client) -> Result<(), Box<dyn std::error::Error>> {
//! let loader = ToolLoader::new()
//!     .load_mcp_tools(client)
//!     .await?;
//!
//! let registry = loader.build();
//! # Ok(())
//! # }
//! ```
//!
//! ## 获取加载统计
//!
//! ```rust,no_run
//! use agentkit::runtime::loader::ToolLoader;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let (registry, stats) = ToolLoader::new()
//!     .register(MyCustomTool::new())
//!     .build_with_stats();
//!
//! stats.print();
//! # Ok(())
//! # }
//! ```
//!
//! ## 便捷函数
//!
//! ```rust,no_run
//! use agentkit::runtime::loader::load_tools;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 一键加载 MCP 工具
//! # #[cfg(feature = "mcp")]
//! let (registry, stats) = load_tools(Some(client)).await?;
//!
//! stats.print();
//! # Ok(())
//! # }
//! ```
//!
//! # 关于内置工具
//!
//! `agentkit-runtime` 不提供内置工具。如果需要使用内置工具，可以：
//!
//! 1. 使用 `agentkit` crate 中的工具类型手动注册：
//!
//! ```rust,no_run
//! use agentkit::runtime::loader::ToolLoader;
//! use agentkit::tools::{ShellTool, FileReadTool};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let loader = ToolLoader::new()
//!     .register(ShellTool::new())
//!     .register(FileReadTool::new());
//!
//! let registry = loader.build();
//! # Ok(())
//! # }
//! ```
//!
//! 2. 或者实现自己的工具类型并注册。

use std::path::Path;

use tracing::{debug, info};

use crate::runtime::tool_registry::{ToolRegistry, ToolSource};

/// 工具加载器构建器
///
/// 提供链式 API 来加载和配置工具。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::runtime::loader::ToolLoader;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let loader = ToolLoader::new()
///     .with_auto_namespace()  // 启用自动命名空间
///     .exclude("dangerous")   // 排除危险工具
///     .load_builtin_tools()
///     .load_skills_from_dir("skills")
///     .await?;
///
/// let registry = loader.build();
/// # Ok(())
/// # }
/// ```
pub struct ToolLoader {
    /// 工具注册表
    registry: ToolRegistry,
    /// 是否启用自动命名空间
    auto_namespace: bool,
    /// 工具名称包含过滤模式
    include_patterns: Vec<String>,
    /// 工具名称排除过滤模式
    exclude_patterns: Vec<String>,
}

impl Default for ToolLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolLoader {
    /// 创建新的工具加载器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// let loader = ToolLoader::new();
    /// ```
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            auto_namespace: false,
            include_patterns: vec![],
            exclude_patterns: vec![],
        }
    }

    /// 启用自动命名空间（按来源）
    ///
    /// 启用后，工具会根据来源自动添加命名空间前缀，
    /// 例如：`builtin::shell`、`skill::weather`。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = ToolLoader::new()
    ///     .with_auto_namespace()
    ///     .load_builtin_tools()
    ///     .await?;
    ///
    /// let registry = loader.build();
    /// // 工具名称会是 builtin::shell 而不是 shell
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_auto_namespace(mut self) -> Self {
        self.auto_namespace = true;
        self
    }

    /// 添加工具名称包含过滤
    ///
    /// 只有名称包含指定模式的工具才会被加载。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// let loader = ToolLoader::new()
    ///     .include("shell")   // 只加载包含 shell 的工具
    ///     .include("file");   // 或包含 file 的工具
    /// ```
    pub fn include(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// 添加工具名称排除过滤
    ///
    /// 名称包含指定模式的工具会被排除。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// let loader = ToolLoader::new()
    ///     .exclude("dangerous")  // 排除危险工具
    ///     .exclude("debug");     // 排除调试工具
    /// ```
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// 检查工具名称是否应该被包含
    fn should_include(&self, name: &str) -> bool {
        // 如果有排除模式匹配，则排除
        if self.exclude_patterns.iter().any(|p| name.contains(p)) {
            return false;
        }

        // 如果有包含模式，必须匹配其中一个
        if !self.include_patterns.is_empty() {
            return self.include_patterns.iter().any(|p| name.contains(p));
        }

        true
    }

    /// 注册单个工具
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::loader::ToolLoader;
    /// use agentkit::tools::ShellTool;
    ///
    /// let loader = ToolLoader::new()
    ///     .register(ShellTool::new());
    /// ```
    pub fn register<T: agentkit_core::tool::Tool + 'static>(mut self, tool: T) -> Self {
        let name = tool.name().to_string();
        if self.should_include(&name) {
            debug!(tool.name = %name, "loader.register.tool");
            self.registry = self.registry.register(tool);
        } else {
            debug!(tool.name = %name, "loader.skip.tool");
        }
        self
    }

    /// 注册带来源的工具
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::{loader::ToolLoader, ToolSource};
    /// use agentkit::tools::ShellTool;
    ///
    /// let loader = ToolLoader::new()
    ///     .register_with_source(ShellTool::new(), ToolSource::BuiltIn);
    /// ```
    pub fn register_with_source<T: agentkit_core::tool::Tool + 'static>(
        mut self,
        tool: T,
        source: ToolSource,
    ) -> Self {
        let name = tool.name().to_string();
        if self.should_include(&name) {
            debug!(tool.name = %name, tool.source = ?source, "loader.register.tool");
            self.registry = self.registry.register_with_source(tool, source);
        }
        self
    }

    /// 合并另一个注册表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::{loader::ToolLoader, ToolRegistry};
    ///
    /// let loader1 = ToolLoader::new();
    /// let loader2 = ToolLoader::new()
    ///     .merge(loader1.build());
    /// ```
    pub fn merge(mut self, other: ToolRegistry) -> Self {
        self.registry = self.registry.merge(other);
        self
    }

    /// 加载内置工具
    ///
    /// 内置工具包括：
    /// - `shell`: 执行系统命令
    /// - `cmd_exec`: 受限命令执行（仅 curl）
    /// - `git`: Git 操作
    /// - `file_read`, `file_write`, `file_edit`: 文件操作
    /// - `http_request`, `web_fetch`: 网络请求
    /// - `browse`, `browser_open`: 浏览器操作
    /// - `memory_store`, `memory_recall`: 记忆存储
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `skills` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = ToolLoader::new()
    ///     .load_builtin_tools();
    ///
    /// let registry = loader.build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "skills")]
    pub fn load_builtin_tools(mut self) -> Self {
        use crate::tools::{
            BrowseTool, BrowserOpenTool, CmdExecTool, FileEditTool, FileReadTool, FileWriteTool,
            GitTool, HttpRequestTool, MemoryRecallTool, MemoryStoreTool, ShellTool, WebFetchTool,
        };

        info!("loader.builtin_tools.start");

        // 直接注册具体工具实例
        if self.should_include("shell") {
            self.registry = self
                .registry
                .register_with_source(ShellTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("cmd_exec") {
            self.registry = self
                .registry
                .register_with_source(CmdExecTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("git") {
            self.registry = self
                .registry
                .register_with_source(GitTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("file_read") {
            self.registry = self
                .registry
                .register_with_source(FileReadTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("file_write") {
            self.registry = self
                .registry
                .register_with_source(FileWriteTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("file_edit") {
            self.registry = self
                .registry
                .register_with_source(FileEditTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("http_request") {
            self.registry = self
                .registry
                .register_with_source(HttpRequestTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("web_fetch") {
            self.registry = self
                .registry
                .register_with_source(WebFetchTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("browse") {
            self.registry = self
                .registry
                .register_with_source(BrowseTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("browser_open") {
            self.registry = self
                .registry
                .register_with_source(BrowserOpenTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("memory_store") {
            self.registry = self
                .registry
                .register_with_source(MemoryStoreTool::new(), ToolSource::BuiltIn);
        }
        if self.should_include("memory_recall") {
            self.registry = self
                .registry
                .register_with_source(MemoryRecallTool::new(), ToolSource::BuiltIn);
        }

        info!(count = 12, "loader.builtin_tools.done");
        self
    }

    /// 从目录加载 Skills
    ///
    /// Skills 会被转换为工具并注册，来源标记为 `ToolSource::Skill`。
    ///
    /// # 参数
    ///
    /// - `dir`: Skills 目录路径
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `skills` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = ToolLoader::new()
    ///     .load_skills_from_dir("skills")
    ///     .await?;
    ///
    /// let registry = loader.build();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_skills_from_dir(
        mut self,
        dir: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let dir = dir.as_ref();
        info!(skills.dir = %dir.display(), "loader.skills.start");

        // 使用 agentkit 的 skills 加载器
        #[cfg(feature = "skills")]
        {
            use crate::skills::registry::{SkillRegistry, load_skills_from_dir};

            let skill_registry: SkillRegistry = load_skills_from_dir(dir).await?;
            let skill_tools = skill_registry.as_tools();
            let count = skill_tools.len();

            for tool in skill_tools {
                let name = tool.name().to_string();
                if self.should_include(&name) {
                    self.registry = self.registry.register_arc(tool);
                    debug!(tool.name = %name, "loader.skills.registered");
                }
            }

            info!(count, "loader.skills.done");
        }

        #[cfg(not(feature = "skills"))]
        {
            info!("skills feature not enabled, skipping skills loading");
        }

        Ok(self)
    }

    /// 从 MCP 服务器加载工具
    ///
    /// 连接到 MCP（Model Context Protocol）服务器并加载可用工具。
    ///
    /// # 参数
    ///
    /// - `client`: MCP 客户端
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `mcp` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "mcp")]
    /// use agentkit::runtime::loader::ToolLoader;
    /// # #[cfg(feature = "mcp")]
    /// use rmcp::client::Client;
    ///
    /// # #[cfg(feature = "mcp")]
    /// # async fn example(client: Client) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let loader = ToolLoader::new()
    ///     .load_mcp_tools(client)
    ///     .await?;
    ///
    /// let registry = loader.build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "mcp")]
    pub async fn load_mcp_tools(
        mut self,
        client: crate::mcp::McpClient,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use crate::mcp::McpTool;

        info!("loader.mcp.start");

        let tools = client.list_tools().await?;
        let mut count = 0;

        for mcp_tool in tools {
            let name = mcp_tool.name.clone();
            if self.should_include(&name) {
                let adapter = McpTool::new(client.clone(), mcp_tool);
                self.registry = self.registry.register_with_source(adapter, ToolSource::Mcp);
                count += 1;
                debug!(tool.name = %name, "loader.mcp.registered");
            }
        }

        info!(count, "loader.mcp.done");
        Ok(self)
    }

    /// 从 A2A 代理加载工具
    ///
    /// 从 A2A（Agent-to-Agent）协议加载远程代理的工具。
    ///
    /// # 参数
    ///
    /// - `agent_card`: A2A 代理卡片
    /// - `transport`: A2A 传输层
    ///
    /// # Feature 标志
    ///
    /// 需要启用 `a2a` feature。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "a2a")]
    /// use agentkit::runtime::loader::ToolLoader;
    /// # #[cfg(feature = "a2a")]
    /// use ra2a::types::AgentCard;
    ///
    /// # #[cfg(feature = "a2a")]
    /// # async fn example(card: AgentCard, transport: impl ra2a::transport::Transport) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let loader = ToolLoader::new()
    ///     .load_a2a_tools(card, transport)
    ///     .await?;
    ///
    /// let registry = loader.build();
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "a2a")]
    pub async fn load_a2a_tools(
        mut self,
        agent_card: ra2a::types::AgentCard,
        transport: impl ra2a::transport::Transport + 'static,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use crate::a2a::A2AToolAdapter;

        info!("loader.a2a.start");

        let mut count = 0;
        if let Some(capabilities) = &agent_card.capabilities {
            for capability in &capabilities.tools {
                let name = capability.name.clone();
                if self.should_include(&name) {
                    let adapter = A2AToolAdapter::new(capability.clone(), transport.clone());
                    self.registry = self.registry.register_with_source(adapter, ToolSource::A2A);
                    count += 1;
                    debug!(tool.name = %name, "loader.a2a.registered");
                }
            }
        }

        info!(count, "loader.a2a.done");
        Ok(self)
    }

    /// 构建最终的 ToolRegistry
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = ToolLoader::new()
    ///     .load_builtin_tools()
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> ToolRegistry {
        info!(
            total_tools = self.registry.len(),
            enabled_tools = self.registry.enabled_len(),
            "loader.build"
        );
        self.registry
    }

    /// 构建并获取 ToolRegistry 和统计信息
    ///
    /// # 返回值
    ///
    /// 返回 `(ToolRegistry, ToolLoadStats)` 元组。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use agentkit::runtime::loader::ToolLoader;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let (registry, stats) = ToolLoader::new()
    ///     .load_builtin_tools()
    ///     .build_with_stats();
    ///
    /// stats.print();
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_with_stats(self) -> (ToolRegistry, ToolLoadStats) {
        let total = self.registry.len();
        let enabled = self.registry.enabled_len();

        let stats = ToolLoadStats {
            total,
            enabled,
            builtin: self.registry.filter_by_source(ToolSource::BuiltIn).len(),
            skill: self.registry.filter_by_source(ToolSource::Skill).len(),
            mcp: self.registry.filter_by_source(ToolSource::Mcp).len(),
            a2a: self.registry.filter_by_source(ToolSource::A2A).len(),
            custom: self.registry.filter_by_source(ToolSource::Custom).len(),
        };

        info!(
            total = stats.total,
            enabled = stats.enabled,
            builtin = stats.builtin,
            skill = stats.skill,
            mcp = stats.mcp,
            a2a = stats.a2a,
            custom = stats.custom,
            "loader.build_with_stats"
        );

        (self.registry, stats)
    }
}

/// 工具加载统计
///
/// 包含工具加载的详细信息。
///
/// # 字段说明
///
/// - `total`: 总工具数
/// - `enabled`: 启用的工具数
/// - `builtin`: 内置工具数
/// - `skill`: Skill 工具数
/// - `mcp`: MCP 工具数
/// - `a2a`: A2A 工具数
/// - `custom`: 自定义工具数
///
/// # 示例
///
/// ```rust
/// use agentkit::runtime::loader::ToolLoadStats;
///
/// let stats = ToolLoadStats {
///     total: 15,
///     enabled: 14,
///     builtin: 12,
///     skill: 2,
///     mcp: 0,
///     a2a: 0,
///     custom: 1,
/// };
///
/// stats.print();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ToolLoadStats {
    /// 总工具数
    pub total: usize,
    /// 启用的工具数
    pub enabled: usize,
    /// 内置工具数
    pub builtin: usize,
    /// Skill 工具数
    pub skill: usize,
    /// MCP 工具数
    pub mcp: usize,
    /// A2A 工具数
    pub a2a: usize,
    /// 自定义工具数
    pub custom: usize,
}

impl ToolLoadStats {
    /// 创建新的空统计
    pub fn new() -> Self {
        Self::default()
    }

    /// 打印统计信息到标准输出
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit::runtime::loader::ToolLoadStats;
    ///
    /// let stats = ToolLoadStats {
    ///     total: 15,
    ///     enabled: 14,
    ///     builtin: 12,
    ///     skill: 2,
    ///     mcp: 0,
    ///     a2a: 0,
    ///     custom: 1,
    /// };
    ///
    /// stats.print();
    /// // 输出:
    /// // Tool Loading Statistics:
    /// //   Total:   15
    /// //   Enabled: 14
    /// //   By Source:
    /// //     - BuiltIn: 12
    /// //     - Skill:   2
    /// //     - MCP:     0
    /// //     - A2A:     0
    /// //     - Custom:  1
    /// ```
    pub fn print(&self) {
        println!("Tool Loading Statistics:");
        println!("  Total:   {}", self.total);
        println!("  Enabled: {}", self.enabled);
        println!("  By Source:");
        println!("    - BuiltIn: {}", self.builtin);
        println!("    - Skill:   {}", self.skill);
        println!("    - MCP:     {}", self.mcp);
        println!("    - A2A:     {}", self.a2a);
        println!("    - Custom:  {}", self.custom);
    }
}

/// 便捷的工具加载函数
///
/// 一键加载内置工具和 Skills。
///
/// # 参数
///
/// - `skill_dir`: Skills 目录路径（可选）
/// - `include_builtin`: 是否包含内置工具
///
/// # 返回值
///
/// 返回 `(ToolRegistry, ToolLoadStats)` 元组。
///
/// # 示例
///
/// ```rust,no_run
/// use agentkit::runtime::loader::load_all_tools;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let (registry, stats) = load_all_tools(
///     Some("skills"),  // Skills 目录
///     true,            // 包含内置工具
/// ).await?;
///
/// stats.print();
/// # Ok(())
/// # }
/// ```
pub async fn load_all_tools(
    skill_dir: Option<impl AsRef<Path>>,
    include_builtin: bool,
) -> Result<(ToolRegistry, ToolLoadStats), Box<dyn std::error::Error + Send + Sync>> {
    let mut loader = ToolLoader::new();

    if include_builtin {
        #[cfg(feature = "skills")]
        {
            loader = loader.load_builtin_tools();
        }
    }

    if let Some(dir) = skill_dir {
        loader = loader.load_skills_from_dir(dir).await?;
    }

    Ok(loader.build_with_stats())
}
