# :material-function-variant: Functions (Lambda)

A `Fn` is a user-defined **lambda function** (type code `100`, `RAY_LAMBDA`). It
wraps a compiled function object built from Rayfall source — for example
`(fn [x] (* x x))` — and can be applied either directly or inside a
[query](../query-guide/select.md).

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Fn, Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## Construction

Compile a lambda from its Rayfall source with `Fn::new`. The source must be a
`(fn …)` expression:

```rust
let square = Fn::new("(fn [x] (* x x))")?;
assert_eq!(square.source(), Some("(fn [x] (* x x))"));
```

Passing anything that is not a `(fn …)` expression is an error:

```rust
assert!(Fn::new("(+ 1 2)").is_err());
```

## Direct call

`call` applies the lambda and evaluates immediately. It works on a scalar…

```rust
let square = Fn::new("(fn [x] (* x x))")?;
assert_eq!(square.call(&[Value::i64(5)])?.as_i64()?, 25);
```

…or element-wise over a vector:

```rust
let square = Fn::new("(fn [x] (* x x))")?;
let out = square.call(&[Value::vec(&[2i64, 3, 4])])?;
assert_eq!(out.as_slice::<i64>()?, &[4, 9, 16]);
```

Multiple arguments are passed in order:

```rust
let add = Fn::new("(fn [x y] (+ x y))")?;
assert_eq!(add.call(&[Value::i64(10), Value::i64(32)])?.as_i64()?, 42);
```

## Applying inside a query

`apply` turns a lambda into an [`Expr`](../query-guide/expressions.md), so it can
feed a projection, a predicate, or an aggregation. It returns a `Result` because
the lambda is bound into the global environment on first use (the query DAG
compiler resolves it by name):

```rust
use rayforce::{col, Fn, Table, Value};

let t = Table::new(
    &["id", "value"],
    &[Value::sym_vec(&["a", "b", "c"]), Value::vec(&[2i64, 3, 4])],
)?;

let square = Fn::new("(fn [x] (* x x))")?;
let out = t
    .select()
    .col("id")
    .agg("squared", square.apply([col("value")])?)
    .execute()?;

assert_eq!(out.column("squared")?.as_slice::<i64>()?, &[4, 9, 16]);
```

Because `apply` yields a plain `Expr`, you can wrap it like any other
expression — for instance in an aggregate:

```rust
use rayforce::{col, sum, Fn, Table, Value};

let t = Table::new(&["value"], &[Value::vec(&[2i64, 3, 4])])?;
let square = Fn::new("(fn [x] (* x x))")?;
let out = t
    .select()
    .agg("sum_sq", sum(square.apply([col("value")])?))
    .execute()?;

assert_eq!(out.column("sum_sq")?.get(0)?.as_i64()?, 29); // 4 + 9 + 16
```

## Lambdas from a loaded file

A lambda defined in a Rayfall file and bound by name — e.g. after
`(load "prelude.rfl")` — is fetched with `Fn::from_global`, not `Fn::new`
(`new` compiles source; `from_global` looks up an existing binding):

```rust
use rayforce::{eval, Fn, Value};

eval("(load \"prelude.rfl\")")?; // defines a `sq` lambda in the global env
let sq = Fn::from_global("sq")?;
assert_eq!(sq.call(&[Value::i64(9)])?.as_i64()?, 81);
```

`from_global` also reuses the existing name when the lambda is later used in a
query via `apply`, so no extra binding is generated.

!!! tip "`new` vs `from_global` vs `from_value`"
    - `Fn::new("(fn …)")` — compile **source** into a new lambda.
    - `Fn::from_global("name")` — take a lambda already **bound in the env**
      (including anything pulled in by `(load …)`).
    - `Fn::from_value(v)` — wrap an existing lambda `Value`, e.g. one returned by
      [`eval`](values.md).

## Introspection

`source` returns the original text (when built with `Fn::new`), and a `Fn`
renders as that source rather than the engine's opaque `lambda`:

```rust
let square = Fn::new("(fn [x] (* x x))")?;
assert_eq!(format!("{square}"), "(fn [x] (* x x))");
```
