//! Skill 缓存模块
//!
//! 缓存已加载的 Skills，避免重复加载
//! 参考 zeroclaw 的设计

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};
use crate::skills::SkillDefinition;

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    skill: SkillDefinition,
    loaded_at: SystemTime,
    expires_at: Option<SystemTime>,
}

impl CacheEntry {
    fn new(skill: SkillDefinition, ttl: Option<Duration>) -> Self {
        let now = SystemTime::now();
        let expires_at = ttl.map(|d| now + d);
        
        Self {
            skill,
            loaded_at: now,
            expires_at,
        }
    }
    
    fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| SystemTime::now() > exp).unwrap_or(false)
    }
}

/// Skill 缓存
pub struct SkillCache {
    entries: HashMap<String, CacheEntry>,
    default_ttl: Option<Duration>,
    max_size: usize,
}

impl SkillCache {
    /// 创建新的缓存
    pub fn new(max_size: usize, default_ttl: Option<Duration>) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
            max_size,
        }
    }
    
    /// 创建默认缓存（无 TTL，最大 100 个）
    pub fn default_cache() -> Self {
        Self::new(100, None)
    }
    
    /// 获取缓存的 Skill
    pub fn get(&self, key: &str) -> Option<&SkillDefinition> {
        self.entries.get(key)
            .filter(|e| !e.is_expired())
            .map(|e| &e.skill)
    }
    
    /// 缓存 Skill
    pub fn insert(&mut self, key: String, skill: SkillDefinition) {
        // 如果缓存已满，移除最旧的条目
        if self.entries.len() >= self.max_size {
            self.remove_oldest();
        }
        
        let entry = CacheEntry::new(skill, self.default_ttl);
        self.entries.insert(key, entry);
    }
    
    /// 移除过期的条目
    pub fn cleanup(&mut self) -> usize {
        let expired_keys: Vec<String> = self.entries
            .iter()
            .filter(|(_, e)| e.is_expired())
            .map(|(k, _)| k.clone())
            .collect();
        
        let count = expired_keys.len();
        for key in expired_keys {
            self.entries.remove(&key);
        }
        count
    }
    
    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// 移除最旧的条目
    fn remove_oldest(&mut self) {
        if let Some(oldest_key) = self.entries
            .iter()
            .min_by_key(|(_, e)| e.loaded_at)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }
    }
}

/// 带缓存的 Skill 加载器包装器
pub struct CachedSkillLoader {
    cache: SkillCache,
    loader: crate::skills::SkillLoader,
}

impl CachedSkillLoader {
    /// 创建新的缓存加载器
    pub fn new(skills_dir: &Path, cache: SkillCache) -> Self {
        Self {
            cache,
            loader: crate::skills::SkillLoader::new(skills_dir),
        }
    }
    
    /// 获取 Skill（优先从缓存读取）
    pub async fn get_skill(&mut self, name: &str) -> Option<crate::skills::SkillDefinition> {
        // 尝试从缓存读取
        if let Some(skill) = self.cache.get(name) {
            return Some(skill.clone());
        }
        
        // 从文件加载（简化实现，实际应该调用 loader）
        None
    }
    
    /// 缓存 Skill
    pub fn cache_skill(&mut self, skill: crate::skills::SkillDefinition) {
        self.cache.insert(skill.name.clone(), skill);
    }
    
    /// 获取缓存统计
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.max_size)
    }
    
    /// 获取底层 loader
    pub fn loader(&mut self) -> &mut crate::skills::SkillLoader {
        &mut self.loader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_basic() {
        let mut cache = SkillCache::default_cache();
        
        let skill = SkillDefinition::new("test", "Test skill");
        cache.insert("test".to_string(), skill.clone());
        
        assert_eq!(cache.len(), 1);
        assert!(cache.get("test").is_some());
        assert!(cache.get("nonexistent").is_none());
    }
    
    #[test]
    fn test_cache_ttl() {
        let mut cache = SkillCache::new(100, Some(Duration::from_millis(100)));
        
        let skill = SkillDefinition::new("test", "Test skill");
        cache.insert("test".to_string(), skill.clone());
        
        // 立即检查，应该存在
        assert!(cache.get("test").is_some());
        
        // 等待过期
        std::thread::sleep(Duration::from_millis(150));
        
        // 过期后应该不存在
        assert!(cache.get("test").is_none());
    }
    
    #[test]
    fn test_cache_max_size() {
        let mut cache = SkillCache::new(3, None);
        
        for i in 0..5 {
            let skill = SkillDefinition::new(&format!("test{}", i), "Test");
            cache.insert(format!("test{}", i), skill);
        }
        
        // 缓存应该只保留最新的 3 个
        assert_eq!(cache.len(), 3);
        assert!(cache.get("test0").is_none());
        assert!(cache.get("test1").is_none());
        assert!(cache.get("test2").is_some());
        assert!(cache.get("test3").is_some());
        assert!(cache.get("test4").is_some());
    }
    
    #[test]
    fn test_cleanup() {
        let mut cache = SkillCache::new(100, Some(Duration::from_millis(50)));
        
        // 添加一些会过期的条目
        for i in 0..3 {
            let skill = SkillDefinition::new(&format!("test{}", i), "Test");
            cache.insert(format!("test{}", i), skill);
        }
        
        // 等待过期
        std::thread::sleep(Duration::from_millis(100));
        
        // 清理过期条目
        let removed = cache.cleanup();
        assert_eq!(removed, 3);
        assert_eq!(cache.len(), 0);
    }
}
