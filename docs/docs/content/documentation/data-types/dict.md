# :material-key-variant: Dict

A `Dict` maps a keys collection to a values collection (type code `99`). Keys are
typically a [symbol vector](symbol.md) and values any `Value` — a vector, a list,
or another dict. A dict whose values are equal-length vectors is the building
block of a [table](../table/overview.md).

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## Construction

`Value::dict(keys, values)` takes two `Value`s — the keys and the values.

```rust
let keys = Value::sym_vec(&["a", "b", "c"]);
let vals = Value::vec(&[1i64, 2, 3]);
let d = Value::dict(keys, vals);

assert!(d.is_dict());
assert_eq!(d.dict_len()?, 3);
```

## Accessing keys and values

```rust
let keys = Value::sym_vec(&["a", "b", "c"]);
let vals = Value::vec(&[1i64, 2, 3]);
let d = Value::dict(keys, vals);

// The whole keys / values collections
assert_eq!(d.dict_keys()?.get(0)?.as_sym()?, "a");
assert_eq!(d.dict_values()?.as_slice::<i64>()?, &[1, 2, 3]);
```

## Lookup

`dict_get` returns `Result<Option<Value>>`: `Ok(Some(..))` on a hit, `Ok(None)`
when the key is absent.

```rust
let keys = Value::sym_vec(&["a", "b", "c"]);
let vals = Value::vec(&[1i64, 2, 3]);
let d = Value::dict(keys, vals);

let got = d.dict_get(&Value::sym("b"))?;
assert_eq!(got.unwrap().as_i64()?, 2);

assert!(d.dict_get(&Value::sym("missing"))?.is_none());
```

!!! tip "Dict is the path to Table"
    A dict of symbol keys → equal-length vector values is exactly a column-major
    table. When you need named columns with row semantics, build a
    [`Table`](../table/overview.md) instead of querying a raw dict.
