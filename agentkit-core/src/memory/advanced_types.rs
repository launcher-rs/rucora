//! Memory（记忆）高级类型定义
//!
//! 本模块提供增强的 Memory 类型，支持：
//! - 命名空间隔离
//! - 重要性评分
//! - GDPR 数据导出
//! - 程序记忆存储

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 记忆条目（增强版）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedMemoryEntry {
    /// 唯一标识符
    pub id: String,
    /// 记忆内容
    pub content: String,
    /// 命名空间（用于隔离不同 Agent/用户的记忆）
    pub namespace: String,
    /// 记忆类别（如 "user_preference", "procedural", "fact" 等）
    pub category: String,
    /// 重要性评分（0.0 - 1.0，越高越重要）
    pub importance: Option<f64>,
    /// 被更新条目替代标记（用于版本追踪）
    pub superseded_by: Option<String>,
    /// 会话 ID（可选，用于会话级记忆）
    pub session_id: Option<String>,
    /// 创建时间戳（Unix 秒）
    pub created_at: u64,
    /// 更新时间戳（Unix 秒）
    pub updated_at: u64,
    /// 可选元数据
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl AdvancedMemoryEntry {
    /// 创建新的记忆条目
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self {
            id: id.into(),
            content: content.into(),
            namespace: "default".to_string(),
            category: "general".to_string(),
            importance: None,
            superseded_by: None,
            session_id: None,
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    /// 设置命名空间
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// 设置类别
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// 设置重要性评分
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = Some(importance.clamp(0.0, 1.0));
        self
    }

    /// 设置会话 ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 标记为被替代
    pub fn mark_superseded(&mut self, new_id: impl Into<String>) {
        self.superseded_by = Some(new_id.into());
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
    }

    /// 检查是否被替代
    pub fn is_superseded(&self) -> bool {
        self.superseded_by.is_some()
    }
}

/// 增强记忆查询
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdvancedMemoryQuery {
    /// 查询文本
    pub text: String,
    /// 命名空间过滤
    pub namespace: Option<String>,
    /// 类别过滤
    pub category: Option<String>,
    /// 会话 ID 过滤
    pub session_id: Option<String>,
    /// 结果数量限制
    pub limit: usize,
    /// 最小重要性过滤
    pub min_importance: Option<f64>,
    /// 时间范围过滤（起始时间，Unix 秒）
    pub since: Option<u64>,
    /// 时间范围过滤（结束时间，Unix 秒）
    pub until: Option<u64>,
    /// 是否包含已被替代的记忆
    pub include_superseded: bool,
}

impl AdvancedMemoryQuery {
    /// 创建新的查询
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// 设置命名空间过滤
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// 设置类别过滤
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// 设置会话 ID 过滤
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置结果限制
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// 设置最小重要性
    pub fn with_min_importance(mut self, importance: f64) -> Self {
        self.min_importance = Some(importance);
        self
    }

    /// 设置时间范围
    pub fn with_time_range(mut self, since: u64, until: u64) -> Self {
        self.since = Some(since);
        self.until = Some(until);
        self
    }

    /// 包含已被替代的记忆
    pub fn include_superseded(mut self) -> Self {
        self.include_superseded = true;
        self
    }
}

/// GDPR 数据导出过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportFilter {
    /// 命名空间过滤
    pub namespace: Option<String>,
    /// 会话 ID 过滤
    pub session_id: Option<String>,
    /// 时间范围起始
    pub since: Option<u64>,
    /// 时间范围结束
    pub until: Option<u64>,
    /// 类别过滤
    pub category: Option<String>,
}

impl ExportFilter {
    /// 创建新的导出过滤器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置命名空间
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// 设置会话 ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置时间范围
    pub fn with_time_range(mut self, since: u64, until: u64) -> Self {
        self.since = Some(since);
        self.until = Some(until);
        self
    }

    /// 设置类别
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// 程序记忆（从对话中提取的 how-to 知识）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProceduralMemory {
    /// 程序名称
    pub name: String,
    /// 程序描述
    pub description: String,
    /// 步骤列表
    pub steps: Vec<String>,
    /// 适用场景
    pub applicable_scenarios: Vec<String>,
    /// 成功次数
    pub success_count: u32,
    /// 失败次数
    pub failure_count: u32,
    /// 创建时间戳
    pub created_at: u64,
    /// 最后使用时间戳
    pub last_used_at: u64,
}

