# Container Types

Container types hold multiple values. RayforceDB provides homogeneous
typed columns (`RayVector<T>`) and heterogeneous boxed lists
(`RayList`), plus a string atom (`RayString`) and symbol-keyed
dictionaries (`RayDict`).

## RayVector&lt;T&gt;

Homogeneous typed columns. Backed by a contiguous typed buffer
(`ray_vec_new` / `ray_vec_from_raw`).

### Creating vectors

```rust
use rayforce::{RayVector, RaySymbol};

// From iterator (bulk-copies into a fresh vector via ray_vec_from_raw)
let prices: RayVector<i64> = RayVector::from_iter([100i64, 200, 300, 400]);
let ratios: RayVector<f64> = RayVector::from_iter([1.5, 2.0, 3.5]);

// Symbol vectors intern each name and store a 64-bit ID per element.
let symbols = RayVector::<RaySymbol>::from_iter(["AAPL", "GOOGL", "MSFT"]);
```

### Vector operations

```rust
use rayforce::RayVector;

let v: RayVector<i64> = RayVector::from_iter([10i64, 20, 30]);

println!("Length: {}", v.len());  // → 3
assert!(!v.is_empty());

// Access elements
assert_eq!(v.get(0), Some(10));
assert_eq!(v.get(2), Some(30));
assert_eq!(v.get(3), None);

// Read-only slice into the underlying buffer
let slice: &[i64] = v.as_slice();
println!("{slice:?}");
```

### Mutation

`RayVector::<i64>::set(idx, value)` and `<f64>::set(idx, value)` walk
the engine's COW path: `ray_vec_set` returns a (possibly new) owned
vector pointer, and the wrapper adopts it (releasing the old reference
when needed).

```rust
let mut v = RayVector::<i64>::from_iter([1i64, 2, 3]);
v.set(1, 999);
assert_eq!(v.as_slice(), &[1, 999, 3]);
```

### Supported element specialisations

| Element | Backed by | Mutation |
|---------|-----------|----------|
| `i64` | `ray_vec_from_raw(RAY_I64, …)` | `set(idx, i64)` |
| `f64` | `ray_vec_from_raw(RAY_F64, …)` | `set(idx, f64)` |
| `RaySymbol` | `ray_sym_vec_new(RAY_SYM_W64, …)` + `ray_vec_append` | — (read-only) |

`as_ray_obj() -> &RayObj` exposes the underlying `RayObj` for handing
off to functions like `RayTable::from_dict` that take ownership of
columns.

## RayList

Heterogeneous boxed list — each slot is a `ray_t*`. Built on
`ray_list_new` / `ray_list_append` / `ray_list_get`.

### Creating lists

```rust
use rayforce::RayList;

let mut list = RayList::new();
list.push(42i64);          // RayList::push takes anything that
list.push("hello");        //   `Into<RayObj>` — primitives, &str,
list.push(3.14f64);        //   slices, owned RayObj, etc.

// Or build from an iterator:
let list = RayList::from_iter([1i64, 2, 3]);
```

### List operations

```rust
let mut list = RayList::new();
list.push(1i64);
list.push(2i64);
list.push(3i64);

println!("Length: {}", list.len());      // → 3

if let Some(item) = list.get(0) {
    println!("First: {}", item);
}

// Iterate (each item is a freshly retained RayObj)
for item in list.iter() {
    println!("{}", item);
}
```

!!! note "List mutation"
    `RayList::push` adopts the COW return from `ray_list_append`; the
    wrapper releases the previous list reference automatically. There
    is no `RayList::set`/`pop` in the current bindings — file an issue
    or extend the wrapper if you need them.

## RayString

A `RAY_STR` atom: SSO under 7 bytes (inline in the header), pool-backed
for longer strings. New in 2.0 (1.0 used a `TYPE_C8` char vector).

```rust
use rayforce::RayString;

// From &str
let s = RayString::from("Hello, World!");

// From owned String
let owned = String::from("Rust");
let s = RayString::from(owned);
```

### String operations

```rust
let s = RayString::new("Hello");

println!("Length: {}", s.len());  // → 5
let rust_string: String = s.to_string();
println!("{}", s);                // → Hello
```

## RayDict

Symbol-keyed dictionary (`RAY_DICT`). Built via `ray_dict_new(keys,
vals)` (which **consumes** both inputs); lookups go through
`ray_dict_get` (returns owned).

### Creating dictionaries

`from_pairs` interns each name as a symbol and assembles the dict for
you:

```rust
use rayforce::{RayDict, RayI64, RayString, RayType};

let dict = RayDict::from_pairs([
    ("name",   RayString::new("Alice").ptr().clone()),
    ("age",    RayI64::new(30).ptr().clone()),
    ("active", true.into()),
])?;
```

### Dictionary operations

```rust
let dict = RayDict::from_pairs([
    ("a", RayI64::new(1).ptr().clone()),
    ("b", RayI64::new(2).ptr().clone()),
])?;

println!("Size: {}", dict.len());      // → 2
assert!(!dict.is_empty());

// Borrowed view into the keys / values columns:
let keys   = dict.keys();
let values = dict.values();

let dict2 = dict.clone();              // bumps refcount via ray_retain
```

### Accessing values

```rust
let dict = RayDict::from_pairs([
    ("price", RayI64::new(100).ptr().clone()),
])?;

// Lookup by string key — the binding interns the symbol for you and
// calls ray_dict_get, which returns owned references.
if let Some(value) = dict.get("price") {
    println!("Price: {}", value);
}
```

## Type reference

| Type | Tag | Element layout | Mutable |
|------|-----|----------------|---------|
| `RayVector<T>` | `RAY_I64` / `RAY_F64` / `RAY_SYM` | contiguous typed buffer | `set` for `<i64>`/`<f64>` |
| `RayList` | `RAY_LIST` (0) | `ray_t*` per slot | `push` |
| `RayString` | `-RAY_STR` (atom) | SSO + pool | — |
| `RayDict` | `RAY_DICT` | `[keys, vals]` pair | — (immutable wrapper) |

## Performance notes

### Vectors vs lists

**Use a typed `RayVector<T>` when:**
- All elements share the same type.
- You need columnar ops (the engine's analytics pipelines operate on
  these).
- Memory density and SIMD-friendliness matter.

**Use `RayList` when:**
- Elements have different types.
- You're building a record or a heterogeneous tuple.

```
RayVector<i64>:  [ i64 | i64 | i64 | i64 ]   ← contiguous
RayList:         [ ptr | ptr | ptr | ptr ]   ← pointers
```

## Common patterns

### Records as dicts

```rust
use rayforce::{RayDict, RayI64, RayType};

let record = RayDict::from_pairs([
    ("id",    RayI64::new(1).ptr().clone()),
    ("name",  rayforce::RayString::new("Alice").ptr().clone()),
    ("score", rayforce::RayF64::new(95.5).ptr().clone()),
])?;
```

### Tables as `RayList<RayVector<T>>`

```rust
use rayforce::{RayList, RayVector, RayType};

let mut columns = RayList::new();
columns.push(RayVector::<i64>::from_iter([1i64, 2, 3]).as_ray_obj().clone());
columns.push(RayVector::<f64>::from_iter([1.1, 2.2, 3.3]).as_ray_obj().clone());
```

(For an actual table use `RayTable::from_dict` — see
[Tables](table.md).)

## Next steps

- **[Table](table.md)** — `RayTable` reference.
- **[Scalars](scalars.md)** — scalar type reference.
- **[FFI](../ffi.md)** — low-level helpers (raw pointers, retain/release).
