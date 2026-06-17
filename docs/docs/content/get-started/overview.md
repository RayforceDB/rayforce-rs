# :material-human-greeting-variant: Introduction to Rayforce-RS

`rayforce` provides convenient, high-performance Rust bindings for
[RayforceDB](https://github.com/RayforceDB/rayforce) v2 — a lightweight,
SIMD-vectorized columnar database. It lets you build values, tables, and queries
with idiomatic Rust and run them inside the RayforceDB runtime with little-to-no
practical overhead.

```rust
use rayforce::{col, sum, Runtime, Table, Value};

let _rt = Runtime::new()?;                        // one live runtime per process

let t = Table::new(
    &["sym", "price", "size"],
    &[
        Value::sym_vec(&["AAPL", "MSFT", "AAPL", "GOOG"]),
        Value::vec(&[100.0f64, 200.0, 110.0, 300.0]),
        Value::vec(&[10i64, 20, 30, 40]),
    ],
)?;

// select total:sum size by sym from t where price > 150.0
let totals = t
    .select()
    .agg("total", sum(col("size")))
    .filter(col("price").gt(150.0))
    .by("sym")
    .execute()?;

println!("{totals}");
# Ok::<(), rayforce::RayError>(())
```

## :material-flash: Direct FFI, no shim

The bindings link the RayforceDB core (`librayforce.a`) directly and call its C
API natively. There is no separate process, no IPC shim, and no serialization
between your Rust code and the engine for in-process work.

!!! note "Zero-copy where it counts"
    A `Value` is a thin, reference-counted handle onto memory owned by the
    engine. Building a numeric column is a single `memcpy`
    (`Value::vec(&[T])`), and reading it back is zero-copy: a numeric column is
    exposed as a `&[T]` slice via `value.as_slice::<T>()` rather than copied
    element-by-element. Cloning a `Value` only bumps a refcount.

The result is a thin, ergonomic safe layer over a fast vectorized engine —
without the cost of crossing a process boundary.

## :material-run: 30-second quickstart

Every program starts by creating a single live [`Runtime`](installation.md). It
is an RAII guard: keep it alive for as long as you touch any `Value`.

```rust
use rayforce::{col, Runtime, Table, Value};

let _rt = Runtime::new()?;

// Build a table from typed columns.
let trades = Table::new(
    &["sym", "price", "size"],
    &[
        Value::sym_vec(&["AAPL", "MSFT", "AAPL"]),
        Value::vec(&[100.0f64, 200.0, 110.0]),
        Value::vec(&[10i64, 20, 30]),
    ],
)?;

// Filter and project with the fluent query DSL.
let big = trades
    .select()
    .filter(col("size").gt(15i64))
    .execute()?;

println!("{big}");
# Ok::<(), rayforce::RayError>(())
```

!!! tip "Integer literals default to `i32`"
    A bare literal like `42` is `i32` in Rust. Annotate the element type when you
    build non-`i32` vectors or atoms — `Value::vec(&[1i64, 2, 3])`,
    `col("size").gt(15i64)`.

## :material-arrow-right: Next steps

- [:octicons-package-16: Installation](installation.md) — prerequisites and how to
  build against a local RayforceDB core.
- [:material-cog-outline: Technical Details](technical-details.md) — the two-crate
  workspace, the `Value` RAII model, and the single-thread runtime.
- [:material-file-document: Documentation](../documentation/overview.md) — data
  types, tables, the query guide, IPC, and serialization.
