use criterion::{criterion_group, criterion_main, Criterion, black_box};
use corvus_memory::vector::KnnIndex;
use rand::Rng;
use std::collections::HashMap;

fn generate_random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn bench_vector_search(c: &mut Criterion) {
    let dim = 128;
    let num_vectors = 1000;
    let num_queries = 100;

    // Create index and add vectors
    let mut index = KnnIndex::new();
    for i in 0..num_vectors {
        let vec = generate_random_vector(dim);
        index.add(format!("vec_{}", i), vec, HashMap::new());
    }

    // Generate query vectors
    let queries: Vec<Vec<f32>> = (0..num_queries)
        .map(|_| generate_random_vector(dim))
        .collect();

    // Benchmark search
    c.bench_function("vector_search_k10", |b| {
        b.iter(|| {
            for query in &queries {
                black_box(index.search(black_box(query), 10));
            }
        });
    });

    c.bench_function("vector_search_k100", |b| {
        b.iter(|| {
            for query in &queries {
                black_box(index.search(black_box(query), 100));
            }
        });
    });

    // Benchmark build
    c.bench_function("vector_index_build", |b| {
        let mut index = KnnIndex::new();
        for i in 0..num_vectors {
            let vec = generate_random_vector(dim);
            index.add(format!("vec_{}", i), vec, HashMap::new());
        }
        b.iter(|| {
            black_box(&mut index).build();
        });
    });
}

fn bench_add_vectors(c: &mut Criterion) {
    let dim = 128;
    let num_vectors = 100;

    c.bench_function("add_100_vectors", |b| {
        b.iter(|| {
            let mut index = KnnIndex::new();
            for i in 0..num_vectors {
                let vec = generate_random_vector(dim);
                black_box(&mut index).add(
                    black_box(format!("vec_{}", i)),
                    black_box(vec),
                    black_box(HashMap::new()),
                );
            }
        });
    });
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let dim = 128;
    let vec1 = generate_random_vector(dim);
    let vec2 = generate_random_vector(dim);

    c.bench_function("cosine_similarity", |b| {
        b.iter(|| {
            black_box(KnnIndex::cosine_similarity(
                black_box(&vec1),
                black_box(&vec2),
            ));
        });
    });
}

criterion_group!(
    benches,
    bench_vector_search,
    bench_add_vectors,
    bench_cosine_similarity
);
criterion_main!(benches);
