# :material-format-text: String

A `String` value is a character vector — free-form text stored inline, byte for
byte. Unlike a [symbol](symbol.md) it is *not* interned, so strings suit unique
or high-cardinality text.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value, Str};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Atoms

```rust
let s = Value::string("a longer string value");
assert_eq!(s.as_string()?, "a longer string value");

// short and empty strings round-trip too
assert_eq!(Value::string("hi").as_string()?, "hi");
assert_eq!(Value::string("").as_string()?, "");
```

The empty string is the string null:

```rust
assert!(Value::string("").is_atom_null());
```

## The `Str` wrapper

Because a bare `&str` converts to a [symbol](symbol.md) through `ToValue`, the
`Str` wrapper exists to ask for a *string* atom explicitly in the conversion
path.

```rust
use rayforce::ToValue;
assert_eq!(Str("str").to_value().as_string()?, "str");
```

So `Value::string("x")` and `Str("x").to_value()` produce the same string atom,
while `"x".to_value()` produces a symbol.

## Vectors

`Value::str_vec` builds a vector of strings of any length.

```rust
let strs = Value::str_vec(&["hello", "a longer value here", ""]);
assert_eq!(strs.len(), 3);
assert_eq!(strs.get(0)?.as_string()?, "hello");
assert_eq!(strs.get(1)?.as_string()?, "a longer value here");
```

!!! tip "Choose symbol or string deliberately"
    Reach for a [symbol](symbol.md) when the same values repeat (keys, tickers,
    categories); reach for a string when text is mostly unique or arbitrarily
    long. See the comparison table on the [Symbol](symbol.md#symbol-vs-string)
    page.
