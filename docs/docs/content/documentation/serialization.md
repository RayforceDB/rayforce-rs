# :material-swap-horizontal: Serialization

Any [`Value`](data-types/values.md) can be turned into a byte buffer and back.
This is useful for persistence, caching, and moving data between processes — and
it is the **same wire format** the [IPC](ipc.md) layer uses on the network.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## :material-export: Serialize and deserialize

- `value.serialize() -> Result<Vec<u8>>` encodes a `Value` into bytes.
- `Value::deserialize(&[u8]) -> Result<Value>` decodes bytes back into a `Value`.

```rust
use rayforce::{Runtime, Value};
let _rt = Runtime::new()?;

let v = Value::i64(123456789);
let bytes = v.serialize()?;
let restored = Value::deserialize(&bytes)?;

assert_eq!(restored.as_i64()?, 123456789);
# Ok::<(), rayforce::RayError>(())
```

A round-trip preserves the value exactly — including its formatting:

```rust
use rayforce::{Runtime, Value};
let _rt = Runtime::new()?;

let v = Value::vec(&[7i64, 8, 9]);
let back = Value::deserialize(&v.serialize()?)?;
assert_eq!(v.format(), back.format());
# Ok::<(), rayforce::RayError>(())
```

## :material-atom: Atoms

Every atom type round-trips:

```rust
use rayforce::{Runtime, Value};
let _rt = Runtime::new()?;

fn roundtrip(v: &Value) -> rayforce::Result<Value> {
    Value::deserialize(&v.serialize()?)
}

assert_eq!(roundtrip(&Value::f64(123.456))?.as_f64()?, 123.456);
assert_eq!(roundtrip(&Value::sym("hello"))?.as_sym()?, "hello");
assert_eq!(roundtrip(&Value::string("a string"))?.as_string()?, "a string");
assert!(roundtrip(&Value::bool(true))?.as_bool()?);
# Ok::<(), rayforce::RayError>(())
```

## :material-vector-line: Vectors

Vectors round-trip element-for-element; numeric vectors read back zero-copy:

```rust
use rayforce::{Runtime, Value};
let _rt = Runtime::new()?;

let v = Value::vec(&[1i64, 2, 3, 4, 5]);
let back = Value::deserialize(&v.serialize()?)?;
assert_eq!(back.as_slice::<i64>()?, &[1, 2, 3, 4, 5]);

let syms = Value::sym_vec(&["a", "bb", "ccc"]);
let back = Value::deserialize(&syms.serialize()?)?;
assert_eq!(back.len(), 3);
assert_eq!(back.get(1)?.as_sym()?, "bb");
# Ok::<(), rayforce::RayError>(())
```

## :material-table: Tables

Tables (and lists and dicts) round-trip just the same. Serialize through the
table's underlying value and rebuild it on the other side:

```rust
use rayforce::{Runtime, Table, Value};
let _rt = Runtime::new()?;

let t = Table::new(
    &["sym", "px"],
    &[
        Value::sym_vec(&["AAPL", "MSFT"]),
        Value::vec(&[100.0f64, 200.0]),
    ],
)?;

let bytes = t.as_value().serialize()?;
let restored = Value::deserialize(&bytes)?.as_table()?;

assert_eq!(restored.shape(), (2, 2));
assert_eq!(restored.column("px")?.as_slice::<f64>()?, &[100.0, 200.0]);
# Ok::<(), rayforce::RayError>(())
```

!!! tip "Invalid input is an error, not a panic"
    `Value::deserialize` returns `Result`. Feeding it bytes that are not a valid
    payload yields an `Err` rather than undefined behavior:

    ```rust
    use rayforce::{Runtime, Value};
    let _rt = Runtime::new()?;
    assert!(Value::deserialize(&[0u8, 1, 2, 3, 4, 5, 6, 7]).is_err());
    # Ok::<(), rayforce::RayError>(())
    ```

## :material-arrow-right: See also

- [:material-network: IPC](ipc.md) — uses this exact format over the wire.
