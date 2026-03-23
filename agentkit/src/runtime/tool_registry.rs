//! 工具注册表模块
//!
//! # 概述
//!
//! `ToolRegistry` 是 Agentkit 运行时中管理所有可用工具的核心组件。
//! 它支持从多种来源注册工具，并提供丰富的查询和过滤功能。
//!
//! # 主要特性
//!
//! - **多来源支持**: 支持内置工具、Skills、MCP、A2A 等多种工具来源
//! - **命名空间**: 避免工具名称冲突
//! - **元数据管理**: 记录工具来源、分类、启用状态等信息
//! - **动态过滤**: 按来源、分类、标签过滤工具
//! - **灵活注册**: 支持单个注册、批量注册、合并注册表
//!
//! # 使用示例
//!
//! ## 基本使用
//!
//! ```rust
//! use agentkit_runtime::{ToolRegistry, ToolSource};
//! use agentkit::tools::ShellTool;
//!
//! // 创建注册表
//! let registry = ToolRegistry::new()
//!     .register(ShellTool::new());
//!
//! // 获取工具
//! let tool = registry.get("shell");
//! assert!(tool.is_some());
//!
//! // 获取所有工具定义（用于注册到 LLM）
//! let definitions = registry.definitions();
//! ```
//!
//! ## 使用命名空间
//!
//! ```rust
//! use agentkit_runtime::{ToolRegistry, ToolSource};
//! use agentkit::tools::{ShellTool, FileReadTool};
//!
//! // 使用命名空间避免冲突
//! let registry1 = ToolRegistry::new()
//!     .with_namespace("system")
//!     .register(ShellTool::new());
//!
//! let registry2 = ToolRegistry::new()
//!     .with_namespace("file")
//!     .register(FileReadTool::new());
//!
//! // 合并注册表
//! let merged = registry1.merge(registry2);
//!
//! // 通过完整名称访问
//! assert!(merged.get("system::shell").is_some());
//! assert!(merged.get("file::file_read").is_some());
//! ```
//!
//! ## 按来源过滤
//!
//! ```rust
//! use agentkit_runtime::{ToolRegistry, ToolSource};
//! use agentkit::tools::{ShellTool, FileReadTool};
//!
//! let registry = ToolRegistry::new()
//!     .register_with_source(ShellTool::new(), ToolSource::BuiltIn)
//!     .register_with_source(FileReadTool::new(), ToolSource::BuiltIn);
//!
//! // 按来源过滤
//! let builtin_tools = registry.filter_by_source(ToolSource::BuiltIn);
//! assert_eq!(builtin_tools.len(), 2);
//! ```
//!
//! ## 动态启用/禁用工具
//!
//! ```rust
//! use agentkit_runtime::ToolRegistry;
//! use agentkit::tools::ShellTool;
//!
//! let mut registry = ToolRegistry::new()
//!     .register(ShellTool::new());
//!
//! // 禁用工具
//! registry.set_tool_enabled("shell", false);
//!
//! // 禁用的工具不会被获取
//! assert!(registry.get("shell").is_none());
//!
//! // 但仍在注册表中
//! assert_eq!(registry.len(), 1);
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use agentkit_core::tool::Tool;
use agentkit_core::tool::ToolCategory;
use agentkit_core::tool::types::ToolDefinition;

/// 工具来源类型枚举
///
/// 用于标识工具的来源，便于管理和过滤。
///
/// # 变体说明
///
/// - `BuiltIn`: 内置工具，如 shell、file、http 等基础工具
/// - `Skill`: 从 Skills 目录加载的技能转换的工具
/// - `Mcp`: 从 MCP（Model Context Protocol）服务器加载的工具
/// - `A2A`: 从 A2A（Agent-to-Agent）协议加载的工具
/// - `Custom`: 用户自定义工具
///
/// # 示例
///
/// ```rust
/// use agentkit_runtime::ToolSource;
///
/// let source = ToolSource::BuiltIn;
/// assert_eq!(source.as_str(), "builtin");
///
/// let skill_source = ToolSource::Skill;
/// assert_eq!(skill_source.as_str(), "skill");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolSource {
    /// 内置工具（如 shell、file、http 等）
    BuiltIn,
    /// 从 Skill 转换的工具
    Skill,
    /// 从 MCP 服务器加载的工具
    Mcp,
    /// 从 A2A 协议加载的工具
    A2A,
    /// 用户自定义工具
    Custom,
}

