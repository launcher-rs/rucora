use agentkit_core::retrieval::{VectorQuery, VectorRecord, VectorStore};
use agentkit_retrieval::in_memory::InMemoryVectorStore;
use serde_json::json;

// 说明：VectorStore 的一个最小 contract 测试。
// 这里使用 agentkit crate 中的 InMemoryVectorStore 作为被测实现。

#[tokio::test]
async fn vector_store_contract_upsert_get_count_clear_should_work() {
    let store = InMemoryVectorStore::new();

    store
        .upsert(vec![VectorRecord::new("a", vec![1.0, 0.0]).with_text("A")])
        .await
        .unwrap();

    let n = store.count().await.unwrap();
    assert_eq!(n, 1);

    let got = store.get(vec!["a".to_string()]).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, "a");

    store.clear().await.unwrap();
    let n2 = store.count().await.unwrap();
    assert_eq!(n2, 0);
}

#[tokio::test]
async fn vector_store_contract_search_should_return_sorted_results() {
    let store = InMemoryVectorStore::new();

    store
        .upsert(vec![
            VectorRecord::new("a", vec![1.0, 0.0]).with_metadata(json!({"k": 1})),
            VectorRecord::new("b", vec![0.9, 0.1]).with_metadata(json!({"k": 2})),
            VectorRecord::new("c", vec![0.0, 1.0]).with_metadata(json!({"k": 3})),
        ])
        .await
        .unwrap();

    let results = store
        .search(VectorQuery::new(vec![1.0, 0.0]).with_top_k(3))
        .await
        .unwrap();

    assert!(!results.is_empty());
    // 约定：结果应按 score 降序排序。
    for w in results.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
}