impl ProceduralMemory {
    /// 创建新的程序记忆
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self {
            name: name.into(),
            description: description.into(),
            steps: Vec::new(),
            applicable_scenarios: Vec::new(),
            success_count: 0,
            failure_count: 0,
            created_at: now,
            last_used_at: now,
        }
    }

    /// 添加步骤
    pub fn add_step(mut self, step: impl Into<String>) -> Self {
        self.steps.push(step.into());
        self
    }

    /// 添加适用场景
    pub fn add_scenario(mut self, scenario: impl Into<String>) -> Self {
        self.applicable_scenarios.push(scenario.into());
        self
    }

    /// 记录成功使用
    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
    }

    /// 记录失败使用
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

/// 记忆统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// 总条目数
    pub total_entries: usize,
    /// 各命名空间条目数
    pub entries_by_namespace: HashMap<String, usize>,
    /// 各类别条目数
    pub entries_by_category: HashMap<String, usize>,
    /// 平均重要性
    pub avg_importance: f64,
    /// 被替代条目数
    pub superseded_count: usize,
}

/// 记忆衰减配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    /// 是否启用衰减
    pub enabled: bool,
    /// 衰减半衰期（秒）
    pub half_life_seconds: u64,
    /// 最小重要性阈值（低于此值会被清理）
    pub min_importance_threshold: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            half_life_seconds: 30 * 24 * 60 * 60, // 30 天
            min_importance_threshold: 0.1,
        }
    }
}

/// 计算衰减后的重要性
pub fn calculate_decayed_importance(
    original_importance: f64,
    created_at: u64,
    config: &DecayConfig,
) -> f64 {
    if !config.enabled || original_importance <= 0.0 {
        return original_importance;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(created_at, |d| d.as_secs());

    let age_seconds = now.saturating_sub(created_at);
    let half_lives = age_seconds as f64 / config.half_life_seconds as f64;

    // 指数衰减公式：I = I0 * (0.5 ^ (t / t_half))
    let decayed = original_importance * 0.5f64.powf(half_lives);

    decayed.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_memory_entry_builder() {
        let entry = AdvancedMemoryEntry::new("test-1", "测试内容")
            .with_namespace("user-123")
            .with_category("preference")
            .with_importance(0.8)
            .with_session_id("session-456");

        assert_eq!(entry.id, "test-1");
        assert_eq!(entry.content, "测试内容");
        assert_eq!(entry.namespace, "user-123");
        assert_eq!(entry.category, "preference");
        assert_eq!(entry.importance, Some(0.8));
        assert_eq!(entry.session_id, Some("session-456".to_string()));
    }

    #[test]
    fn test_importance_clamping() {
        let entry = AdvancedMemoryEntry::new("test", "content").with_importance(1.5);
        assert_eq!(entry.importance, Some(1.0));

        let entry2 = AdvancedMemoryEntry::new("test", "content").with_importance(-0.5);
        assert_eq!(entry2.importance, Some(0.0));
    }

    #[test]
    fn test_memory_query_builder() {
        let query = AdvancedMemoryQuery::new("搜索内容")
            .with_namespace("user-123")
            .with_category("fact")
            .with_limit(10)
            .with_min_importance(0.5);

        assert_eq!(query.text, "搜索内容");
        assert_eq!(query.namespace, Some("user-123".to_string()));
        assert_eq!(query.category, Some("fact".to_string()));
        assert_eq!(query.limit, 10);
        assert_eq!(query.min_importance, Some(0.5));
    }

    #[test]
    fn test_procedural_memory() {
        let mut proc = ProceduralMemory::new("deploy", "部署流程")
            .add_step("构建镜像")
            .add_step("推送仓库")
            .add_step("更新服务")
            .add_scenario("发布新版本");

        assert_eq!(proc.steps.len(), 3);
        assert_eq!(proc.applicable_scenarios.len(), 1);

        proc.record_success();
        proc.record_success();
        proc.record_failure();

        assert_eq!(proc.success_count, 2);
        assert_eq!(proc.failure_count, 1);
        assert!((proc.success_rate() - 0.6667).abs() < 0.001);
    }

    #[test]
    fn test_decay_calculation() {
        let config = DecayConfig {
            enabled: true,
            half_life_seconds: 86400, // 1 天
            min_importance_threshold: 0.1,
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        // 刚刚创建的记忆不应衰减
        let importance = calculate_decayed_importance(1.0, now, &config);
        assert!((importance - 1.0).abs() < 0.01);

        // 禁用衰减时不应衰减
        let disabled_config = DecayConfig {
            enabled: false,
            ..config
        };
        let importance = calculate_decayed_importance(1.0, now - 86400, &disabled_config);
        assert!((importance - 1.0).abs() < 0.01);
    }
}
