# Installation

## Prerequisites

Before installing rayforce-rs, ensure you have:

- **Rust** 1.70 or later (`rustup update stable`)
- **C compiler** (gcc or clang) and `make`
- **LLVM/clang** development headers — required by `bindgen`
- **Git** — `build.rs` clones the rayforce C sources at build time

On Ubuntu / Debian:

```bash
sudo apt install llvm-dev libclang-dev clang git build-essential
```

On macOS:

```bash
xcode-select --install
brew install llvm
```

## Add to Cargo.toml

```toml
[dependencies]
rayforce = "0.1"
```

Or use cargo add:

```bash
cargo add rayforce
```

## Build from source

If you want to work against the latest sources:

```bash
git clone https://github.com/RayforceDB/rayforce-rs.git
cd rayforce-rs
cargo build --release
cargo test
cargo run --example basic
```

## How the build works

`build.rs` runs *before* the Rust crate compiles. On first build (or any time
`librayforce.a` is missing) it does, in order:

1. Reads `RAYFORCE_GITHUB` (defaults to
   `https://github.com/RayforceDB/rayforce.git`).
2. `git clone` into cargo's per-build `OUT_DIR`, specifically
   `OUT_DIR/rayforce-c/`.
3. `make lib` in that directory, producing `librayforce.a`.
4. `bindgen` against the single public header
   `OUT_DIR/rayforce-c/include/rayforce.h` to produce `bindings.rs`.
5. Cargo compiles the Rust crate, which `include!`s `bindings.rs` and
   links statically against `librayforce.a`.

`build.rs` checks `librayforce.a` first, so subsequent builds reuse the
existing library and finish in seconds. Run `cargo clean` to force a
fresh clone + build.

!!! note "Where the clone ends up"
    For a default debug build:
    ```
    target/debug/build/rayforce-<hash>/out/rayforce-c/
    ```
    The `<hash>` part is cargo-generated; it changes on dependency or
    toolchain upgrades.

## Iterating against a local rayforce checkout

Set `RAYFORCE_GITHUB` to a `file://` URL pointing at a local rayforce
worktree to avoid hitting GitHub during development:

```bash
RAYFORCE_GITHUB=file:///path/to/rayforce cargo build
```

Useful when working on both the C engine and the Rust bindings
simultaneously.

## Verifying installation

Create a simple test program:

```rust
// src/main.rs
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;
    println!("RayforceDB version: {}", rf.version());

    let result = rf.eval("sum 1 2 3")?;
    println!("1 + 2 + 3 = {}", result);
    Ok(())
}
```

Run it:

```bash
cargo run
```

Expected output:

```
RayforceDB version: 2.1.0
1 + 2 + 3 = 6
```

## Platform Notes

### Linux

Works out of the box on most distributions. Ensure you have:

```bash
# Ubuntu / Debian
sudo apt install llvm-dev libclang-dev clang git build-essential

# Fedora / RHEL
sudo dnf install llvm-devel clang-devel clang make gcc git
```

### macOS

```bash
xcode-select --install
brew install llvm
```

### Windows

Currently requires WSL2. Native Windows support is planned.

## Troubleshooting

### `bindgen` fails

Install the LLVM / clang development headers (see the platform-specific
commands above). On unusual setups you may also need
`LIBCLANG_PATH=/path/to/libclang.so` exported.

### Linker errors

Check that the C library built successfully — its artefacts live in
cargo's `OUT_DIR` (see [How the build works](#how-the-build-works)
above). A clean rebuild often resolves stale state:

```bash
cargo clean
cargo build
```

### Version mismatch

Same fix — `cargo clean` purges the cached clone and triggers a fresh
download + build.

## What's Next?

Now that you have rayforce-rs installed, check out the
[Quick Start](quickstart.md) guide.
