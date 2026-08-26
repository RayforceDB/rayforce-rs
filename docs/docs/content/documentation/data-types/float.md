# :material-decimal: Float

Rayforce provides two IEEE-754 floating-point types: single-precision `F32` and
double-precision `F64`.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Atoms

```rust
let a = Value::f32(1.25);
let b = Value::f64(3.5);

assert_eq!(a.as_f32()?, 1.25);
assert_eq!(b.as_f64()?, 3.5);
```

Readers are type-checked: an `F64` will not read back as an `F32` or an `I64`.

```rust
assert!(Value::f64(5.0).as_i64().is_err());
```

## NaN is null

The float null is `NaN`. Constructing a float atom from `NaN` produces a null,
which round-trips through `Option<f64>`:

```rust
assert!(Value::f64(f64::NAN).is_atom_null());

let none: Option<f64> = None;
assert!(none.to_value().is_null());
```

!!! note "Comparisons follow IEEE semantics"
    Because the null *is* `NaN`, it is never equal to itself. Use
    `is_atom_null()` (atoms) or `is_null_at(idx)` (vectors) to test for nulls
    rather than an equality check.

## Vectors

```rust
let f = Value::vec(&[1.0f64, 2.0, 3.0]);
assert_eq!(f.as_slice::<f64>()?, &[1.0, 2.0, 3.0]);

let g = Value::vec(&[1.5f32, 2.5]);
assert_eq!(g.as_slice::<f32>()?, &[1.5, 2.5]);
```

`as_slice::<T>()` is zero-copy and enforces the element type, so an `F64` vector
cannot be read as `F32`. A literal like `1.0` defaults to `f64`; write `1.0f32`
for single precision.

See [Vectors](vector.md) for the full set of vector operations.
