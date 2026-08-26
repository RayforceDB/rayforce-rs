# :material-format-list-bulleted: List

A `List` is an ordered, **heterogeneous** collection of `Value`s (type code `0`).
Where a [vector](vector.md) holds one element type in a packed buffer, a list can
mix atoms, vectors, dicts, or even nested lists.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Construction

```rust
let items = [Value::i64(42), Value::sym("x"), Value::f64(3.5)];
let l = Value::list(&items);

assert!(l.is_list());
assert_eq!(l.len(), 3);
```

`Value::list` *retains* each element, so the originals stay valid after the list
is built:

```rust
let items = [Value::i64(42), Value::sym("x")];
let l = Value::list(&items);
assert_eq!(items[0].as_i64()?, 42); // still usable
```

## Access

Use `get` to pull element `i` back out as a `Value`, then read it with the
appropriate typed reader:

```rust
let l = Value::list(&[Value::i64(42), Value::sym("x"), Value::f64(3.5)]);
assert_eq!(l.get(0)?.as_i64()?, 42);
assert_eq!(l.get(1)?.as_sym()?, "x");
assert_eq!(l.get(2)?.as_f64()?, 3.5);
```

## Building incrementally

Start from `empty_list(capacity)` and append with `list_push`:

```rust
let mut l = Value::empty_list(2);
l.list_push(&Value::i64(1))?;
l.list_push(&Value::sym("two"))?;
assert_eq!(l.len(), 2);
assert_eq!(l.get(1)?.as_sym()?, "two");
```

!!! tip "List vs Vector"
    Reach for a [vector](vector.md) when every element is the same scalar type —
    it is packed, zero-copy, and far faster to scan. Reach for a list only when
    you genuinely need mixed types or nested structure.

A list of symbols paired with a list of values forms a [dict](dict.md).
