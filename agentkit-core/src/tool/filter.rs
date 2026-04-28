//! 工具过滤分组系统
//!
//! 本模块提供工具可见性控制，支持：
//! - `always` 模式：工具始终对 LLM 可见
//! - `dynamic` 模式：工具仅在用户消息包含特定关键词时可见
//!
//! 这对 MCP 工具数量较多时非常有用，可以避免 token 浪费。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::ToolDefinition;

/// 工具可见性模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToolVisibility {
    /// 始终可见
    #[default]
    Always,
    /// 动态可见（基于关键词匹配）
    Dynamic,
}

/// 工具过滤器配置
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolFilterConfig {
    /// 工具可见性映射（工具名 -> 可见性模式）
    pub visibility: HashMap<String, ToolVisibility>,
    /// 动态工具的关键词映射（工具名 -> 关键词列表）
    pub dynamic_keywords: HashMap<String, Vec<String>>,
    /// 默认可见性（未配置的工具使用此设置）
    pub default_visibility: ToolVisibility,
    /// 是否启用过滤
    pub enabled: bool,
}

impl ToolFilterConfig {
    /// 创建新的过滤器配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置工具为始终可见
    pub fn always_visible(mut self, tool_name: impl Into<String>) -> Self {
        self.visibility
            .insert(tool_name.into(), ToolVisibility::Always);
        self
    }

    /// 设置工具为动态可见
    pub fn dynamic_visible(mut self, tool_name: impl Into<String>, keywords: Vec<String>) -> Self {
        let name = tool_name.into();
        self.visibility
            .insert(name.clone(), ToolVisibility::Dynamic);
        self.dynamic_keywords.insert(name, keywords);
        self
    }

    /// 设置默认可见性
    pub fn with_default_visibility(mut self, visibility: ToolVisibility) -> Self {
        self.default_visibility = visibility;
        self
    }

    /// 启用过滤
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// 禁用过滤
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// 检查工具是否应该始终可见
    pub fn is_always_visible(&self, tool_name: &str) -> bool {
        if !self.enabled {
            return true;
        }
        match self.visibility.get(tool_name) {
            Some(ToolVisibility::Always) => true,
            Some(ToolVisibility::Dynamic) => false,
            None => matches!(self.default_visibility, ToolVisibility::Always),
        }
    }

    /// 检查工具是否是动态的
    pub fn is_dynamic(&self, tool_name: &str) -> bool {
        if !self.enabled {
            return false;
        }
        matches!(
            self.visibility.get(tool_name),
            Some(ToolVisibility::Dynamic)
        )
    }

    /// 获取工具的关键词列表
    pub fn get_keywords(&self, tool_name: &str) -> &[String] {
        self.dynamic_keywords
            .get(tool_name)
            .map_or(&[], |v| v.as_slice())
    }

    /// 检查用户消息是否匹配工具的关键词
    pub fn matches_keywords(&self, tool_name: &str, user_message: &str) -> bool {
        let keywords = self.get_keywords(tool_name);
        if keywords.is_empty() {
            return true; // 没有关键词时默认匹配
        }

        let message_lower = user_message.to_lowercase();
        keywords
            .iter()
            .any(|kw| message_lower.contains(&kw.to_lowercase()))
    }
}

/// 工具过滤器
#[derive(Debug, Clone)]
pub struct ToolFilter {
    config: ToolFilterConfig,
}

impl ToolFilter {
    /// 创建新的工具过滤器
    pub fn new(config: ToolFilterConfig) -> Self {
        Self { config }
    }

    /// 获取配置
    pub fn config(&self) -> &ToolFilterConfig {
        &self.config
    }

    /// 过滤工具列表
    ///
    /// # 参数
    /// - `tools`: 所有可用工具
    /// - `user_message`: 用户消息（用于动态匹配）
    ///
    /// # 返回
    /// 过滤后的工具列表
    pub fn filter_tools(
        &self,
        tools: Vec<ToolDefinition>,
        user_message: &str,
    ) -> Vec<ToolDefinition> {
        if !self.config.enabled {
            return tools;
        }

        tools
            .into_iter()
            .filter(|tool| self.should_include_tool(&tool.name, user_message))
            .collect()
    }

    /// 检查工具是否应该包含
    fn should_include_tool(&self, tool_name: &str, user_message: &str) -> bool {
        // 始终可见的工具
        if self.config.is_always_visible(tool_name) {
            return true;
        }

        // 动态工具需要匹配关键词
        if self.config.is_dynamic(tool_name) {
            return self.config.matches_keywords(tool_name, user_message);
        }

        false
    }

    /// 获取工具可见性统计
    pub fn get_stats(&self, all_tools: &[ToolDefinition]) -> ToolFilterStats {
        let total = all_tools.len();
        let always_visible = all_tools
            .iter()
            .filter(|t| self.config.is_always_visible(&t.name))
            .count();
        let dynamic = all_tools
            .iter()
            .filter(|t| self.config.is_dynamic(&t.name))
            .count();

        ToolFilterStats {
            total,
            always_visible,
            dynamic,
            hidden: total - always_visible - dynamic,
        }
    }
}

/// 工具过滤统计
#[derive(Debug, Clone, Copy, Default)]
pub struct ToolFilterStats {
    /// 总工具数
    pub total: usize,
    /// 始终可见的工具数
    pub always_visible: usize,
    /// 动态工具数
    pub dynamic: usize,
    /// 隐藏的工具数
    pub hidden: usize,
}

impl ToolFilterStats {
    /// 获取可见工具数
    pub fn visible_count(&self) -> usize {
        self.always_visible + self.dynamic
    }

    /// 获取可见比例
    pub fn visible_ratio(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.visible_count() as f64 / self.total as f64
        }
    }
}

