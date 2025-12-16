# Welcome to RayforceDB Rust!

<div class="rust-badge">
    <svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg"><circle cx="16" cy="16" r="14" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="16" cy="16" r="5" fill="currentColor"/></svg>
    Built for Rust
</div>

**rayforce-rs** provides safe, ergonomic Rust bindings for [RayforceDB](https://rayforcedb.com) - the ultra-fast columnar database.

## Why Rust?

RayforceDB is written in pure C for maximum performance. **rayforce-rs** brings that performance to Rust with:

- **Memory Safety** - No null pointers, no buffer overflows, no data races
- **Zero-Cost Abstractions** - Idiomatic Rust API that compiles to efficient C calls
- **Fearless Concurrency** - Share data safely across threads
- **Type Safety** - Catch errors at compile time, not runtime

## Quick Overview

```rust
use rayforce::{Rayforce, RayI64, RayVector, RayTable};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize RayforceDB runtime
    let ray = Rayforce::new()?;
    
    // Create typed values
    let price = RayI64::from_value(100);
    let prices: RayVector<i64> = RayVector::from_iter([100, 200, 300]);
    
    // Evaluate expressions
    let result = ray.eval("(+ 1 2 3)")?;
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

- :material-database: **Full API Coverage**
  
    All RayforceDB types: scalars, vectors, lists, dicts, tables. All queries: select, update, insert, upsert, joins.

- :material-connection: **IPC Support**
  
    Connect to remote RayforceDB instances with async-ready networking.

</div>

## What's Next?

1. **[Installation](installation.md)** - Add rayforce-rs to your project
2. **[Quick Start](quickstart.md)** - Build your first application
3. **[API Reference](../api/overview.md)** - Explore the full API

## System Requirements

- **Rust**: 1.70 or later
- **OS**: Linux, macOS (Windows support coming soon)
- **Build Tools**: C compiler (gcc/clang) for building the RayforceDB C library

!!! tip "Need Help?"
    Join the [RayforceDB Zulip](https://rayforcedb.zulipchat.com) community or open an issue on [GitHub](https://github.com/RayforceDB/rayforce-rs).

