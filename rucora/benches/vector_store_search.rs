use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rucora_core::retrieval::{VectorQuery, VectorRecord, VectorStore};
use rucora_retrieval::in_memory::InMemoryVectorStore;

fn bench_inmemory_vector_store_search(c: &mut Criterion) {
    let store = InMemoryVectorStore::new();

    // 用一个专用 runtime 来跑 async bench。
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let mut records = Vec::new();
        for i in 0..2000 {
            let v = vec![
                (i as f32).sin(),
                (i as f32).cos(),
                (i as f32 * 0.01).sin(),
                (i as f32 * 0.01).cos(),
            ];
            records.push(VectorRecord::new(format!("id-{i}"), v));
        }
        store.upsert(records).await.unwrap();
    });

    c.bench_function("inmemory_vector_store/search top_k=10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let q = VectorQuery::new(vec![0.1, 0.2, 0.3, 0.4]).with_top_k(10);
                let r = store.search(black_box(q)).await.unwrap();
                black_box(r)
            })
        })
    });
}

criterion_group!(benches, bench_inmemory_vector_store_search);
criterion_main!(benches);