/// 工具组定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGroup {
    /// 组名称
    pub name: String,
    /// 组描述
    pub description: String,
    /// 组内工具名称列表
    pub tools: Vec<String>,
    /// 组的可见性
    pub visibility: ToolVisibility,
    /// 动态关键词（仅当 visibility 为 Dynamic 时使用）
    pub keywords: Vec<String>,
}

impl ToolGroup {
    /// 创建新的工具组
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            tools: Vec::new(),
            visibility: ToolVisibility::Always,
            keywords: Vec::new(),
        }
    }

    /// 添加工具到组
    pub fn add_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tools.push(tool_name.into());
        self
    }

    /// 设置可见性为动态
    pub fn with_dynamic_visibility(mut self, keywords: Vec<String>) -> Self {
        self.visibility = ToolVisibility::Dynamic;
        self.keywords = keywords;
        self
    }

    /// 检查工具是否在组中
    pub fn contains(&self, tool_name: &str) -> bool {
        self.tools.contains(&tool_name.to_string())
    }
}

/// 工具组管理器
#[derive(Debug, Clone, Default)]
pub struct ToolGroupManager {
    groups: Vec<ToolGroup>,
}

impl ToolGroupManager {
    /// 创建新的组管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加工具组
    pub fn add_group(mut self, group: ToolGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// 从组配置生成过滤器配置
    pub fn to_filter_config(&self) -> ToolFilterConfig {
        let mut config = ToolFilterConfig::new().enable();

        for group in &self.groups {
            for tool_name in &group.tools {
                match group.visibility {
                    ToolVisibility::Always => {
                        config
                            .visibility
                            .insert(tool_name.clone(), ToolVisibility::Always);
                    }
                    ToolVisibility::Dynamic => {
                        config
                            .visibility
                            .insert(tool_name.clone(), ToolVisibility::Dynamic);
                        config
                            .dynamic_keywords
                            .insert(tool_name.clone(), group.keywords.clone());
                    }
                }
            }
        }

        config
    }

    /// 查找工具所属的组
    pub fn find_group_for_tool(&self, tool_name: &str) -> Option<&ToolGroup> {
        self.groups.iter().find(|g| g.contains(tool_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tool(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: Some(format!("{name} tool")),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    #[test]
    fn test_tool_filter_config() {
        let config = ToolFilterConfig::new()
            .always_visible("file_read")
            .always_visible("file_write")
            .dynamic_visible(
                "docker_build",
                vec!["docker".to_string(), "container".to_string()],
            )
            .dynamic_visible(
                "kubernetes",
                vec!["k8s".to_string(), "kubernetes".to_string()],
            );

        assert!(config.is_always_visible("file_read"));
        assert!(config.is_always_visible("file_write"));
        assert!(!config.is_always_visible("docker_build"));
        assert!(config.is_dynamic("docker_build"));
        assert!(config.is_dynamic("kubernetes"));
    }

    #[test]
    fn test_keyword_matching() {
        let config = ToolFilterConfig::new().dynamic_visible(
            "docker_build",
            vec!["docker".to_string(), "container".to_string()],
        );

        assert!(config.matches_keywords("docker_build", "I want to build a docker image"));
        assert!(config.matches_keywords("docker_build", "Use container for deployment"));
        assert!(!config.matches_keywords("docker_build", "Just a normal message"));
    }

    #[test]
    fn test_tool_filter() {
        let config = ToolFilterConfig::new()
            .always_visible("file_read")
            .dynamic_visible("docker_build", vec!["docker".to_string()])
            .with_default_visibility(ToolVisibility::Always)
            .enable();

        let filter = ToolFilter::new(config);

        let tools = vec![
            create_test_tool("file_read"),
            create_test_tool("docker_build"),
            create_test_tool("shell"),
        ];

        // 不匹配关键词时，docker_build 不应包含
        let filtered = filter.filter_tools(tools.clone(), "Just a normal message");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|t| t.name == "file_read"));
        assert!(filtered.iter().any(|t| t.name == "shell"));

        // 匹配关键词时，docker_build 应包含
        let filtered = filter.filter_tools(tools, "I need docker for this");
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_tool_group() {
        let group = ToolGroup::new("filesystem", "文件系统工具")
            .add_tool("file_read")
            .add_tool("file_write")
            .add_tool("file_list");

        assert!(group.contains("file_read"));
        assert!(group.contains("file_write"));
        assert!(!group.contains("shell"));
    }

    #[test]
    fn test_tool_group_manager() {
        let manager = ToolGroupManager::new()
            .add_group(
                ToolGroup::new("core", "核心工具")
                    .add_tool("file_read")
                    .add_tool("file_write"),
            )
            .add_group(
                ToolGroup::new("devops", "DevOps 工具")
                    .add_tool("docker_build")
                    .add_tool("kubernetes")
                    .with_dynamic_visibility(vec!["deploy".to_string(), "ops".to_string()]),
            );

        let config = manager.to_filter_config();
        assert!(config.is_always_visible("file_read"));
        assert!(config.is_dynamic("docker_build"));
        assert!(config.matches_keywords("docker_build", "deploy to production"));
    }

    #[test]
    fn test_filter_stats() {
        let config = ToolFilterConfig::new()
            .always_visible("file_read")
            .dynamic_visible("docker_build", vec!["docker".to_string()])
            .enable();

        let filter = ToolFilter::new(config);

        let tools = vec![
            create_test_tool("file_read"),
            create_test_tool("docker_build"),
            create_test_tool("shell"),
        ];

        let stats = filter.get_stats(&tools);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.always_visible, 1);
        assert_eq!(stats.dynamic, 1);
        assert_eq!(stats.hidden, 1);
        assert_eq!(stats.visible_count(), 2);
    }
}
