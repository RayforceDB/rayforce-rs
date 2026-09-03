# :material-vector-line: Vectors

A vector is a homogeneous, contiguous buffer of one element type — the workhorse
of a columnar engine. Rayforce vectors live in engine-owned memory, and the Rust
bindings give you **zero-copy** access to that memory wherever possible. This is
the page to read if you care about performance.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

The numeric element types are captured by the `VecElem` trait:
`u8`, `i16`, `i32`, `i64`, `f32`, `f64`. (Booleans, symbols, strings, and
temporals have their own constructors — see [Boolean](boolean.md),
[Symbol](symbol.md), [String](string.md), [Temporal](temporal.md).)

## Construction — a single memcpy

`Value::vec(&[T])` builds a vector with one bulk copy of the whole slice, not an
element-by-element loop.

```rust
let v = Value::vec(&[1i64, 2, 3, 4, 5]);
assert_eq!(v.len(), 5);
```

!!! tip "Annotate the element type"
    A bare integer literal is `i32`, and a bare float literal is `f64`. For other
    widths annotate the first element: `Value::vec(&[1i64, 2, 3])`,
    `Value::vec(&[1.5f32, 2.5])`.

## Zero-copy reads — `as_slice`

`as_slice::<T>()` borrows the engine buffer directly as `&[T]`; nothing is
copied. The element type is checked at the boundary, so a mismatch is an error
rather than a reinterpretation of bytes.

```rust
let v = Value::vec(&[1i64, 2, 3, 4, 5]);
assert_eq!(v.as_slice::<i64>()?, &[1, 2, 3, 4, 5]);
assert!(v.as_slice::<i32>().is_err()); // wrong element type rejected
```

## Boxed access — `get`, `iter`, `to_vec`

When you want owned values or per-element `Value`s:

```rust
let v = Value::vec(&[10i64, 20, 30]);

assert_eq!(v.get(0)?.as_i64()?, 10);
assert!(v.get(3).is_err()); // out of range

let owned: Vec<i64> = v.to_vec()?;             // FromValue per element
assert_eq!(owned, vec![10, 20, 30]);

let via_iter: Vec<i64> =
    v.iter().map(|r| r.unwrap().as_i64().unwrap()).collect();
assert_eq!(via_iter, vec![10, 20, 30]);
```

Prefer `as_slice` for numeric scans; reach for `get`/`iter`/`to_vec` when you
need `Value`s or a `Vec<T>`.

## Mutation — `set`, `push`

```rust
let mut v = Value::vec(&[1i64, 2, 3]);
v.set(1, 99i64)?;
assert_eq!(v.as_slice::<i64>()?, &[1, 99, 3]);

v.push(4i64)?;
assert_eq!(v.as_slice::<i64>()?, &[1, 99, 3, 4]);
```

Both are type-checked, and an out-of-range `set` fails without corrupting or
leaking the vector:

```rust
let mut v = Value::vec(&[1i64, 2, 3]);
assert!(v.set(0, 1.0f64).is_err()); // wrong element type
assert!(v.set(5, 9i64).is_err());   // out of range
assert_eq!(v.as_slice::<i64>()?, &[1, 2, 3]); // unchanged
```

!!! warning "The `i64` literal-suffix gotcha"
    `set` and `push` infer the element type from the literal you pass. On an
    `I64` vector, `v.push(4)` passes an `i32` and fails the type check — you must
    write `v.push(4i64)` and `v.set(1, 99i64)`. The same applies to `i16`,
    `f32`, etc.

## Slicing and concatenation

```rust
let v = Value::vec(&[1i64, 2, 3, 4, 5]);
let s = v.slice(1, 3)?; // offset 1, length 3
assert_eq!(s.as_slice::<i64>()?, &[2, 3, 4]);

let a = Value::vec(&[1i64, 2]);
let b = Value::vec(&[3i64, 4]);
let c = a.concat(&b)?;
assert_eq!(c.as_slice::<i64>()?, &[1, 2, 3, 4]);
```

## Nulls { #nulls }

Nulls live **in-band**: an element is null when it holds its type's sentinel —
`i16::MIN`, `i32::MIN`, `i64::MIN`, `NaN`, the all-zero GUID, the empty symbol,
the empty string. For the fixed-width types the engine also keeps a `HAS_NULLS`
attribute as a fast-path hint and checks it first; `Value::vec` raises it when
the buffer it is handed already contains a sentinel, and `set_null` raises it
when marking an element. Symbol and string vectors need no hint: the empty
value *is* the null.

```rust
// A buffer carrying a sentinel is null from construction:
let raw = Value::vec(&[1i64, i64::MIN, 3]);
assert!(raw.is_null_at(1));
assert!(raw.get(1)?.is_null());
assert_eq!(raw.as_slice::<i64>()?, &[1, i64::MIN, 3]); // payload untouched
assert_eq!(raw.to_vec::<Option<i64>>()?, vec![Some(1), None, Some(3)]);

// Marking an element null writes the sentinel and raises the hint:
let mut v = Value::vec(&[1i64, 2, 3]);
v.set_null(1, true)?;
assert!(v.is_null_at(1));
assert!(v.get(1)?.is_null());
assert_eq!(v.get(0)?.as_i64()?, 1); // neighbors untouched
```

An empty symbol or string element is reported by `is_null_at`, but `get` hands
back the empty atom rather than the null singleton, so plain `String` extraction
keeps working and `Option<String>` sees the null:

```rust
let strs = Value::str_vec(&["hello", ""]);
assert!(strs.is_null_at(1));
assert_eq!(strs.get(1)?.as_string()?, "");
assert!(strs.get(1)?.is_atom_null());
assert_eq!(
    strs.to_vec::<Option<String>>()?,
    vec![Some("hello".to_string()), None]
);
```

!!! note "Clearing a null, and the hint's blind spot"
    `set_null(idx, false)` is a no-op — the engine cannot know the value the
    sentinel replaced — so overwrite the element with `set` to un-null it.
    Conversely, a numeric sentinel written through `set` or `push` does not
    raise the hint, so `is_null_at` will not report it; the boxed atom still
    answers `is_atom_null()` and extracts as `None`. Prefer `set_null` for
    writing nulls, and `is_null_at(idx)` or `Option<T>` extraction for reading
    them, over hand-written sentinel comparisons.

## Constructed vectors match the engine

```rust
use rayforce::eval;
let v = Value::vec(&[0i64, 1, 2, 3, 4]);
assert_eq!(v.format(), eval("(til 5)")?.format()); // "0 1 2 3 4"
```

For heterogeneous collections, see [Lists](list.md); for keyed data, see
[Dicts](dict.md).
