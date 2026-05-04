# FFI (Foreign Function Interface)

The `rayforce::ffi` module exposes the safe `RayObj` wrapper plus a
small set of helpers over the bindgen-generated C bindings.

!!! warning "Advanced usage"
    Most users should stay on the high-level API
    ([`Rayforce`](../get-started/quickstart.md), the typed scalar /
    container wrappers, `RayTable`). The FFI surface is here for
    callers that need raw `ray_t*` access or want to call C functions
    that don't yet have a safe wrapper.

## What's in this layer

- `RayObj` — owned `*mut ray_t` with refcount-managed lifetime.
- `read_*` accessor helpers — `unsafe fn`s that walk the bindgen
  `__bindgen_anon_*` paths to read fields of `ray_t`. They live in
  `src/ffi.rs` as `pub(crate)` and are surfaced here mostly for
  reference; user-facing code should normally rely on the typed
  wrappers.
- A handful of constructor / mutation helpers used by the typed
  layers (`new_list`, `new_vector`, `push_to_list`, `list_get`,
  `new_symbol`, `new_table_from_pairs`, `new_dict`, `set_global`).
- Re-export of every bindgen-generated `ray_*` extern through the
  crate root (so you can call e.g. `rayforce::ray_eval_str` directly).

## Core types

### `RayObj`

The fundamental owned wrapper:

```rust
use rayforce::RayObj;

let obj = RayObj::from(42i64);

// Type / length / nil / error tests
let t   = obj.type_code();        // -RAY_I64 (== -5)
let n   = obj.len();              // 0 for non-string atoms
let nil = obj.is_nil();           // false
let err = obj.is_error();         // false

// Refcount inspection
let rc = obj.ref_count();
```

`Clone` calls `ray_retain`; `Drop` calls `ray_release`. NULL pointers
are tolerated — `is_nil()` returns true and `Drop` is a no-op.

### `ray_t`

The bindgen-rendered C struct. It's a Rust struct with two
`__BindgenUnionField` arms; the `read_*` helpers walk the allocated
arm:

```rust
// Pseudocode (simplified bindgen output):
#[repr(C)]
pub struct ray_t {
    pub __bindgen_anon_1: __BindgenUnionField<ray_t__bindgen_ty_1>,
    pub __bindgen_anon_2: __BindgenUnionField<ray_t__bindgen_ty_2>,
    pub bindgen_union_field: [u64; 4],
}
```

Total size is **32 bytes** — the data buffer for vectors / strings
follows immediately afterwards. `read_data_ptr` exploits this:

```rust
// Slice-aware element data pointer.
unsafe fn read_data_ptr(p: *const ray_t) -> *mut u8 {
    let attrs = read_attrs(p);
    if attrs & (RAY_ATTR_SLICE as u8) != 0 {
        // Walk the slice metadata to the parent's data + offset.
    } else {
        // Inline: data[] starts at offset 32.
        (p as *mut u8).add(std::mem::size_of::<ray_t>())
    }
}
```

## Runtime

```rust
use rayforce::Rayforce;

let rf = Rayforce::new()?;
println!("major: {}", rf.version_major());      // 2
println!("string: {}", rf.version());            // "2.1.0"

let result = rf.eval("sum 1 2 3")?;
let n: i64 = result.try_into()?;
assert_eq!(n, 6);
```

The `eval` path internally calls `ray_eval_str`, then checks for
errors via `ffi::is_error` (mirroring the C `RAY_IS_ERR` macro),
extracts the engine's short tag with `ray_err_code` /
`ray_err_from_obj`, and frees the error object via `ray_error_free`
before returning `RayforceError::Ray { code, message, kind }`.

There is **no** `eval_obj` — the 1.0 entry point that took a
pre-parsed object is gone in 2.0; pass Rayfall source as a `&str`.

## Type conversions

### Rust → `RayObj`

`From` impls cover the primitive atom types and slice-construction
for the common numeric column kinds:

