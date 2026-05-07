# API Overview

Reference for the rayforce-rs API as it stands against rayforce 2.0.

## Module structure

```
rayforce
├── ffi           # Low-level FFI: RayObj, accessor helpers, raw bindings
├── types         # Type system
│   ├── scalars   # Scalar atom wrappers (RayI64, RayF64, RaySymbol, ...)
│   ├── containers# RayVector, RayList, RayString, RayDict
│   └── table     # RayTable
├── query         # SelectQuery / UpdateQuery / InsertQuery / UpsertQuery
│                 # plus Column / Expression / Operation
├── ipc           # Connection (blocking remote-server client)
└── error         # RayforceError + ray_err_t mapping
```

## Core types

### Runtime

| Type | Description |
|------|-------------|
| `Rayforce` | Main runtime handle. Wraps `*mut ray_runtime_t`. |
| `RayforceBuilder` | Builder for constructing a `Rayforce` with custom argv. |
| `RayObj` | Owned `*mut ray_t` with refcount-managed lifetime. |

### Scalar atom types

| Type | Description | Rust equivalent |
|------|-------------|-----------------|
| `RayBool` (`B8`) | Boolean atom | `bool` |
| `RayU8` (`U8`) | Unsigned byte | `u8` |
| `RayI16` (`I16`) | 16-bit signed integer | `i16` |
| `RayI32` (`I32`) | 32-bit signed integer | `i32` |
| `RayI64` (`I64`) | 64-bit signed integer | `i64` |
| `RayF64` (`F64`) | 64-bit floating point | `f64` |
| `RayString` | SSO string atom (≤7 bytes inline, pool-backed otherwise) | `String` |
| `RaySymbol` (`Symbol`) | Interned string symbol | — |
| `RayDate` (`Date`) | Date value | `chrono::NaiveDate` |
| `RayTime` (`Time`) | Time value | `chrono::NaiveTime` |
| `RayTimestamp` (`Timestamp`) | Timestamp | `chrono::NaiveDateTime` |
| `RayGuid` (`GUID`) | 16-byte GUID | `uuid::Uuid` |

!!! note "RayChar (`C8`) was removed"
    The single-character atom type doesn't exist in rayforce 2.0. Use
    `RayU8::new(b'a')` or `RayString::new("a")`.

### Container types

| Type | Description |
|------|-------------|
| `RayVector<T>` | Homogeneous typed column. Specialised for `i64`, `f64`, `RaySymbol`. |
| `RayList` | Heterogeneous boxed list (`RAY_LIST`). |
| `RayDict` | Symbol-keyed dictionary (`RAY_DICT`). |
| `RayTable` | Columnar table (`RAY_TABLE`). |

### Query types

The query builder is back. `SelectQuery`, `UpdateQuery`,
`InsertQuery`, `UpsertQuery` plus the helper types `Column`,
`Expression`, and `Operation` are exported at the crate root. Each
builder renders Rayfall source on demand and dispatches through
`Rayforce::eval`, so the underlying engine call is the same as a
hand-written `eval` — the builder is the syntax sugar.

See **[Query Builder](query.md)** for the full reference.

### IPC client

`Connection` wraps the public `ray_ipc_*` C symbols
(connect / close / send / send_async / send_verbose) for talking to
a remote Rayforce server. See **[IPC](ipc.md)** for the full
reference.

## Key traits

### `RayType`

Every typed wrapper implements `RayType`:

```rust
pub trait RayType: Sized {
    /// 2.0 type tag (negative for atoms, non-negative for vectors / compounds).
    const TYPE_CODE: i8;
    /// Human-readable name for error messages.
    const RAY_NAME: &'static str;

    fn from_ptr(ptr: RayObj) -> Result<Self>;
    fn ptr(&self) -> &RayObj;
    fn type_code(&self) -> i8 { self.ptr().type_code() }
}
```

### `From` / `TryFrom` conversions

`RayObj` implements `From<T>` for the Rust primitive atom types:

```rust
let obj = RayObj::from(42i64);
let obj = RayObj::from(3.14f64);
let obj = RayObj::from(true);
let obj = RayObj::from("hello");          // → RAY_STR atom
let obj = RayObj::from([1i64, 2, 3].as_slice());   // → RAY_I64 vector
```

And `TryFrom<RayObj>` going the other direction:

```rust
let n: i64    = obj.try_into()?;
let x: f64    = obj.try_into()?;
let b: bool   = obj.try_into()?;
let s: String = obj.try_into()?;
```

## Error handling

All fallible operations return `Result<T, RayforceError>`:

```rust
use rayforce::{Rayforce, RayforceError, Result};

fn main() -> Result<()> {
    let rf = Rayforce::new()?;
    let _result = rf.eval("sum 1 2 3")?;
    Ok(())
}
```

### Error variants

| Variant | When |
|---------|------|
| `RuntimeCreationFailed` | `ray_runtime_create` returned NULL. |
| `EvalFailed(String)` | Generic eval failure. |
| `TypeMismatch { expected, actual }` | A typed `from_ptr` saw the wrong tag. |
| `IndexOutOfBounds { index, length }` | Out-of-range vector / table access. |
| `NullPointer` | Hit a NULL where a value was expected. |
| `InvalidString` | Embedded NUL byte / non-UTF-8 in input. |
| `KeyNotFound(String)` | Missing dict / table column. |
| `IoError(String)` | Filesystem / network I/O. |
| `ConversionError(String)` | Conversion failure. |
| `AllocationFailed` | `ray_alloc` / construction returned NULL. |
| `InvalidGuid(String)` | GUID parse failure. |
| `CApiError(String)` | Generic C-side failure. |
| `Ray { code, message, kind }` | Engine error from `ray_eval_str`. `kind` is the matching `ray_err_t`. |

## Quick reference

### Creating values

```rust
use rayforce::*;

// Scalars
let i = RayI64::new(42);
let f = RayF64::new(3.14);
let s = RaySymbol::new("name");
let str_ = RayString::new("hello");

// Vectors
let v: RayVector<i64> = RayVector::from_iter([1i64, 2, 3]);
let s = RayVector::<RaySymbol>::from_iter(["a", "b", "c"]);

// Lists
let mut l = RayList::new();
l.push(42i64);
l.push("hello");

// Dictionaries (string keys are interned to symbols)
let d = RayDict::from_pairs([
    ("key", RayString::new("value").ptr().clone()),
])?;
```

### Evaluating expressions

```rust
let rf = Rayforce::new()?;
let result = rf.eval("sum 1 2 3")?;        // returns RayObj wrapping 6
let n: i64 = result.try_into()?;
```

`Rayforce::eval` takes Rayfall source as `&str` and returns a `RayObj`.
On engine errors it returns `RayforceError::Ray { code, message, kind }`
where `code` is the short tag (`"oom"`, `"type"`, `"range"`, …) from
`ray_err_code`.

### Querying tables

```rust
let table = rf.eval("(table [a b] (list [1 2] [3 4]))")?;

// Bind under a name and run a select via eval:
rf.eval("(set t (table [a b] (list [1 2] [3 4])))")?;
let result = rf.eval("(select {from:t a:a where:(> b 2)})")?;
```

## Next steps

- **[Scalars](types/scalars.md)** — detailed scalar type docs.
- **[Containers](types/containers.md)** — `RayVector` / `RayList` / `RayDict` / `RayString`.
- **[Tables](types/table.md)** — `RayTable` reference.
- **[FFI](ffi.md)** — low-level details (`RayObj`, accessor helpers, raw bindings).
