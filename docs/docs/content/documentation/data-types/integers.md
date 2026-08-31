# :material-numeric: Integers

Rayforce has four integer types: the unsigned byte `U8` plus three signed widths
`I16`, `I32`, and `I64`. Each has an atom constructor, a typed reader, and a
contiguous vector form.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Atoms

```rust
let a = Value::u8(255);
let b = Value::i16(-12345);
let c = Value::i32(2_000_000_000);
let d = Value::i64(i64::MAX);

assert_eq!(a.as_u8()?, 255);
assert_eq!(b.as_i16()?, -12345);
assert_eq!(c.as_i32()?, 2_000_000_000);
assert_eq!(d.as_i64()?, i64::MAX);
```

Readers are type-checked and return `rayforce::Result<T>`; asking for the wrong
width is an error rather than a silent cast:

```rust
assert!(Value::i64(5).as_i32().is_err());
```

## Null sentinels

The null of each signed integer is its `MIN` value. There is no unsigned null.

```rust
assert!(Value::i16(i16::MIN).is_atom_null());
assert!(Value::i32(i32::MIN).is_atom_null());
assert!(Value::i64(i64::MIN).is_atom_null());
assert!(!Value::i64(0).is_atom_null());
```

Pair this with `Option<T>` for ergonomic null handling (see
[Values & Conversions](values.md)):

```rust
assert_eq!(Value::i64(i64::MIN).extract::<Option<i64>>()?, None);
```

## Vectors

`Value::vec(&[T])` builds a homogeneous integer vector with a single bulk copy.
Annotate the element type — a bare literal defaults to `i32`.

```rust
let v = Value::vec(&[1i64, 2, 3, 4, 5]);
assert_eq!(v.len(), 5);
assert_eq!(v.as_slice::<i64>()?, &[1, 2, 3, 4, 5]);

let bytes = Value::vec(&[1u8, 2, 3]);
let shorts = Value::vec(&[-1i16, 0, 1]);
let words = Value::vec(&[10i32, 20]);
```

`as_slice::<T>()` is zero-copy and rejects a mismatched element type:

```rust
let v = Value::vec(&[1i64, 2, 3]);
assert!(v.as_slice::<i32>().is_err());
```

!!! tip "Annotate non-`i32` vectors"
    `Value::vec(&[1, 2, 3])` is an `I32` vector. For `I64` write
    `Value::vec(&[1i64, 2, 3])`; for `I16` write `Value::vec(&[1i16, 2, 3])`.

See [Vectors](vector.md) for indexing, mutation, slicing, and null bitmaps.
