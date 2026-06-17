//! Criterion benchmarks for the performance-critical paths.
//!
//! Run with `cargo bench -p rayforce`. Highlights the design's headline wins:
//! single-`memcpy` column construction, zero-copy `&[T]` reads (vs. boxing each
//! element), engine-side aggregation, group-by, and serialization.
//!
//! The core is single-threaded with one live runtime per process; Criterion
//! runs benchmark routines synchronously on one thread, so we create the
//! runtime once (and leak it for the process lifetime) on that thread.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use rayforce::{col, lit, sum, Runtime, Table, Value};
use std::sync::Once;

static INIT: Once = Once::new();

fn ensure_runtime() {
    INIT.call_once(|| {
        // Leak: the runtime must outlive every benchmark on this thread.
        std::mem::forget(Runtime::new().expect("runtime"));
    });
}

const N: usize = 100_000;

fn i64_data() -> Vec<i64> {
    (0..N as i64).collect()
}

fn bench_vector(c: &mut Criterion) {
    ensure_runtime();
    let data = i64_data();

    let mut g = c.benchmark_group("vector");
    g.throughput(Throughput::Elements(N as u64));

    // Build a column from a slice — one memcpy under the hood.
    g.bench_function("construct_memcpy", |b| {
        b.iter(|| {
            let v = Value::vec(black_box(&data));
            black_box(v.len());
        })
    });

    let v = Value::vec(&data);

    // Zero-copy read: borrow the storage as &[i64] and reduce it.
    g.bench_function("read_zero_copy_slice", |b| {
        b.iter(|| {
            let s = v.as_slice::<i64>().unwrap();
            black_box(s.iter().copied().sum::<i64>())
        })
    });
    g.finish();
}

/// Boxed element read: one FFI hop + allocation per element — the cost the
/// zero-copy `as_slice` path avoids. Smaller N keeps wall-clock sane.
fn bench_boxed_read(c: &mut Criterion) {
    ensure_runtime();
    let small = Value::vec(&(0..10_000i64).collect::<Vec<_>>());
    let mut g = c.benchmark_group("vector_boxed_read_10k");
    g.throughput(Throughput::Elements(10_000));
    g.bench_function("read_boxed_get", |b| {
        b.iter(|| {
            let mut acc = 0i64;
            for i in 0..small.len() {
                acc += small.get(i).unwrap().as_i64().unwrap();
            }
            black_box(acc)
        })
    });
    g.finish();
}

fn bench_aggregation(c: &mut Criterion) {
    ensure_runtime();
    let v = Value::vec(&i64_data());

    let mut g = c.benchmark_group("aggregation");
    g.throughput(Throughput::Elements(N as u64));
    g.bench_function("engine_sum", |b| {
        b.iter(|| {
            let r = sum(lit(black_box(v.clone()))).execute().unwrap();
            black_box(r.as_i64().unwrap())
        })
    });
    g.finish();
}

fn bench_query(c: &mut Criterion) {
    ensure_runtime();

    // 100k rows over 10 symbol groups.
    let groups = ["g0", "g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8", "g9"];
    let syms: Vec<&str> = (0..N).map(|i| groups[i % groups.len()]).collect();
    let sym = Value::sym_vec(&syms);
    let price = Value::vec(&(0..N).map(|i| i as f64).collect::<Vec<_>>());
    let size = Value::vec(&i64_data());
    let t = Table::new(&["sym", "price", "size"], &[sym, price, size]).unwrap();

    let mut g = c.benchmark_group("query");
    g.throughput(Throughput::Elements(N as u64));

    // total size by symbol
    g.bench_function("group_by_sum", |b| {
        b.iter(|| {
            let r = t
                .select()
                .agg("total", sum(col("size")))
                .by("sym")
                .execute()
                .unwrap();
            black_box(r.nrows())
        })
    });

    // filter + aggregate
    g.bench_function("filter_then_count", |b| {
        b.iter(|| {
            let r = t
                .select()
                .agg("n", col("size").count())
                .filter(col("price").gt(50_000.0))
                .execute()
                .unwrap();
            black_box(r.nrows())
        })
    });
    g.finish();
}

fn bench_serde(c: &mut Criterion) {
    ensure_runtime();
    let v = Value::vec(&i64_data());
    let bytes = v.serialize().unwrap();

    let mut g = c.benchmark_group("serde");
    g.throughput(Throughput::Bytes(bytes.len() as u64));
    g.bench_function("serialize", |b| {
        b.iter(|| black_box(v.serialize().unwrap().len()))
    });
    g.bench_function("deserialize", |b| {
        b.iter(|| {
            let d = Value::deserialize(black_box(&bytes)).unwrap();
            black_box(d.len())
        })
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_vector,
    bench_boxed_read,
    bench_aggregation,
    bench_query,
    bench_serde
);
criterion_main!(benches);
