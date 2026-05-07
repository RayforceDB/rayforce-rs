# Welcome to RayforceDB Rust!

<div class="rust-badge">
    <svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg"><circle cx="16" cy="16" r="14" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="16" cy="16" r="5" fill="currentColor"/></svg>
    Built for Rust
</div>

**rayforce-rs** provides safe, ergonomic Rust bindings for [RayforceDB](https://rayforcedb.com) — the ultra-fast columnar database. The current bindings target **rayforce 2.0**.

## Why Rust?

RayforceDB is written in pure C for maximum performance. **rayforce-rs** brings that performance to Rust with:

- **Memory Safety** — No null pointers, no buffer overflows, no data races
- **Zero-Cost Abstractions** — Idiomatic Rust API that compiles to efficient C calls
- **Fearless Concurrency** — Share data safely across threads
- **Type Safety** — Catch errors at compile time, not runtime

## Quick Overview

```rust
use rayforce::{Rayforce, RayI64, RayVector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the RayforceDB runtime
    let rf = Rayforce::new()?;

    // Create typed values
    let _price = RayI64::new(100);
    let _prices: RayVector<i64> = RayVector::from_iter([100i64, 200, 300]);

    // Evaluate Rayfall expressions
    let result = rf.eval("sum 1 2 3")?;
    println!("1 + 2 + 3 = {}", result);

    Ok(())
}
```

## Feature Highlights

<div class="grid cards" markdown>

- :material-lightning-bolt: **Blazing Fast**
  
    Sub-millisecond query performance on analytical workloads through columnar storage and vectorized operations.

- :fontawesome-brands-rust: **Rust Idiomatic**
  
    Familiar patterns: `From`/`Into` traits, iterators, `Result` error handling, and smart pointers.

- :material-database: **Tables & Rayfall**

    Full type system: scalars, vectors, lists, dicts, tables. Build queries fluently with `SelectQuery` / `UpdateQuery` / `InsertQuery` / `UpsertQuery`, or hand-write Rayfall source — both paths run through the same engine.

- :material-shield-check: **Safe ref counting**

    Every `RayObj` owns one strong reference. `Clone` calls `ray_retain`; `Drop` calls `ray_release`. The borrow checker keeps lifetimes honest.

- :material-lan: **Remote IPC**

    Talk to a Rayforce server with `Connection::connect(host, port)` (auth optional) and ship Rayfall over the wire via `send` / `send_async` / `send_verbose`.

</div>

## What's Next?

1. **[Installation](installation.md)** — Add rayforce-rs to your project
2. **[Quick Start](quickstart.md)** — Build your first application
3. **[API Reference](../api/overview.md)** — Explore the full API

## System Requirements

- **Rust**: 1.70 or later
- **OS**: Linux, macOS (Windows via WSL2)
- **Build Tools**: a C17 compiler (gcc/clang), `make`, `git`, and LLVM/clang for `bindgen`

!!! tip "Need Help?"
    Join the [RayforceDB Zulip](https://rayforcedb.zulipchat.com) community or open an issue on [GitHub](https://github.com/RayforceDB/rayforce-rs).
