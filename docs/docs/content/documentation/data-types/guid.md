# :material-identifier: GUID

A `GUID` is a 128-bit globally-unique identifier stored as 16 raw bytes — handy
for keys that must be unique across machines without coordination.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value, Guid};
    let _rt = Runtime::new()?;
    ```

## Constructor and reader

`Value::guid` takes a `&[u8; 16]`; `as_guid` reads it back as `[u8; 16]`.

```rust
let bytes = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
    0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10,
];
let g = Value::guid(&bytes);
assert_eq!(g.as_guid()?, bytes);
```

## Null

The all-zero GUID is the null:

```rust
assert!(Value::guid(&[0u8; 16]).is_atom_null());
```

## The `Guid` wrapper

`Guid([u8; 16])` implements `ToValue` and `FromValue`, so a GUID flows through the
generic conversion API as well as the dedicated constructor.

```rust
use rayforce::ToValue;
let g = Guid([7u8; 16]);
assert_eq!(g.to_value().extract::<Guid>()?, g);
```

!!! tip "Two equivalent entry points"
    `Value::guid(&bytes)` and `Guid(bytes).to_value()` produce the same value.
    Use the wrapper when you are already going through `ToValue`/`extract`; use
    the constructor for a direct one-off.
