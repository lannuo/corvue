# Corvus Performance & Benchmarks

This document describes performance characteristics and benchmarking approach for Corvus.

## Current Performance

Corvus is designed for high performance as a pure Rust implementation. Key components:

| Component | Type | Status |
|-----------|------|--------|
| InMemoryMemory | O(1) inserts, O(n) queries | ✓ Implemented |
| TagMemoWave | O(n + m) wave propagation | ✓ Implemented |
| EPA Analysis | O(d) where d = embedding dimension | ✓ Implemented |
| Residual Pyramid | O(d * k) where k = levels | ✓ Implemented |
| SQLite Storage | O(log n) indexed queries | ✓ Implemented |

## Adding Benchmarks

To add benchmarks, use the `criterion` crate:

1. Add to crate's Cargo.toml:
```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "my_benchmark"
harness = false
```

2. Create a benchmark file in `benches/`:
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_function(c: &mut Criterion) {
    c.bench_function("function_name", |b| {
        b.iter(|| {
            // Your benchmark code here
        });
    });
}

criterion_group!(benches, bench_function);
criterion_main!(benches);
```

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --package <crate-name>
```

## Optimization Notes

- **TagMemo Wave**: Wave propagation performance depends on network size and activation threshold
- **SQLite Storage**: Use WAL mode and appropriate indexes for best performance
- **Memory Usage**: TagMemo maintains in-memory graph - use persistent storage for large datasets

## Future Optimizations

- [ ] Vector index integration (USearch, FAISS)
- [ ] Parallel wave propagation
- [ ] Caching for frequent queries
- [ ] Batch operations
