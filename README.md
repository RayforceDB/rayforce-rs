<table style="border-collapse:collapse;border:0;">
  <tr>
    <td style="border:0;padding:0;">
      <a href="https://rs.rayforcedb.com">
        <picture>
            <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/RayforceDB/rayforce-rs/refs/heads/master/docs/docs/assets/logo_light_full.svg">
            <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/RayforceDB/rayforce-rs/refs/heads/master/docs/docs/assets/logo_dark_full.svg">
            <img src="https://raw.githubusercontent.com/RayforceDB/rayforce-rs/refs/heads/master/docs/docs/assets/logo_dark_full.svg" width="200">
        </picture>
      </a>
    </td>
    <td style="border:0;padding:0;">
      <h1>High-Performance, Zero-Copy Rust bindings for <a href="https://core.rayforcedb.com"><picture>
            <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/RayforceDB/rayforce-rs/refs/heads/master/docs/docs/assets/logo_light_full.svg">
            <img src="https://raw.githubusercontent.com/RayforceDB/rayforce-rs/refs/heads/master/docs/docs/assets/logo_dark_full.svg" alt="RayforceDB" height="40" style="vertical-align: bottom;">
        </picture></a></h1>
    </td>
  </tr>
</table>

[![CI](https://github.com/RayforceDB/rayforce-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/RayforceDB/rayforce-rs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rayforce.svg)](https://crates.io/crates/rayforce)
[![Docs.rs](https://img.shields.io/docsrs/rayforce)](https://docs.rs/rayforce)
[![License: MIT](https://img.shields.io/badge/License-MIT-e5601f.svg)](LICENCE)
![Rust Version](https://img.shields.io/badge/rustc-1.74%2B-orange.svg)

Rust bindings for [RayforceDB](https://core.rayforcedb.com), a high-performance columnar
database designed for analytics and data operations. The core is written in pure C with
minimal overhead — combining columnar storage with SIMD vectorization for lightning-fast
analytics on time-series and big-data workloads.

The bindings call the core's C API **directly** (no marshalling shim), so reads are
zero-copy where it counts: a numeric column is exposed as a `&[T]` slice rather than
copied element by element.

**Full Documentation:** https://rs.rayforcedb.com/

## Features

- **Fluent API** — chainable, intuitive query builders that read like the operation; real
  operator overloads for arithmetic, methods for comparisons.
- **Zero-Copy & High Performance** — build a column in a single `memcpy`, read it back as a
  borrow; minimal overhead between Rust and the RayforceDB runtime via the C API.
- **Type-Safe** — a full value model (atoms, vectors, lists, dicts, tables) with
  `ToValue`/`FromValue` conversions and optional `chrono` temporals.
- **Lightweight** — the core is less than a 1 MB footprint.
- **Batteries included** — CSV & splayed I/O, binary serialization, and a TCP/IPC client.

## Quick Start

```rust
use rayforce::{col, Runtime, Table, Value};

let _rt = Runtime::new()?;                       // one live runtime per process

let quotes = Table::new(
    &["symbol", "bid", "ask"],
    &[
        Value::sym_vec(&["AAPL", "AAPL", "AAPL", "GOOG", "GOOG", "GOOG"]),
        Value::vec(&[100.0f64, 101.0, 102.0, 200.0, 201.0, 202.0]),
        Value::vec(&[110.0f64, 111.0, 112.0, 210.0, 211.0, 212.0]),
    ],
)?;

let result = quotes
    .select()
    .agg("max_bid", col("bid").max())
    .agg("min_bid", col("bid").min())
    .agg("avg_ask", col("ask").avg())
    .agg("count",   col("bid").count())
    .filter(col("bid").ge(110.0).and(col("ask").gt(100.0)))
    .by("symbol")
    .execute()?;

println!("{result}");
```

```text
┌────────┬─────────┬─────────┬─────────┬───────┐
│ symbol │ max_bid │ min_bid │ avg_ask │ count │
│  SYM   │   F64   │   F64   │   F64   │  I64  │
├────────┼─────────┼─────────┼─────────┼───────┤
│ GOOG   │ 202.0   │ 200.0   │ 211.0   │ 3     │
├────────┴─────────┴─────────┴─────────┴───────┤
│ 1 rows (1 shown) 5 columns (5 shown)         │
└──────────────────────────────────────────────┘
```

## Installation

The crate links a local RayforceDB core checkout. Point `RAYFORCE_SRC` at it (defaults to
`~/rayforce`); the build script compiles and statically links `librayforce.a`.

```toml
# Cargo.toml
[dependencies]
rayforce = { git = "https://github.com/RayforceDB/rayforce-rs" }
```

```sh
export RAYFORCE_SRC=/path/to/rayforce      # default: ~/rayforce
cargo build
cargo test
```

Requirements: a C toolchain (`make`, `clang`) and `libclang` for `bindgen` (on macOS,
`LIBCLANG_PATH` may be needed — see `.cargo/config.toml`). The `chrono` feature is on by
default for date/time/timestamp conversions.

---

**Built with ❤️ for high-performance data processing | <a href="https://rs.rayforcedb.com/content/license.html">MIT Licensed</a> | RayforceDB**
