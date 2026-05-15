use async_trait::async_trait;
use rucora_core::error::MemoryError;
use rucora_core::memory::{Memory, MemoryItem, MemoryQuery};

struct TestMemory {
    items: std::sync::Arc<std::sync::Mutex<Vec<MemoryItem>>>,
}

impl TestMemory {
    fn new() -> Self {
        Self {
            items: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Memory for TestMemory {
    async fn add(&self, item: MemoryItem) -> Result<(), MemoryError> {
        self.items.lock().unwrap().push(item);
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>, MemoryError> {
        let items = self.items.lock().unwrap();
        let results: Vec<MemoryItem> = items
            .iter()
            .filter(|item| item.content.contains(&query.text))
            .take(query.limit)
            .cloned()
            .collect();
        Ok(results)
    }
}

#[tokio::test]
async fn memory_contract_should_add_and_query() {
    let memory = TestMemory::new();

    // 添加记忆
    memory
        .add(MemoryItem {
            id: "1".to_string(),
            content: "Rust 是一门系统编程语言".to_string(),
            metadata: None,
        })
        .await
        .unwrap();

    memory
        .add(MemoryItem {
            id: "2".to_string(),
            content: "Python 适合数据科学".to_string(),
            metadata: None,
        })
        .await
        .unwrap();

    // 查询记忆
    let results = memory
        .query(MemoryQuery {
            text: "Rust".to_string(),
            limit: 10,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");
}

#[tokio::test]
async fn memory_contract_should_return_empty_on_no_match() {
    let memory = TestMemory::new();

    let results = memory
        .query(MemoryQuery {
            text: "不存在的关键词".to_string(),
            limit: 10,
        })
        .await
        .unwrap();

    assert!(results.is_empty());
}

#[tokio::test]
async fn memory_contract_should_respect_limit() {
    let memory = TestMemory::new();

    for i in 0..5 {
        memory
            .add(MemoryItem {
                id: i.to_string(),
                content: format!("测试内容 {i}"),
                metadata: None,
            })
            .await
            .unwrap();
    }

    let results = memory
        .query(MemoryQuery {
            text: "测试".to_string(),
            limit: 2,
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
}