impl ToolSource {
    /// 获取来源的字符串表示
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolSource;
    ///
    /// assert_eq!(ToolSource::BuiltIn.as_str(), "builtin");
    /// assert_eq!(ToolSource::Skill.as_str(), "skill");
    /// assert_eq!(ToolSource::Mcp.as_str(), "mcp");
    /// assert_eq!(ToolSource::A2A.as_str(), "a2a");
    /// assert_eq!(ToolSource::Custom.as_str(), "custom");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolSource::BuiltIn => "builtin",
            ToolSource::Skill => "skill",
            ToolSource::Mcp => "mcp",
            ToolSource::A2A => "a2a",
            ToolSource::Custom => "custom",
        }
    }
}

/// 工具元数据
///
/// 存储工具的附加信息，用于管理和过滤。
///
/// # 字段说明
///
/// - `source`: 工具来源
/// - `categories`: 工具分类列表（支持多标签）
/// - `enabled`: 是否启用（禁用的工具不会被执行）
/// - `tags`: 额外标签，用于自定义过滤
///
/// # 示例
///
/// ```rust
/// use agentkit_runtime::{ToolMetadata, ToolSource};
/// use agentkit_core::tool::ToolCategory;
///
/// let metadata = ToolMetadata {
///     source: ToolSource::BuiltIn,
///     categories: vec![ToolCategory::System],
///     enabled: true,
///     tags: vec!["critical".to_string()],
/// };
///
/// assert_eq!(metadata.source, ToolSource::BuiltIn);
/// assert!(metadata.enabled);
/// ```
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    /// 工具来源
    pub source: ToolSource,
    /// 工具分类
    pub categories: Vec<ToolCategory>,
    /// 是否启用
    pub enabled: bool,
    /// 额外标签
    pub tags: Vec<String>,
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self {
            source: ToolSource::Custom,
            categories: vec![ToolCategory::Basic],
            enabled: true,
            tags: vec![],
        }
    }
}

/// 带元数据的工具包装
///
/// 将工具实例与其元数据一起包装，便于统一管理。
///
/// # 示例
///
/// ```rust
/// use agentkit_runtime::{ToolWrapper, ToolSource};
/// use agentkit::tools::ShellTool;
///
/// let wrapper = ToolWrapper::new(ShellTool::new())
///     .with_source(ToolSource::BuiltIn)
///     .with_tags(vec!["system".to_string(), "critical".to_string()]);
///
/// assert_eq!(wrapper.metadata.source, ToolSource::BuiltIn);
/// assert!(wrapper.metadata.tags.contains(&"system".to_string()));
/// ```
#[derive(Clone)]
pub struct ToolWrapper {
    /// 工具实例
    pub tool: Arc<dyn Tool>,
    /// 工具元数据
    pub metadata: ToolMetadata,
}

