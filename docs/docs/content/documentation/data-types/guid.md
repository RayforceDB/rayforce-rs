# :material-identifier: GUID

A `GUID` is a 128-bit globally-unique identifier stored as 16 raw bytes — handy
for keys that must be unique across machines without coordination.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value, Guid};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
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

## Vectors { #vectors }

A GUID vector is contiguous 16-byte cells, so `[u8; 16]` is a `VecElem`:
`Value::guid_vec(&[[u8; 16]])` builds one with a single `memcpy` (it is
`Value::vec` for that element), and `guid_slice()` borrows the engine buffer as
`&[[u8; 16]]` without copying — the way to move a column of keys in and out in
bulk.

```rust
let a = [0x11u8; 16];
let b = [0x22u8; 16];
let v = Value::guid_vec(&[a, [0u8; 16], b]);
assert_eq!(v.len(), 3);
assert_eq!(v.guid_slice()?, &[a, [0u8; 16], b]);
```

The all-zero cell is the null from construction, as any sentinel handed to
`Value::vec` is (see [Vectors — Nulls](vector.md#nulls)); boxed reads give the
null singleton, and `Option<Guid>` sees it:

```rust
assert!(v.is_null_at(1));
assert_eq!(v.to_vec::<Option<Guid>>()?, vec![Some(Guid(a)), None, Some(Guid(b))]);
```

`set` and `push` take a `[u8; 16]`:

```rust
let mut v = Value::guid_vec(&[a]);
v.push(b)?;
assert_eq!(v.guid_slice()?, &[a, b]);
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
