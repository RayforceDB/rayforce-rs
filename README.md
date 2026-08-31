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

// One live runtime per process; the scope brackets its whole life.
Runtime::scope(|_rt| {
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
    Ok(())
})?;
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

The RayforceDB **core** and the **rayforce-q** IPC client are C. Both ship inside the
crate, so nothing is fetched at build time — the build script compiles them and statically
links `librayforce.a`.

```toml
# Cargo.toml
[dependencies]
rayforce = { git = "https://github.com/RayforceDB/rayforce-rs" }
```

Requirements: a C toolchain (`make`, `clang`) and `libclang` for `bindgen`.

### Working on the bindings

The C sources live in git submodules under `rayforce-sys/vendor/`, so a checkout needs
them initialized:

```sh
git clone --recurse-submodules https://github.com/RayforceDB/rayforce-rs
# in an existing clone:
git submodule update --init --recursive

cargo build
cargo test
```

### Choosing the core version

Each release links one pinned core version. It lives in two places that must agree — the
`rayforce-sys/vendor/rayforce` submodule, and the `CORE_VERSION` / `CORE_COMMIT` constants
in `rayforce-sys/build.rs` that get stamped into `librayforce.a` (a crate unpacked from
crates.io has no git history for the core's Makefile to read a version from).

To move the pin, move both:

```sh
git -C rayforce-sys/vendor/rayforce fetch --tags
git -C rayforce-sys/vendor/rayforce checkout v2.6.0
git add rayforce-sys/vendor/rayforce

git -C rayforce-sys/vendor/rayforce rev-parse --short=7 HEAD   # CORE_COMMIT
$EDITOR rayforce-sys/build.rs                                  # CORE_VERSION, CORE_COMMIT

./scripts/check-vendored-pin.sh    # names the mismatch if they disagree
cargo test --workspace
```

`rayforce-sys/vendor/rayforce-q` works the same way, minus the constants — nothing is
stamped from it.

To build against a core you are changing instead, point the build script at your own
checkout. These take precedence over the vendored copies:

```sh
export RAYFORCE_SRC=/path/to/rayforce
export RAYFORCE_Q_SRC=/path/to/rayforce-q
```

Such a checkout is built in place, so incremental state is preserved — except across a
core-flavour switch. Release and debug objects share every filename, so the first build
after `RAYFORCE_CORE_DEBUG` changes drops every object under `src/` and the
`librayforce.a` beside them, and records the flags in an untracked `.stamp`. Nothing
tracked by git is touched.

`bindgen` locates `libclang` via `LIBCLANG_PATH`. This is deliberately **not** set in the
repo's `.cargo/config.toml`. If bindgen can't auto-detect libclang, set it yourself:

```sh
# macOS (CTL):
export LIBCLANG_PATH="/Library/Developer/CommandLineTools/usr/lib"
# Linux:
export LIBCLANG_PATH="$(dirname "$(find /usr/lib -name 'libclang*.so*' | head -1)")"
```

The `chrono` is on by default for date/time/timestamp conversions.

---

**Built with ❤️ for high-performance data processing | <a href="https://rs.rayforcedb.com/content/license.html">MIT Licensed</a> | RayforceDB**
