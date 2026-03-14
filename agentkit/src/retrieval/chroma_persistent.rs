//! Chroma 本地嵌入式向量存储实现（持久化版）。
//!
//! 基于本地文件存储（JSON 格式），无需 HTTP 服务器，
//! 数据持久化到磁盘，重启后数据不丢失。
//!
//! 存储路径可通过 CHROMA_PERSIST_DIR 环境变量指定（默认 ./chroma_db）

use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::RwLock,
};

use agentkit_core::{
    error::ProviderError,
    retrieval::{SearchResult, VectorQuery, VectorRecord, VectorStore},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Chroma 本地嵌入式向量存储（持久化到文件）。
pub struct ChromaPersistentStore {
    persist_dir: PathBuf,
    collection: String,
    cache: RwLock<HashMap<String, PersistedRecord>>,
}

/// 持久化记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRecord {
    id: String,
    vector: Vec<f32>,
    metadata: Option<serde_json::Value>,
}

impl ChromaPersistentStore {
    /// 从环境变量创建 Store。
    ///
    /// 环境变量：
    /// - CHROMA_PERSIST_DIR: 持久化目录（默认 ./chroma_db）
    /// - CHROMA_COLLECTION: 集合名称（默认 default）
    pub fn from_env() -> Result<Self, ProviderError> {
        let persist_dir =
            env::var("CHROMA_PERSIST_DIR").unwrap_or_else(|_| "./chroma_db".to_string());
        let collection = env::var("CHROMA_COLLECTION").unwrap_or_else(|_| "default".to_string());

        Self::new(persist_dir, collection)
    }

    /// 创建 Store。
    pub fn new(
        persist_dir: impl AsRef<Path>,
        collection: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        let persist_dir = persist_dir.as_ref().to_path_buf();
        let collection = collection.into();

        // 确保目录存在
        let collection_dir = persist_dir.join(&collection);
        fs::create_dir_all(&collection_dir)
            .map_err(|e| ProviderError::Message(format!("无法创建目录: {}", e)))?;

        // 加载已有数据
        let cache = Self::load_from_disk(&collection_dir)?;

        Ok(Self {
            persist_dir,
            collection,
            cache: RwLock::new(cache),
        })
    }

    /// 获取集合目录路径。
    fn collection_dir(&self) -> PathBuf {
        self.persist_dir.join(&self.collection)
    }

    /// 获取数据文件路径。
    fn data_file(&self) -> PathBuf {
        self.collection_dir().join("data.json")
    }

    /// 从磁盘加载数据。
    fn load_from_disk(
        collection_dir: &Path,
    ) -> Result<HashMap<String, PersistedRecord>, ProviderError> {
        let data_file = collection_dir.join("data.json");
        if !data_file.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&data_file)
            .map_err(|e| ProviderError::Message(format!("读取数据文件失败: {}", e)))?;

        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let records: Vec<PersistedRecord> = serde_json::from_str(&content)
            .map_err(|e| ProviderError::Message(format!("解析数据文件失败: {}", e)))?;

        let cache: HashMap<String, PersistedRecord> =
            records.into_iter().map(|r| (r.id.clone(), r)).collect();

        Ok(cache)
    }

    /// 保存数据到磁盘。
    fn save_to_disk(&self) -> Result<(), ProviderError> {
        let cache = self
            .cache
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;

        let records: Vec<&PersistedRecord> = cache.values().collect();
        let json = serde_json::to_string_pretty(&records)
            .map_err(|e| ProviderError::Message(format!("序列化数据失败: {}", e)))?;

        drop(cache);

        let data_file = self.data_file();
        let mut file = fs::File::create(&data_file)
            .map_err(|e| ProviderError::Message(format!("创建数据文件失败: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| ProviderError::Message(format!("写入数据文件失败: {}", e)))?;

        Ok(())
    }

    /// 设置集合名称（会切换集合并加载新集合的数据）。
    pub fn with_collection(mut self, collection: impl Into<String>) -> Result<Self, ProviderError> {
        let collection = collection.into();
        let collection_dir = self.persist_dir.join(&collection);
        fs::create_dir_all(&collection_dir)
            .map_err(|e| ProviderError::Message(format!("无法创建目录: {}", e)))?;

        let cache = Self::load_from_disk(&collection_dir)?;

        self.collection = collection;
        self.cache = RwLock::new(cache);

        Ok(self)
    }

    /// 计算余弦相似度。
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot_product / (norm_a * norm_b)
    }

    /// 检查集合是否存在。
    pub fn collection_exists(&self) -> bool {
        self.collection_dir().exists()
    }

    /// 删除集合。
    pub fn delete_collection(&self) -> Result<(), ProviderError> {
        let collection_dir = self.collection_dir();
        if collection_dir.exists() {
            fs::remove_dir_all(&collection_dir)
                .map_err(|e| ProviderError::Message(format!("删除集合失败: {}", e)))?;
        }
        Ok(())
    }
}

#[async_trait]
impl VectorStore for ChromaPersistentStore {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<(), ProviderError> {
        if records.is_empty() {
            return Ok(());
        }

        let mut cache = self
            .cache
            .write()
            .map_err(|_| ProviderError::Message("无法获取写锁".to_string()))?;

        for record in records {
            let persisted = PersistedRecord {
                id: record.id,
                vector: record.vector,
                metadata: record.metadata,
            };
            cache.insert(persisted.id.clone(), persisted);
        }

        drop(cache);

        self.save_to_disk()?;

        Ok(())
    }

    async fn delete(&self, ids: Vec<String>) -> Result<(), ProviderError> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut cache = self
            .cache
            .write()
            .map_err(|_| ProviderError::Message("无法获取写锁".to_string()))?;

        for id in ids {
            cache.remove(&id);
        }

        drop(cache);

        self.save_to_disk()?;

        Ok(())
    }

    async fn get(&self, ids: Vec<String>) -> Result<Vec<VectorRecord>, ProviderError> {
        let cache = self
            .cache
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;

        let mut results = Vec::new();
        for id in ids {
            if let Some(record) = cache.get(&id) {
                let text = record
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                results.push(VectorRecord {
                    id: record.id.clone(),
                    vector: record.vector.clone(),
                    text,
                    metadata: record.metadata.clone(),
                });
            }
        }

        Ok(results)
    }

    async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>, ProviderError> {
        let cache = self
            .cache
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;

        let query_vector = &query.vector;
        let mut scores: Vec<(String, f32)> = cache
            .values()
            .map(|record| {
                let score = Self::cosine_similarity(query_vector, &record.vector);
                (record.id.clone(), score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let threshold = query.score_threshold.unwrap_or(0.0);
        let top_k = query.top_k;

        let mut results = Vec::new();
        for (id, score) in scores.iter().take(top_k) {
            if *score < threshold {
                break;
            }

            if let Some(record) = cache.get(id) {
                let text = record
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                results.push(SearchResult {
                    id: id.clone(),
                    score: *score,
                    vector: Some(record.vector.clone()),
                    text,
                    metadata: record.metadata.clone(),
                });
            }
        }

        Ok(results)
    }

    async fn clear(&self) -> Result<(), ProviderError> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| ProviderError::Message("无法获取写锁".to_string()))?;

        cache.clear();

        drop(cache);

        self.save_to_disk()?;

        Ok(())
    }

    async fn count(&self) -> Result<usize, ProviderError> {
        let cache = self
            .cache
            .read()
            .map_err(|_| ProviderError::Message("无法获取读锁".to_string()))?;

        Ok(cache.len())
    }
}
