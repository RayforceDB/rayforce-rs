# :material-tag: Symbol

A `Symbol` is an *interned* string: the engine stores each distinct symbol once
and refers to it by a small integer id. Symbols are ideal for repeated, low-
cardinality labels — ticker codes, column names, enum-like tags — because
comparison and storage operate on the id, not the characters.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Atoms

```rust
let s = Value::sym("hello");
assert_eq!(s.as_sym()?, "hello"); // as_sym -> Result<String>
```

`as_sym` returns an owned `String`. A bare `&str` also converts to a symbol
through `ToValue`:

```rust
use rayforce::ToValue;
assert_eq!("AAPL".to_value().as_sym()?, "AAPL");
```

## Vectors

`Value::sym_vec` interns each entry and stores the ids contiguously.

```rust
let syms = Value::sym_vec(&["aaa", "bbb", "ccc"]);
assert_eq!(syms.len(), 3);
assert_eq!(syms.get(1)?.as_sym()?, "bbb");
```

Indexing is lossless and cheap: `get` boxes a symbol straight from its id with no
string round-trip.

```rust
let syms = Value::sym_vec(&["alpha", "beta"]);
assert_eq!(syms.get(0)?.as_sym()?, "alpha");
```

## Symbol vs String

Symbols and [strings](string.md) are different types with different trade-offs:

| | Symbol | String |
|---|--------|--------|
| Storage | interned id, deduplicated | character vector per value |
| Best for | repeated labels, keys | free-form, unique text |
| Atom from `&str` | `Value::sym` / `"x".to_value()` | `Value::string` / `Str("x").to_value()` |
| Reader | `as_sym` | `as_string` |

!!! tip "`&str` defaults to a symbol"
    Through the general `ToValue` path a bare `&str` becomes a *symbol*. Wrap it
    in [`Str`](string.md) when you want a string atom instead. (Inside query
    expressions the default flips to string — see [Values](values.md).)

Dictionary and table keys are typically symbol vectors — see [Dict](dict.md).