impl ToolWrapper {
    /// 从工具实例创建包装
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolWrapper;
    /// use agentkit::tools::ShellTool;
    ///
    /// let wrapper = ToolWrapper::new(ShellTool::new());
    /// assert_eq!(wrapper.tool.name(), "shell");
    /// ```
    pub fn new<T: Tool + 'static>(tool: T) -> Self {
        let categories = tool.categories().to_vec();
        Self {
            tool: Arc::new(tool),
            metadata: ToolMetadata {
                source: ToolSource::Custom,
                categories,
                enabled: true,
                tags: vec![],
            },
        }
    }

    /// 从 Arc<dyn Tool> 创建包装
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use agentkit_runtime::ToolWrapper;
    /// use agentkit::tools::ShellTool;
    ///
    /// let tool: Arc<dyn agentkit_core::tool::Tool> = Arc::new(ShellTool::new());
    /// let wrapper = ToolWrapper::new_arc(tool);
    /// ```
    pub fn new_arc(tool: Arc<dyn Tool>) -> Self {
        let categories = tool.categories().to_vec();
        Self {
            tool,
            metadata: ToolMetadata {
                source: ToolSource::Custom,
                categories,
                enabled: true,
                tags: vec![],
            },
        }
    }

    /// 设置工具来源
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::{ToolWrapper, ToolSource};
    /// use agentkit::tools::ShellTool;
    ///
    /// let wrapper = ToolWrapper::new(ShellTool::new())
    ///     .with_source(ToolSource::BuiltIn);
    ///
    /// assert_eq!(wrapper.metadata.source, ToolSource::BuiltIn);
    /// ```
    pub fn with_source(mut self, source: ToolSource) -> Self {
        self.metadata.source = source;
        self
    }

    /// 设置工具标签
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolWrapper;
    /// use agentkit::tools::ShellTool;
    ///
    /// let wrapper = ToolWrapper::new(ShellTool::new())
    ///     .with_tags(vec!["system".to_string(), "critical".to_string()]);
    ///
    /// assert!(wrapper.metadata.tags.contains(&"system".to_string()));
    /// ```
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags = tags;
        self
    }

    /// 设置启用状态
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolWrapper;
    /// use agentkit::tools::ShellTool;
    ///
    /// let wrapper = ToolWrapper::new(ShellTool::new())
    ///     .with_enabled(false);
    ///
    /// assert!(!wrapper.metadata.enabled);
    /// ```
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.metadata.enabled = enabled;
        self
    }
}

/// Tool 注册表：集中管理所有可用工具，支持多种来源和元数据管理。
///
/// # 特性
///
/// - 支持从不同来源注册工具（内置、Skill、MCP、A2A、自定义）
/// - 支持按分类、来源、标签过滤工具
/// - 支持动态启用/禁用工具
/// - 支持工具命名空间（避免名称冲突）
/// - 支持多个注册表合并
///
/// # 示例
///
/// ## 基本注册和查询
///
/// ```rust
/// use agentkit_runtime::ToolRegistry;
/// use agentkit::tools::ShellTool;
///
/// let registry = ToolRegistry::new()
///     .register(ShellTool::new());
///
/// // 获取工具
/// assert!(registry.get("shell").is_some());
///
/// // 获取工具定义
/// let definitions = registry.definitions();
/// assert!(!definitions.is_empty());
/// ```
///
/// ## 使用命名空间
///
/// ```rust
/// use agentkit_runtime::ToolRegistry;
/// use agentkit::tools::ShellTool;
///
/// let registry = ToolRegistry::new()
///     .with_namespace("system")
///     .register(ShellTool::new());
///
/// // 需要使用完整名称
/// assert!(registry.get("system::shell").is_some());
/// assert!(registry.get("shell").is_none());
/// ```
///
/// ## 合并注册表
///
/// ```rust
/// use agentkit_runtime::ToolRegistry;
/// use agentkit::tools::{ShellTool, FileReadTool};
///
/// let registry1 = ToolRegistry::new()
///     .with_namespace("sys")
///     .register(ShellTool::new());
///
/// let registry2 = ToolRegistry::new()
///     .with_namespace("file")
///     .register(FileReadTool::new());
///
/// let merged = registry1.merge(registry2);
/// assert!(merged.get("sys::shell").is_some());
/// assert!(merged.get("file::file_read").is_some());
/// ```
#[derive(Default, Clone)]
pub struct ToolRegistry {
    /// 工具映射表（名称 -> 包装）
    tools: HashMap<String, ToolWrapper>,
    /// 命名空间前缀（可选）
    namespace_prefix: Option<String>,
}

