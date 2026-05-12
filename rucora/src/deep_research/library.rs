//! 研究库实现

use async_trait::async_trait;
use rucora_core::research::{ResearchError, ResearchReport, ResearchLibrary};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// 基于内存的研究库
pub struct InMemoryResearchLibrary {
    reports: std::sync::RwLock<HashMap<String, ResearchReport>>,
}

impl InMemoryResearchLibrary {
    pub fn new() -> Self {
        Self {
            reports: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryResearchLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResearchLibrary for InMemoryResearchLibrary {
    async fn save(&self, report: &ResearchReport) -> Result<String, ResearchError> {
        let id = report.id.clone();
        let mut reports = self.reports.write().unwrap();
        reports.insert(id.clone(), report.clone());
        Ok(id)
    }

    async fn search(&self, query: &str) -> Result<Vec<ResearchReport>, ResearchError> {
        let reports = self.reports.read().unwrap();
        let query_lower = query.to_lowercase();
        let mut results: Vec<ResearchReport> = reports
            .values()
            .filter(|r| {
                r.topic.to_lowercase().contains(&query_lower)
                    || r.summary.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();
        results.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<ResearchReport>, ResearchError> {
        let reports = self.reports.read().unwrap();
        Ok(reports.get(id).cloned())
    }

    async fn list(&self, limit: usize) -> Result<Vec<ResearchReport>, ResearchError> {
        let reports = self.reports.read().unwrap();
        let mut list: Vec<ResearchReport> = reports.values().cloned().collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        list.truncate(limit);
        Ok(list)
    }

    async fn delete(&self, id: &str) -> Result<(), ResearchError> {
        let mut reports = self.reports.write().unwrap();
        reports.remove(id);
        Ok(())
    }
}

/// 基于文件系统的研究库
pub struct FileResearchLibrary {
    base_path: PathBuf,
}

impl FileResearchLibrary {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub async fn init(&self) -> Result<(), ResearchError> {
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| ResearchError::Storage(e.to_string()))
    }

    fn report_path(&self, id: &str) -> PathBuf {
        self.base_path.join(format!("{id}.json"))
    }
}

#[async_trait]
impl ResearchLibrary for FileResearchLibrary {
    async fn save(&self, report: &ResearchReport) -> Result<String, ResearchError> {
        let path = self.report_path(&report.id);
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| ResearchError::Storage(e.to_string()))?;
        fs::write(&path, json)
            .await
            .map_err(|e| ResearchError::Storage(e.to_string()))?;
        Ok(report.id.clone())
    }

    async fn search(&self, query: &str) -> Result<Vec<ResearchReport>, ResearchError> {
        let mut results = Vec::new();
        let mut entries = fs::read_dir(&self.base_path)
            .await
            .map_err(|e| ResearchError::Storage(e.to_string()))?;

        let query_lower = query.to_lowercase();
        while let Some(entry) = entries.next_entry().await.map_err(|e| ResearchError::Storage(e.to_string()))? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Ok(content) = fs::read_to_string(&path).await
                    && let Ok(report) = serde_json::from_str::<ResearchReport>(&content)
                        && (report.topic.to_lowercase().contains(&query_lower)
                            || report.summary.to_lowercase().contains(&query_lower))
                        {
                            results.push(report);
                        }
        }

        results.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<ResearchReport>, ResearchError> {
        let path = self.report_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ResearchError::Storage(e.to_string()))?;
        let report = serde_json::from_str(&content)
            .map_err(|e| ResearchError::Storage(e.to_string()))?;
        Ok(Some(report))
    }

    async fn list(&self, limit: usize) -> Result<Vec<ResearchReport>, ResearchError> {
        let mut results = Vec::new();
        let mut entries = fs::read_dir(&self.base_path)
            .await
            .map_err(|e| ResearchError::Storage(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| ResearchError::Storage(e.to_string()))? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Ok(content) = fs::read_to_string(&path).await
                    && let Ok(report) = serde_json::from_str::<ResearchReport>(&content) {
                        results.push(report);
                    }
        }

        results.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        results.truncate(limit);
        Ok(results)
    }

    async fn delete(&self, id: &str) -> Result<(), ResearchError> {
        let path = self.report_path(id);
        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| ResearchError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}