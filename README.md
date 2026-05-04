# rayforce-rs

Rust bindings for [RayforceDB](https://github.com/RayforceDB/rayforce) — a
zero-dependency embeddable engine that fuses columnar analytics and graph
traversal into a single morsel-driven pipeline.

This crate targets **rayforce 2.0**. If you're upgrading from a 1.0
release, see [Migrating from 1.0](#migrating-from-10).

## Features

- **Type system**: `RayBool`, `RayU8`, `RayI16`, `RayI32`, `RayI64`,
  `RayF64`, `RayString`, `RaySymbol`, `RayDate`, `RayTime`,
  `RayTimestamp`, `RayGuid`.
- **Containers**: `RayList` (heterogeneous boxed list), `RayVector<T>`
  (typed columns), `RayDict`, `RayTable`.
- **Rayfall evaluation**: `Rayforce::eval` runs Rayfall source against
  the runtime and returns the result as a typed `RayObj`.
- **Automatic build**: `build.rs` clones the rayforce C sources from
  GitHub and builds the static library before generating bindings.

## Installation

```toml
[dependencies]
rayforce = "0.1"
```

### Build requirements

- **Clang/LLVM** — required by `bindgen`.
- **Git** — for cloning the rayforce sources.
- **Make** + a **C17 compiler** (gcc or clang) — for building the C
  static library.

On Ubuntu/Debian:

```bash
sudo apt install llvm-dev libclang-dev clang git build-essential
```

On macOS:

```bash
xcode-select --install
brew install llvm
```

## Quick start

```rust
use rayforce::{Rayforce, RayI64, RayF64, RayList, RaySymbol, RayString,
               RayVector, RayDict, RayType, Result};

fn main() -> Result<()> {
    let rf = Rayforce::new()?;
    println!("rayforce {}", rf.version());

    // Atoms
    let i = RayI64::new(42);
    let f = RayF64::new(3.14);
    let s = RayString::new("hello");
    let sym = RaySymbol::new("greeting");
    println!("{i} {f} {s} {sym}");

    // Typed vectors
    let ints = RayVector::<i64>::from_iter([1i64, 2, 3, 4, 5]);
    let floats = RayVector::<f64>::from_iter([1.1, 2.2, 3.3]);
    let names = RayVector::<RaySymbol>::from_iter(["alice", "bob", "carol"]);
    println!("{ints} {floats} ({} names)", names.len());

    // Heterogeneous list
    let mut list = RayList::new();
    list.push(1i64);
    list.push("two");
    list.push(3.0f64);

    // Dict
    let dict = RayDict::from_pairs([
        ("name", RayString::new("Alice").ptr().clone()),
        ("age", RayI64::new(30).ptr().clone()),
    ])?;
    println!("dict: {dict:?}");

    // Run a Rayfall expression
    let sum = rf.eval("sum 1 2 3 4 5")?;
    println!("sum: {sum}");
    Ok(())
}
```

## Working with tables

Build a table directly from `(name, column)` pairs:

```rust
use rayforce::{RayTable, RayVector, RaySymbol};

let table = RayTable::from_dict([
    ("id",    RayVector::<i64>::from_iter([1i64, 2, 3]).as_ray_obj().clone()),
    ("name",  RayVector::<RaySymbol>::from_iter(["a", "b", "c"]).as_ray_obj().clone()),
    ("score", RayVector::<f64>::from_iter([95.5, 87.3, 92.1]).as_ray_obj().clone()),
])?;

println!("{} rows × {} cols", table.len()?, table.ncols());
println!("columns: {:?}", table.columns()?);
let scores = table.get_column("score")?;
println!("scores: {scores}");
```

### Querying tables

Run queries through `Rayforce::eval` with a Rayfall source string. The
table is bound under a name in the global environment first:

```rust
table.save("trades")?;
let result = rf.eval("(select {from:trades by: name score:(sum score)})")?;
println!("{result}");
```

## Runtime

```rust
let rf = Rayforce::new()?;
println!("major: {}", rf.version_major());     // 2
println!("string: {}", rf.version());           // "2.1.0"
let r = rf.eval("1 + 2 * 3")?;                  // RayObj wrapping 7i64
let n: i64 = r.try_into()?;
assert_eq!(n, 7);
```

Errors returned by the engine come back as `RayforceError::Ray { code,
message, kind }` where `code` is the short tag (`"oom"`, `"type"`,
`"range"`, ...) and `kind` is the matching `ray_err_t` enum.

## Environment variables

- `RAYFORCE_GITHUB`: override the rayforce repository URL (default
  `https://github.com/RayforceDB/rayforce.git`). Useful when iterating
  against a local checkout — set it to a `file://` URL pointing at the
  worktree.

## Migrating from 1.0

The 2.0 C API was a substantial rewrite. The Rust bindings followed:

- **`Rayforce::eval`** still works, but `eval_obj` is gone (the engine
  no longer exposes object-level eval; pass a Rayfall source string).
- **`Rayforce::version`** now returns a `String` ("2.1.0") instead of
  a `u8`. Use `version_major/minor/patch` for the numeric components.
- **Atoms**: same wrappers, same names. `RayChar` (`C8`) was dropped —
  rayforce 2.0 has no single-char atom; use `RayString` or `RayU8`
  depending on the situation.
- **`RaySymbol`**: construction is unchanged from a Rust caller's
  perspective; under the hood it now goes through `ray_sym_intern`.
- **`RayString`**: was a `TYPE_C8` char vector in 1.0; now wraps a
  `RAY_STR` atom (SSO under 7 bytes, pool reference for longer
  strings).
- **`RayVector::set`** is back, but goes through `ray_vec_set` (COW).
  The vector wrapper adopts the returned pointer.
- **Removed**: the entire query-builder API
  (`Table::select()`/`update()`/`insert()`/`upsert()`,
  `Column::new(...)`, `RayExpression`). The C symbols backing it
  (`ray_select`, `ray_update`, `ray_insert`, `ray_upsert`) are no
  longer in the public 2.0 C API. Compose Rayfall source strings and
  pass them to `Rayforce::eval` instead. A future version may
  re-introduce a Rust-side fluent builder that synthesises Rayfall
  strings.
- **Removed**: the `ipc` module (`hopen` / `Connection`).
  `ray_hopen` / `ray_hclose` / `ray_write` / `ray_read` are no longer
  in the 2.0 public C API. The module will return when upstream
  re-exposes these symbols.
- **Removed**: helpers tied to the old internal-function lookup
  (`get_internal_function`, `binary_set`, `quote`). Use
  `set_global(name, value)` (now backed by `ray_env_set` +
  `ray_sym_intern`) to bind values into the global environment.

`Display` for `RayObj` and the typed wrappers is intentionally minimal
in this release — rayforce 2.0 does not yet expose a public
pretty-printer (the 1.0 `obj_fmt` is internal). Numeric vectors render
their element values; everything else prints as `RayObj(type=…,
len=…)`. Richer formatting is planned as a follow-up once the engine
exposes a stable formatter.

## License

MIT — see [LICENCE](LICENCE).

## See also

- [RayforceDB](https://github.com/RayforceDB/rayforce) — the Rayforce
  database.
- [rayforce-py](https://github.com/RayforceDB/rayforce-py) — official
  Python bindings.