impl ToolRegistry {
    /// 创建新的空注册表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            namespace_prefix: None,
        }
    }

    /// 设置命名空间前缀
    ///
    /// 命名空间用于避免工具名称冲突，特别是在合并多个注册表时。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .with_namespace("system")
    ///     .register(ShellTool::new());
    ///
    /// // 工具名称会被添加前缀
    /// assert!(registry.get("system::shell").is_some());
    /// ```
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace_prefix = Some(namespace.into());
        self
    }

    /// 获取带命名空间的工具名称
    fn namespaced_name(&self, name: &str) -> String {
        if let Some(prefix) = &self.namespace_prefix {
            format!("{}::{}", prefix, name)
        } else {
            name.to_string()
        }
    }

    /// 注册一个工具
    ///
    /// 工具会被自动包装并添加到注册表。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// assert_eq!(registry.len(), 1);
    /// ```
    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        let wrapper = ToolWrapper::new(tool);
        let name = self.namespaced_name(wrapper.tool.name());
        self.tools.insert(name, wrapper);
        self
    }

    /// 注册一个已包装的工具
    ///
    /// 用于注册已经设置了元数据的工具包装。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::{ToolRegistry, ToolWrapper, ToolSource};
    /// use agentkit::tools::ShellTool;
    ///
    /// let wrapper = ToolWrapper::new(ShellTool::new())
    ///     .with_source(ToolSource::BuiltIn);
    ///
    /// let registry = ToolRegistry::new()
    ///     .register_wrapper(wrapper);
    /// ```
    pub fn register_wrapper(mut self, wrapper: ToolWrapper) -> Self {
        let name = self.namespaced_name(wrapper.tool.name());
        self.tools.insert(name, wrapper);
        self
    }

    /// 注册一个 Arc<dyn Tool>
    ///
    /// 用于注册已经包装为 trait 对象的工具。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let tool: Arc<dyn agentkit_core::tool::Tool> = Arc::new(ShellTool::new());
    /// let registry = ToolRegistry::new().register_arc(tool);
    /// ```
    pub fn register_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        let name = self.namespaced_name(tool.name());
        self.tools.insert(
            name,
            ToolWrapper {
                tool,
                metadata: ToolMetadata::default(),
            },
        );
        self
    }

    /// 注册一个带来源的工具
    ///
    /// 便捷方法，用于注册工具并指定来源。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::{ToolRegistry, ToolSource};
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register_with_source(ShellTool::new(), ToolSource::BuiltIn);
    /// ```
    pub fn register_with_source<T: Tool + 'static>(self, tool: T, source: ToolSource) -> Self {
        self.register_wrapper(ToolWrapper::new(tool).with_source(source))
    }

    /// 注册多个工具
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::{ToolRegistry, ToolWrapper};
    /// use agentkit::tools::{ShellTool, FileReadTool};
    ///
    /// let registry = ToolRegistry::new()
    ///     .register_all(vec![
    ///         ToolWrapper::new(ShellTool::new()),
    ///         ToolWrapper::new(FileReadTool::new()),
    ///     ]);
    ///
    /// assert_eq!(registry.len(), 2);
    /// ```
    pub fn register_all<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = ToolWrapper>,
    {
        for wrapper in tools {
            let name = self.namespaced_name(wrapper.tool.name());
            self.tools.insert(name, wrapper);
        }
        self
    }

    /// 从另一个 ToolRegistry 合并工具
    ///
    /// 如果名称冲突，会自动添加对方的命名空间或前缀。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::{ShellTool, FileReadTool};
    ///
    /// let registry1 = ToolRegistry::new()
    ///     .with_namespace("sys")
    ///     .register(ShellTool::new());
    ///
    /// let registry2 = ToolRegistry::new()
    ///     .with_namespace("file")
    ///     .register(FileReadTool::new());
    ///
    /// let merged = registry1.merge(registry2);
    /// assert!(merged.get("sys::shell").is_some());
    /// assert!(merged.get("file::file_read").is_some());
    /// ```
    pub fn merge(mut self, other: ToolRegistry) -> Self {
        for (name, wrapper) in other.tools {
            // 如果名称冲突，使用对方的命名空间或添加前缀
            if self.tools.contains_key(&name) {
                let new_name = if let Some(prefix) = &other.namespace_prefix {
                    format!("{}::{}", prefix, name)
                } else {
                    format!("merged::{}", name)
                };
                self.tools.insert(new_name, wrapper);
            } else {
                self.tools.insert(name, wrapper);
            }
        }
        self
    }

    /// 按分类过滤工具
    ///
    /// 返回所有属于指定分类且启用的工具。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::{ToolRegistry, ToolSource};
    /// use agentkit::tools::ShellTool;
    /// use agentkit_core::tool::ToolCategory;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register_with_source(ShellTool::new(), ToolSource::BuiltIn);
    ///
    /// let system_tools = registry.filter_by_category(ToolCategory::System);
    /// assert!(!system_tools.is_empty());
    /// ```
    pub fn filter_by_category(&self, category: ToolCategory) -> Vec<&ToolWrapper> {
        self.tools
            .values()
            .filter(|w| w.metadata.categories.contains(&category))
            .filter(|w| w.metadata.enabled)
            .collect()
    }

    /// 按来源过滤工具
    ///
    /// 返回所有属于指定来源且启用的工具。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::{ToolRegistry, ToolSource};
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register_with_source(ShellTool::new(), ToolSource::BuiltIn);
    ///
    /// let builtin_tools = registry.filter_by_source(ToolSource::BuiltIn);
    /// assert_eq!(builtin_tools.len(), 1);
    /// ```
    pub fn filter_by_source(&self, source: ToolSource) -> Vec<&ToolWrapper> {
        self.tools
            .values()
            .filter(|w| w.metadata.source == source)
            .filter(|w| w.metadata.enabled)
            .collect()
    }

    /// 按标签过滤工具
    ///
    /// 返回所有包含任一指定标签且启用的工具。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register_wrapper(
    ///         agentkit_runtime::ToolWrapper::new(ShellTool::new())
    ///             .with_tags(vec!["system".to_string(), "critical".to_string()])
    ///     );
    ///
    /// let critical_tools = registry.filter_by_tags(&["critical".to_string()]);
    /// assert!(!critical_tools.is_empty());
    /// ```
    pub fn filter_by_tags(&self, tags: &[String]) -> Vec<&ToolWrapper> {
        self.tools
            .values()
            .filter(|w| tags.iter().any(|tag| w.metadata.tags.contains(tag)) && w.metadata.enabled)
            .collect()
    }

    /// 获取所有启用的工具定义（用于注册到 LLM）
    ///
    /// 返回所有启用工具的 `ToolDefinition` 列表，可用于注册到 LLM 的 function calling。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// let definitions = registry.definitions();
    /// assert_eq!(definitions.len(), 1);
    /// assert_eq!(definitions[0].name, "shell");
    /// ```
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|w| w.metadata.enabled)
            .map(|wrapper| ToolDefinition {
                name: wrapper.tool.name().to_string(),
                description: wrapper.tool.description().map(|s| s.to_string()),
                input_schema: wrapper.tool.input_schema(),
            })
            .collect()
    }

    /// 获取所有启用的工具
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// let tools = registry.enabled_tools();
    /// assert_eq!(tools.len(), 1);
    /// ```
    pub fn enabled_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|w| w.metadata.enabled)
            .map(|w| w.tool.clone())
            .collect()
    }

    /// 根据名称获取工具
    ///
    /// 先尝试直接查找，如果找不到则尝试添加命名空间前缀。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .with_namespace("sys")
    ///     .register(ShellTool::new());
    ///
    /// // 可以通过完整名称获取
    /// assert!(registry.get("sys::shell").is_some());
    /// ```
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        // 先尝试直接查找
        if let Some(wrapper) = self.tools.get(name) {
            if wrapper.metadata.enabled {
                return Some(wrapper.tool.clone());
            }
            return None;
        }

        // 尝试带命名空间查找
        if let Some(prefix) = &self.namespace_prefix {
            let namespaced = format!("{}::{}", prefix, name);
            if let Some(wrapper) = self.tools.get(&namespaced) {
                if wrapper.metadata.enabled {
                    return Some(wrapper.tool.clone());
                }
            }
        }

        None
    }

    /// 启用/禁用工具
    ///
    /// # 返回值
    ///
    /// 如果找到工具则返回 `true`，否则返回 `false`。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let mut registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// // 禁用工具
    /// assert!(registry.set_tool_enabled("shell", false));
    ///
    /// // 禁用的工具无法获取
    /// assert!(registry.get("shell").is_none());
    /// ```
    pub fn set_tool_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(wrapper) = self.tools.get_mut(name) {
            wrapper.metadata.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// 获取工具总数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// assert_eq!(registry.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 获取启用的工具数量
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// assert_eq!(registry.enabled_len(), 1);
    /// ```
    pub fn enabled_len(&self) -> usize {
        self.tools.values().filter(|w| w.metadata.enabled).count()
    }

    /// 检查是否为空
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// 获取所有工具名称
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// let names = registry.tool_names();
    /// assert_eq!(names.len(), 1);
    /// assert_eq!(names[0], "shell");
    /// ```
    pub fn tool_names(&self) -> Vec<&String> {
        self.tools.keys().collect()
    }

    /// 清除所有工具
    ///
    /// # 示例
    ///
    /// ```rust
    /// use agentkit_runtime::ToolRegistry;
    /// use agentkit::tools::ShellTool;
    ///
    /// let mut registry = ToolRegistry::new()
    ///     .register(ShellTool::new());
    ///
    /// registry.clear();
    /// assert!(registry.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.tools.clear();
    }

    /// 调用指定工具。
    ///
    /// # 参数
    ///
    /// * `name` - 工具名称
    /// * `input` - 工具输入参数
    ///
    /// # 返回
    ///
    /// 返回工具执行结果
    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, agentkit_core::error::ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| agentkit_core::error::ToolError::NotFound {
                name: name.to_string(),
            })?;
        tool.call(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentkit_core::error::ToolError;
    use serde_json::Value;
    use serde_json::json;

    struct TestTool {
        name: String,
    }

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn categories(&self) -> &'static [ToolCategory] {
            &[ToolCategory::Basic]
        }

        fn input_schema(&self) -> Value {
            json!({"type": "object"})
        }

        async fn call(&self, _input: Value) -> Result<Value, ToolError> {
            Ok(json!({"ok": true}))
        }
    }

    #[test]
    fn test_tool_registry_namespace() {
        let registry = ToolRegistry::new()
            .with_namespace("test")
            .register(TestTool {
                name: "my_tool".to_string(),
            });

        // 带命名空间查找应该成功
        assert!(registry.get("test::my_tool").is_some());
        // 不带命名空间查找也应该成功（因为会尝试两种查找）
        assert!(registry.get("my_tool").is_some());
    }

    #[test]
    fn test_tool_registry_filter_by_source() {
        let registry = ToolRegistry::new()
            .register_with_source(
                TestTool {
                    name: "builtin_tool".to_string(),
                },
                ToolSource::BuiltIn,
            )
            .register_with_source(
                TestTool {
                    name: "skill_tool".to_string(),
                },
                ToolSource::Skill,
            );

        let builtin_tools = registry.filter_by_source(ToolSource::BuiltIn);
        assert_eq!(builtin_tools.len(), 1);
        assert_eq!(builtin_tools[0].tool.name(), "builtin_tool");

        let skill_tools = registry.filter_by_source(ToolSource::Skill);
        assert_eq!(skill_tools.len(), 1);
        assert_eq!(skill_tools[0].tool.name(), "skill_tool");
    }

    #[test]
    fn test_tool_registry_merge() {
        let registry1 = ToolRegistry::new()
            .with_namespace("ns1")
            .register(TestTool {
                name: "tool".to_string(),
            });

        let registry2 = ToolRegistry::new()
            .with_namespace("ns2")
            .register(TestTool {
                name: "tool".to_string(),
            });

        let merged = registry1.merge(registry2);
        // ns1 的工具
        assert!(merged.get("ns1::tool").is_some());
        // ns2 的工具（合并时会添加额外前缀）
        assert!(merged.get("ns2::tool").is_some());
    }
}
