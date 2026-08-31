# :material-swap-horizontal: Values & Conversions

`Value` is the universal handle for everything the engine holds. It is a cheap,
reference-counted pointer: `clone()` bumps a refcount, and the payload lives in
engine-owned memory. `Value` is `!Send`/`!Sync` — it stays on the thread that
owns the [`Runtime`](../../get-started/overview.md).

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value, ToValue, FromValue, Str, Guid};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## Inspecting a value

```rust
let v = Value::i64(42);
v.type_code(); // -5  (negative => atom)
v.abs_type();  //  5
v.is_atom();   // true
v.is_null();   // false
v.ref_count(); // current handle count
println!("{}", v.format()); // engine's textual form
```

## ToValue / FromValue

The `ToValue` trait converts a native Rust value into a `Value`; `FromValue`
goes the other way via `Value::extract`.

```rust
let v: Value = 42i64.to_value();        // or Value::new(42i64)
let n: i64 = v.extract::<i64>()?;
assert_eq!(n, 42);
```

`Value::new(x)` is shorthand for `x.to_value()`. The supported native types are
`bool`, `u8`, `i16`, `i32`, `i64`, `f32`, `f64`, `&str`, `String`, `Str`,
`Guid`, `Option<T>`, and `Value` itself.

## `&str` → symbol vs `Str(..)` → string

This is the one conversion nuance worth memorizing. In the `ToValue` path a bare
string slice becomes an **interned symbol**, while the `Str` wrapper produces a
**string atom** (a character vector).

```rust
assert_eq!("sym".to_value().as_sym()?, "sym");       // &str  -> SYMBOL
assert_eq!(Str("str").to_value().as_string()?, "str"); // Str  -> STRING
```

See [Symbols](symbol.md) and [Strings](string.md) for why the two differ.

!!! tip "In query expressions the rule flips"
    Inside the query DSL a bare `&str` compiles to a *string* atom so that
    `col("sym").eq("MSFT")` reads naturally. The symbol-by-default rule here
    applies to the general `ToValue` conversion path, not to expressions.

## Nullability with `Option<T>`

`Option<T>` is the idiomatic bridge to engine nulls. `None` serializes to a typed
null, and extracting an `Option<T>` turns a null back into `None`.

```rust
let none: Option<i64> = None;
assert!(none.to_value().is_null());

assert_eq!(Value::i64(7).extract::<Option<i64>>()?, Some(7));
assert_eq!(Value::i64(i64::MIN).extract::<Option<i64>>()?, None);
```

## Guid

A 16-byte globally-unique identifier is carried by the `Guid([u8; 16])` wrapper,
which implements both `ToValue` and `FromValue`.

```rust
let g = Guid([7u8; 16]);
assert_eq!(g.to_value().extract::<Guid>()?, g);
```

See the [GUID](guid.md) page for the dedicated `Value::guid` constructor.

## Constructed values match the engine

Values you build in Rust are bit-for-bit the same as ones the engine produces, so
they format identically:

```rust
use rayforce::eval;
assert_eq!(Value::i64(2).format(), eval("(+ 1 1)")?.format());
```
