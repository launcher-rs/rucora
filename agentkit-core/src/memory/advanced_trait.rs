//! Memory（记忆）高级 trait 定义
//!
//! 本模块提供增强的 Memory trait，支持命名空间、重要性评分、GDPR 导出等功能。

use async_trait::async_trait;

use crate::error::MemoryError;

use super::advanced_types::{
    AdvancedMemoryEntry, AdvancedMemoryQuery, DecayConfig, ExportFilter, MemoryStats,
    ProceduralMemory,
};

/// 增强版 Memory trait
///
/// 该 trait 扩展了基础 Memory trait，提供：
/// - 命名空间隔离
/// - 重要性评分
/// - GDPR 数据导出
/// - 程序记忆存储
/// - 记忆衰减
#[async_trait]
pub trait AdvancedMemory: Send + Sync {
    /// 存储记忆条目
    async fn store(&self, entry: AdvancedMemoryEntry) -> Result<(), MemoryError>;

    /// 检索记忆
    async fn recall(
        &self,
        query: &AdvancedMemoryQuery,
    ) -> Result<Vec<AdvancedMemoryEntry>, MemoryError>;

    /// 按命名空间检索记忆
    async fn recall_namespaced(
        &self,
        namespace: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<AdvancedMemoryEntry>, MemoryError> {
        let query = AdvancedMemoryQuery::new(query)
            .with_namespace(namespace)
            .with_limit(limit);
        self.recall(&query).await
    }

    /// 删除指定 ID 的记忆
    async fn delete(&self, id: &str) -> Result<bool, MemoryError>;

    /// 批量清除命名空间下的所有记忆
    async fn purge_namespace(&self, namespace: &str) -> Result<usize, MemoryError>;

    /// 批量清除会话相关的记忆
    async fn purge_session(&self, session_id: &str) -> Result<usize, MemoryError>;

    /// 存储程序记忆（从对话中提取的 how-to 知识）
    async fn store_procedural(
        &self,
        memory: ProceduralMemory,
        session_id: Option<String>,
    ) -> Result<(), MemoryError>;

    /// 检索程序记忆
    async fn recall_procedural(
        &self,
        scenario: &str,
        limit: usize,
    ) -> Result<Vec<ProceduralMemory>, MemoryError>;

    /// GDPR 数据导出
    async fn export(&self, filter: &ExportFilter) -> Result<Vec<AdvancedMemoryEntry>, MemoryError>;

    /// 获取记忆统计信息
    async fn stats(&self) -> Result<MemoryStats, MemoryError>;

    /// 执行记忆衰减（清理过期/低重要性记忆）
    async fn apply_decay(&self, config: &DecayConfig) -> Result<usize, MemoryError>;

    /// 更新记忆重要性
    async fn update_importance(&self, id: &str, importance: f64) -> Result<(), MemoryError>;

    /// 标记记忆被替代
    async fn mark_superseded(&self, old_id: &str, new_id: &str) -> Result<(), MemoryError>;
}

/// 支持记忆整合的 Memory trait
///
/// 提供记忆整合（consolidation）功能，将短期记忆转为长期记忆
#[async_trait]
pub trait ConsolidatableMemory: AdvancedMemory {
    /// 运行记忆整合
    ///
    /// 将多个相关记忆合并为一个更抽象的记忆
    async fn consolidate(&self, namespace: &str) -> Result<Vec<String>, MemoryError>;

    /// 查找相似记忆
    async fn find_similar(
        &self,
        content: &str,
        threshold: f64,
        limit: usize,
    ) -> Result<Vec<AdvancedMemoryEntry>, MemoryError>;
}

/// Memory 工厂 trait
///
/// 用于创建不同后端的 Memory 实例
#[async_trait]
pub trait MemoryFactory: Send + Sync {
    /// 创建 Memory 实例
    async fn create(
        &self,
        config: &MemoryBackendConfig,
    ) -> Result<Box<dyn AdvancedMemory>, MemoryError>;
}

/// Memory 后端配置
#[derive(Debug, Clone)]
pub enum MemoryBackendConfig {
    /// 内存存储
    InMemory {
        /// 最大容量
        max_capacity: usize,
    },
    /// 文件存储
    File {
        /// 文件路径
        path: std::path::PathBuf,
    },
    /// SQLite 存储
    Sqlite {
        /// 数据库路径
        path: std::path::PathBuf,
    },
    /// Qdrant 向量数据库
    Qdrant {
        /// 服务地址
        url: String,
        /// 集合名称
        collection: String,
        /// API 密钥（可选）
        api_key: Option<String>,
    },
}

/// 记忆重要性评分器
pub trait ImportanceScorer: Send + Sync {
    /// 计算内容的重要性评分
    fn score(&self, content: &str, context: Option<&str>) -> f64;
}

/// 默认重要性评分器（基于简单启发式）
pub struct DefaultImportanceScorer;

impl ImportanceScorer for DefaultImportanceScorer {
    fn score(&self, content: &str, _context: Option<&str>) -> f64 {
        let mut score: f64 = 0.5; // 基础分

        // 长度因子：适中长度的内容更重要
        let len = content.len();
        if len > 50 && len < 1000 {
            score += 0.1;
        }

        // 关键词因子
        let important_keywords = [
            "重要",
            "关键",
            "必须",
            "务必",
            "critical",
            "important",
            "must",
            "key",
        ];
        let content_lower = content.to_lowercase();
        for keyword in &important_keywords {
            if content_lower.contains(keyword) {
                score += 0.05;
            }
        }

        // 代码块因子
        if content.contains("```") || content.contains("`") {
            score += 0.05;
        }

        score.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_importance_scorer() {
        let scorer = DefaultImportanceScorer;

        // 普通内容
        let score1 = scorer.score("这是一段普通的内容", None);
        assert!((0.5..=0.7).contains(&score1));

        // 包含关键词的内容
        let score2 = scorer.score("这是一个重要的关键信息", None);
        assert!(score2 > score1);

        // 包含代码的内容
        let score3 = scorer.score("使用 `code` 函数", None);
        assert!(score3 > score1);
    }

    #[test]
    fn test_memory_backend_config() {
        let config = MemoryBackendConfig::InMemory { max_capacity: 1000 };
        match config {
            MemoryBackendConfig::InMemory { max_capacity } => {
                assert_eq!(max_capacity, 1000);
            }
            _ => panic!("错误的配置类型"),
        }
    }
}
