# :octicons-package-16: Installation

`rayforce` builds against a local checkout of the RayforceDB core. The build
script compiles the core into a static library (`librayforce.a`) and statically
links it, so there is nothing to install at runtime.

## :material-clipboard-check-outline: Prerequisites

- A recent **Rust** toolchain (edition 2021, `rustc` 1.74 or newer). Install via
  [rustup](https://rustup.rs).
- A **C toolchain** — `make` and `clang` — to build the RayforceDB core.
- **`libclang`**, required by [`bindgen`](https://github.com/rust-lang/rust-bindgen)
  to generate the raw FFI bindings.
- A **RayforceDB core** checkout to link against (see below).

!!! note "macOS: `LIBCLANG_PATH`"
    On macOS `bindgen` may not find `libclang` automatically. Point it at your
    installation, for example:

    ```sh
    export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
    ```

    The repository's `.cargo/config.toml` is the place to set this permanently
    for local builds.

## :material-source-branch: Building against a local core

The build links a local RayforceDB core checkout. Point the `RAYFORCE_SRC`
environment variable at it; it defaults to `~/rayforce`. The build script runs
the core's `make lib` to produce `librayforce.a`, then links it.

```sh
# Clone the core somewhere, e.g. your home directory.
git clone https://github.com/RayforceDB/rayforce.git ~/rayforce

# Point the build at it (default is ~/rayforce, so this is optional there).
export RAYFORCE_SRC=~/rayforce

# Build and test the bindings.
cargo build
cargo test
```

!!! note "Tests run single-threaded"
    The engine runs on a single thread with one live runtime per process, so the
    test suite is serialized. Run it with `RUST_TEST_THREADS=1` (or via the
    crate's `serial_test` setup) if you invoke tests directly.

## :material-package-variant-closed: Adding the dependency

Add `rayforce` to your crate:

```sh
cargo add rayforce
```

or in `Cargo.toml`:

```toml
[dependencies]
rayforce = "0.1"
```

### The `chrono` feature (default)

The `chrono` feature is enabled by default. It adds conversions between
RayforceDB temporal types and [`chrono`](https://docs.rs/chrono) types —
`NaiveDate` ↔ date, `NaiveTime` ↔ time, and `DateTime<Utc>` ↔ timestamp (the
RayforceDB epoch is `2000-01-01`).

To build without it, disable default features:

```toml
[dependencies]
rayforce = { version = "0.1", default-features = false }
```

## :material-arrow-right: Next steps

- [:material-human-greeting-variant: Overview](overview.md) — the 30-second
  quickstart.
- [:material-cog-outline: Technical Details](technical-details.md) — how the
  workspace and runtime are put together.