```rust
use rayforce::RayObj;

let i  = RayObj::from(42i64);                           // -RAY_I64 atom
let f  = RayObj::from(3.14f64);                         // -RAY_F64 atom
let b  = RayObj::from(true);                            // -RAY_BOOL atom
let s  = RayObj::from("hello");                         // -RAY_STR atom

let v1 = RayObj::from([1i64, 2, 3].as_slice());         //  RAY_I64 vector
let v2 = RayObj::from([1.1f64, 2.2, 3.3].as_slice());   //  RAY_F64 vector
```

### `RayObj` → Rust

`TryFrom<RayObj>` implementations for `i64`, `i32`, `f64`, `bool`,
`String` — each validates the type tag before reading the union:

```rust
use rayforce::{RayObj, RayforceError};

let obj: RayObj = 42i64.into();
let n: i64 = obj.try_into()?;

let obj: RayObj = "hello".into();
let s: String = obj.try_into()?;
```

## Helpers

Functions exposed by `rayforce::ffi`:

| Function | Purpose |
|----------|---------|
| `new_list()` | Build an empty `RAY_LIST` (`ray_list_new(0)`). |
| `new_vector(type_code, capacity)` | `ray_vec_new(type_code, capacity)`. |
| `push_to_list(&mut list, item)` | `ray_list_append`, adopting the COW return. |
| `list_get(&list, idx)` | `ray_list_get`, retaining the result. |
| `get_at_index(&list, idx)` | 1.0-compat alias for `list_get`. |
| `list_insert_at(&mut list, idx, item)` | `ray_list_insert_at`, adopting the COW return. |
| `new_symbol(s)` | `ray_sym_intern(s,len) → ray_sym(id)`. |
| `symbol_to_string(&obj)` | Resolve a symbol atom back to its interned name. |
| `new_date(days)` / `new_time(ms)` / `new_timestamp(ns)` | Atom constructors. |
| `new_table_from_pairs(iter)` | Build a `RAY_TABLE` via `ray_table_new` + `ray_table_add_col`. |
| `new_dict(keys, vals)` | `ray_dict_new` (consumes both inputs). |
| `set_global(name, value)` | `ray_env_set(ray_sym_intern(name), value)`. |
| `get_obj_raw_ptr(&obj)` | `read_data_ptr` — slice-aware data pointer. |
| `get_obj_len(&obj)` | Same as `obj.len()`. |
| `get_error_message(ptr)` | Extract a short tag from a raw error `ray_t*`. |

## Type codes

The 2.0 type tag set, straight from `rayforce.h`:

| Tag | Value | Meaning |
|-----|-------|---------|
| `RAY_LIST` | 0 | heterogeneous boxed list |
| `RAY_BOOL` | 1 | boolean |
| `RAY_U8` | 2 | unsigned byte |
| `RAY_I16` | 3 | int16 |
| `RAY_I32` | 4 | int32 |
| `RAY_I64` | 5 | int64 |
| `RAY_F32` | 6 | float32 |
| `RAY_F64` | 7 | float64 |
| `RAY_DATE` | 8 | days since 2000-01-01 |
| `RAY_TIME` | 9 | ms since midnight |
| `RAY_TIMESTAMP` | 10 | ns since epoch |
| `RAY_GUID` | 11 | 16-byte GUID |
| `RAY_SYM` | 12 | dictionary-encoded symbol column |
| `RAY_STR` | 13 | variable-length string column / atom |
| `RAY_INDEX` | 97 | accelerator index attached to a vector |
| `RAY_TABLE` | 98 | columnar table |
| `RAY_DICT` | 99 | key-value dictionary |
| `RAY_LAMBDA` | 100 | user-defined function |
| `RAY_UNARY` | 101 | unary builtin |
| `RAY_BINARY` | 102 | binary builtin |
| `RAY_VARY` | 103 | variadic builtin |
| `RAY_NULL` | 126 | typed null / void singleton |
| `RAY_ERROR` | 127 | error object |

Atoms carry the **negative** of the corresponding tag (e.g.
`-RAY_I64 = -5` for an i64 atom). Vectors / compounds use the
positive tag.

## Memory management

### Ownership

