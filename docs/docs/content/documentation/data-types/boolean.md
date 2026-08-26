# :material-toggle-switch: Boolean

The `B8` type is a one-byte boolean. Atoms are built with `Value::bool`, vectors
with `Value::bool_vec`.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Atoms

```rust
assert!(Value::bool(true).as_bool()?);
assert!(!Value::bool(false).as_bool()?);
```

## Vectors

```rust
let b = Value::bool_vec(&[true, false, true]);
assert_eq!(b.len(), 3);
```

### Reading the buffer

Booleans are stored one per byte. `bool_slice()` returns the raw `&[u8]`
backing store, where `1` is `true` and `0` is `false` — zero-copy, no allocation.

```rust
let b = Value::bool_vec(&[true, false, true]);
assert_eq!(b.bool_slice()?, &[1u8, 0, 1]);
```

To get a `Value` per element instead, use `get`:

```rust
let b = Value::bool_vec(&[true, false]);
assert!(b.get(0)?.as_bool()?);
assert!(!b.get(1)?.as_bool()?);
```

!!! tip "`bool_slice` is for `B8` vectors only"
    `bool_slice()` returns `&[u8]` because that is the on-wire layout; it is not
    the same as `as_slice::<u8>()` on a `U8` vector. Use it when you want to scan
    or memcpy a boolean column without boxing each element.

See [Vectors](vector.md) for indexing, slicing, and mutation that apply to every
vector type.
