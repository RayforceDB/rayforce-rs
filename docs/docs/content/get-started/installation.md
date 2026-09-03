# :octicons-package-16: Installation

The RayforceDB core is C, and it ships inside the `rayforce-sys` crate as a
pinned git submodule. The build script compiles it into a static library
(`librayforce.a`) and links it statically — so there is nothing to fetch while
building, and nothing to install at runtime.

## :material-clipboard-check-outline: Prerequisites

- A recent **Rust** toolchain (edition 2021, `rustc` 1.74 or newer). Install via
  [rustup](https://rustup.rs).
- A **C toolchain** — `make` and `clang` — to build the RayforceDB core.
- **`libclang`**, required by [`bindgen`](https://github.com/rust-lang/rust-bindgen)
  to generate the raw FFI bindings.

!!! note "macOS: `LIBCLANG_PATH`"
    On macOS `bindgen` may not find `libclang` automatically. Point it at your
    installation, for example:

    ```sh
    export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
    ```

    The repository's `.cargo/config.toml` is the place to set this permanently
    for local builds.

## :material-tag-outline: Which core version gets linked

Each release of `rayforce` links one specific core version. It is pinned in two
places that must agree:

| What | Where |
| --- | --- |
| The core sources | the `rayforce-sys/vendor/rayforce` submodule |
| The version stamped into the library | `CORE_VERSION` / `CORE_COMMIT` in `rayforce-sys/build.rs` |

The constants exist because the core's `Makefile` normally resolves its version
from `git describe`, and a crate unpacked from crates.io has no git history to
read. `scripts/check-vendored-pin.sh` asserts the two agree, and CI runs it on
every push.

!!! note "The submodules are fetched over SSH"
    `.gitmodules` addresses both submodules as `git@github.com:RayforceDB/…`. An
    existing clone picks that up with `git submodule sync --recursive` before
    `git submodule update --init --recursive`. Without a GitHub SSH key, rewrite
    the URLs to https once:

    ```sh
    git config --global url."https://github.com/".insteadOf "git@github.com:"
    ```

    A `git = "https://…"` Cargo dependency fetches the submodules through Cargo,
    which honours that rewrite only when it shells out to `git` — set
    `net.git-fetch-with-cli = true` in `~/.cargo/config.toml`. A crates.io
    dependency needs none of this; the sources ship inside the crate.

As a consumer you get the core that matches the `rayforce` version you depend
on — pick a different core by picking a different `rayforce` release. The two
sections below are for changing that pin yourself.

### :material-source-branch: Building against your own core checkout

To develop against a core you are changing, point `RAYFORCE_SRC` at it. It takes
precedence over the vendored copy, and is built in place so your incremental
state and the version its git history reports are preserved. `RAYFORCE_Q_SRC`
does the same for the `rayforce-q` IPC client.

!!! warning "Switching core flavour rebuilds your checkout from scratch"

    The one exception to "incremental state is preserved". Release and debug
    objects share every filename and `make` tracks headers but not flags, so
    flipping `RAYFORCE_CORE_DEBUG` would otherwise archive a mixed library. The
    build script therefore drops every object under `src/` and the
    `librayforce.a` beside them on the first build after a change, and records
    the flags in an untracked `.stamp` file next to them. Nothing tracked by git
    is touched.

```sh
export RAYFORCE_SRC=/path/to/rayforce
export RAYFORCE_Q_SRC=/path/to/rayforce-q

cargo build
cargo test
```

Unset them to go back to the vendored sources.

### :material-arrow-up-bold-box-outline: Bumping the pinned version

Moving the pin means moving the submodule and the constants together:

```sh
# 1. Move the submodule to the new tag.
git -C rayforce-sys/vendor/rayforce fetch --tags
git -C rayforce-sys/vendor/rayforce checkout v2.6.0
git add rayforce-sys/vendor/rayforce

# 2. Read back the values build.rs must stamp.
git -C rayforce-sys/vendor/rayforce describe --tags --exact-match   # -> v2.6.0
git -C rayforce-sys/vendor/rayforce rev-parse --short=7 HEAD        # -> e.g. 1a2b3c4
```

Then edit `rayforce-sys/build.rs` to match — `CORE_VERSION` is the tag without
its leading `v`:

```rust
const CORE_VERSION: &str = "2.6.0";
const CORE_COMMIT: &str = "1a2b3c4";
```

And check the result:

```sh
./scripts/check-vendored-pin.sh   # fails, with the mismatch named, if they disagree
cargo test --workspace
```

The same applies to `rayforce-sys/vendor/rayforce-q`, minus the constants —
nothing is stamped from it, so moving the submodule is the whole change.

!!! warning "A new core may need the bindgen allowlist updated"
    A few of the symbols the safe crate calls are not in the public
    `rayforce.h` — they are read from the core's private headers instead.
    `CORE_PRIVATE_HEADERS` and `INTERNAL_FNS` in `rayforce-sys/build.rs` name
    those headers and symbols. Signatures need no maintenance, since bindgen
    reads them from the core, but a bump that renames or relocates one will
    fail the build: bindgen emits nothing for it and the safe crate stops
    compiling against `rayforce_sys`. Fix it by updating those two lists.

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
rayforce = "1"
```

### The `chrono` feature (default)

The `chrono` feature is enabled by default. It adds conversions between
RayforceDB temporal types and [`chrono`](https://docs.rs/chrono) types —
`NaiveDate` ↔ date, `NaiveTime` ↔ time, and `DateTime<Utc>` ↔ timestamp (the
RayforceDB epoch is `2000-01-01`).

To build without it, disable default features:

```toml
[dependencies]
rayforce = { version = "1", default-features = false }
```

## :material-arrow-right: Next steps

- [:material-human-greeting-variant: Overview](overview.md) — the 30-second
  quickstart.
- [:material-cog-outline: Technical Details](technical-details.md) — how the
  workspace and runtime are put together.
