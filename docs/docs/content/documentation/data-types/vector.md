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

## Null bitmap { #null-bitmap }

Vectors track nulls in a **separate bitmap attribute**, not by scanning payload
bytes. A buffer that coincidentally contains a sentinel value (e.g. `i64::MIN`)
is *not* considered null until the element is explicitly marked.

```rust
// A coincidental sentinel is NOT null on its own:
let raw = Value::vec(&[1i64, i64::MIN, 3]);
assert!(!raw.is_null_at(1));

// Explicitly marking an element is the supported path:
let mut v = Value::vec(&[1i64, 2, 3]);
v.set_null(1, true)?;
assert!(v.is_null_at(1));
assert!(v.get(1)?.is_null());
assert_eq!(v.get(0)?.as_i64()?, 1); // neighbors untouched
```

!!! note "Why the bitmap matters"
    Decoupling nulls from payload bytes lets the engine store any in-band value
    without ambiguity and lets a column be marked null in O(1) without touching
    the data. Test for nulls with `is_null_at(idx)` rather than comparing against
    a sentinel.

## Constructed vectors match the engine

```rust
use rayforce::eval;
let v = Value::vec(&[0i64, 1, 2, 3, 4]);
assert_eq!(v.format(), eval("(til 5)")?.format()); // "0 1 2 3 4"
```

For heterogeneous collections, see [Lists](list.md); for keyed data, see
[Dicts](dict.md).