```rust
{
    let obj = RayObj::from(42i64);
    // `obj` owns one strong reference (rc == 1).
} // Drop calls ray_release; rc drops to 0; block returned to the heap.
```

### Refcount semantics

```rust
let a = RayObj::from(42i64);  // rc == 1
let b = a.clone();             // ray_retain → rc == 2
drop(a);                       // rc == 1
drop(b);                       // rc == 0 → freed
```

`ray_retain` / `ray_release` are no-ops for ARENA-flagged singletons
(`RAY_NULL_OBJ`, `RAY_OOM_OBJ`).

### Raw pointer access

```rust
let obj = RayObj::from(42i64);
let ptr: *mut rayforce::ray_t = obj.as_ptr();

// Adopt ownership of an existing rc=1 pointer:
let other = unsafe { RayObj::from_raw(ptr) };
// (Don't double-wrap the same pointer — that breaks the rc accounting.)
```

## Error handling

```rust
use std::ffi::CStr;
use rayforce::*;

let rf = Rayforce::new()?;
match rf.eval("(unbound-symbol)") {
    Ok(v) => println!("{v}"),
    Err(RayforceError::Ray { code, message, kind }) => {
        eprintln!("engine error [{code}]: {message} (kind={kind:?})");
    }
    Err(e) => eprintln!("{e}"),
}
```

The `code` field is the same short string returned by
`ray_err_code(err)` ("oom", "type", "range", …); `kind` is the matching
`ray_err_t` enum value when known.

## What was removed from the 1.0 FFI

These 1.0 helpers no longer exist (the underlying C symbols are gone
or moved to internal-only):

- `eval_obj(obj)` — pass Rayfall source to `Rayforce::eval` instead.
- `quote_obj(obj)` — no public 2.0 equivalent.
- `binary_set(...)` — replaced by `set_global(name, value)` (which
  calls `ray_env_set` + `ray_sym_intern`).
- `env_get_internal_function(name)` /
  `env_get_internal_name_by_function(...)` — internal function
  introspection isn't part of the 2.0 public API.
- `loadfn_from_file(...)` — dynamic-library loading isn't surfaced
  in 2.0.
- `clone_obj` / `drop_obj` / `rc_obj` — replaced by `ray_retain` /
  `ray_release` (and `RayObj::ref_count()` for inspection).
- `obj_fmt(obj, verbose)` — 2.0 formatter isn't a public C symbol.

If your code reaches into the FFI for one of these, see the
[Migrating from 1.0](../../index.md) section in the project README, or
file an issue if the gap is blocking.

## Safety considerations

The FFI layer contains `unsafe` code. Some rules:

1. **Don't double-adopt a pointer.** Each `RayObj::from_raw` takes
   ownership of one strong reference. Wrapping the same pointer
   twice unbalances the rc.
2. **Don't mix wrappers and raw `ray_release`.** Once you've handed
   a pointer to `RayObj::from_raw`, only the wrapper's `Drop` should
   release it.
3. **Validate type tags before casting.** The engine doesn't check;
   neither does the bindings layer until you go through a typed
   wrapper.
4. **Prefer the high-level API.** Drop down to FFI only when you
   need something the typed wrappers don't expose.

```rust
// Safe — typed wrapper handles refcount and validation:
let v: rayforce::RayVector<i64> = rayforce::RayVector::from_iter([1i64, 2, 3]);

// Unsafe — direct pointer manipulation:
unsafe {
    let ptr = v.as_ray_obj().as_ptr();
    // …make sure `ptr` outlives this scope.
}
```

## Where the bindings live

After a build, the bindgen-generated raw bindings end up at:

```
target/debug/build/rayforce-<hash>/out/bindings.rs
```

That file is the source of truth for every `ray_*` extern, every
`RAY_*` constant, and the `ray_t` / `ray_runtime_t` / `ray_err_t`
type definitions. Grep it whenever you need to confirm an FFI
signature or a struct field path.

## Next steps

- **[Types](types/scalars.md)** — high-level wrappers around `RayObj`.
- **[API Overview](overview.md)** — module map and trait reference.
