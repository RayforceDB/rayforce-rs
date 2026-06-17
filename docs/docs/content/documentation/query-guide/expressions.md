# :material-function-variant: Expressions

An `Expr` is a node in a query tree. You build one from columns and literals,
combine them with operators and methods, and either hand the tree to a
[builder](select.md) or evaluate it directly with `Expr::execute()`.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{col, lit, sum, avg, count, min, max, Expr, Operation, Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## `col` and `lit`

There are two leaves in every expression tree:

- `col(name)` — a reference to a column (inside a builder) or a global binding
  (when evaluated standalone).
- `lit(value)` — a literal `Value` folded into the tree.

```rust
let by_name    = col("price");                 // resolved at execution time
let literal    = lit(Value::vec(&[1i64, 2, 3]));
```

## `IntoExpr`: what counts as an operand

Most methods and the right-hand side of every operator accept
`impl IntoExpr`, so you rarely need to wrap scalars by hand. `IntoExpr` is
implemented for:

- `Expr` and `Value`
- the numeric primitives `bool`, `u8`, `i16`, `i32`, `i64`, `f32`, `f64`
- `&str` and `String` — **as string atoms** (see the rule below)
- `Str(s)` for an explicit string atom

```rust
let e = col("a") + 1i64;          // i64 lifted via IntoExpr
let e = col("sym").eq("MSFT");    // &str lifted as a string atom
```

!!! tip "Bare `&str` is a string atom in expressions"
    Inside the DSL a plain `&str` compiles to a **string** atom, which is why
    `col("sym").eq("MSFT")` compares correctly against a symbol column. This is
    the opposite of the general [`ToValue`](../data-types/values.md) path, where
    a `&str` is a *symbol*. The flip is deliberate and only happens in
    expressions.

## Operators (overloaded)

Arithmetic and logic are Rust operator overloads. Each returns a new `Expr`, and
the right operand is any `impl IntoExpr`.

| Operator | Meaning |
|----------|---------|
| `+` `-` `*` `/` `%` | add, subtract, multiply, divide, modulo |
| `&` | logical / element-wise **and** |
| `\|` | logical / element-wise **or** |
| unary `-` | negate |
| unary `!` | not |

```rust
let v = lit(Value::vec(&[1i64, 2, 3]));
let e = (v - 1i64) * 2i64;
assert_eq!(e.execute()?.as_slice::<i64>()?, &[0, 2, 4]);
```

## Comparison methods

Rust's comparison operators cannot return a custom type, so comparisons are
**methods**. Each yields a boolean mask `Expr`.

| Method | Meaning |
|--------|---------|
| `.eq(x)` | equal |
| `.ne(x)` | not equal |
| `.lt(x)` / `.le(x)` | less than / less-or-equal |
| `.gt(x)` / `.ge(x)` | greater than / greater-or-equal |
| `.like(pattern)` | glob match (`"M*"`) on symbols/strings |
| `.is_in(set)` | membership in a vector |
| `.within(range)` | inside an inclusive `[lo, hi]` range |
| `.and(e)` / `.or(e)` | combine masks (same as `&` / `\|`) |
| `.filter(mask)` | keep only elements where the mask is true |

```rust
// A comparison produces a bool mask.
let mask = lit(Value::vec(&[1i64, 2, 3, 4])).gt(2i64);
let r = mask.execute()?;
assert!(!r.get(0)?.as_bool()?);   // 1 > 2  -> false
assert!( r.get(3)?.as_bool()?);   // 4 > 2  -> true
```

```rust
// Combine masks with .and / & — both compile to the same tree.
rayforce::set_global("v", &Value::vec(&[1i64, 5, 10, 15]))?;
let e = col("v").gt(2i64).and(col("v").lt(12i64));
// equivalently: col("v").gt(2i64) & col("v").lt(12i64)
let r = e.execute()?;                 // -> [false, true, true, false]
```

```rust
// within yields the in-range mask.
rayforce::set_global("p", &Value::vec(&[100i64, 200, 300, 150]))?;
let r = col("p").within(Value::vec(&[150i64, 250]))?.execute()?;
// -> [false, true, false, true]
```

!!! note "`within` in expressions, not in `select().filter`"
    `.within` evaluates fine as a standalone expression mask. The WHERE
    compiler used by [`select().filter(..)`](where.md) does **not** lower
    `within` yet — filter on rows with `.is_in`, `.like`, or the comparison
    methods, and reach for `within` when computing masks in expressions or
    [conditional aggregations](select.md#conditional-aggregation).

## Aggregation methods (and free functions)

Aggregations exist in two equivalent spellings: a free function and a method.
`sum(col("x"))` and `col("x").sum()` build the same tree.

| Free fn / method | Result |
|------------------|--------|
| `sum` | sum |
| `avg` | mean |
| `count` | element count |
| `min` / `max` | extremes |
| `first` / `last` | first / last element |
| `median` | median |
| `distinct` | unique values |
| `.deviation()` / `.std()` | (methods) deviation / standard deviation |

```rust
let data = Value::vec(&[1i64, 2, 3, 4]);

// Free-function form
assert_eq!(sum(lit(data.clone())).execute()?.as_i64()?, 10);

// Method form — identical
assert_eq!(lit(data).sum().execute()?.as_i64()?, 10);
```

## Math methods

Rounding helpers are methods on `Expr`:

| Method | Meaning |
|--------|---------|
| `.ceil()` | round up |
| `.floor()` | round down |
| `.round()` | round to nearest |

## Standalone evaluation with `Expr::execute()`

Outside a table, `col(name)` resolves against the engine's globals. Bind a
value, reference it by name, and evaluate:

```rust
let xs = Value::vec(&[10i64, 20, 30]);
rayforce::set_global("xs", &xs)?;

let e = col("xs") + 5i64;
assert_eq!(e.execute()?.as_slice::<i64>()?, &[15, 25, 35]);

assert_eq!(sum(col("xs")).execute()?.as_i64()?, 60);
```

`Expr::execute()` returns a `Result<Value>`. The related `Expr::compile()`
returns the compiled `Value` tree (useful for inspection); inside builders this
happens for you.

```rust
let compiled = col("price").gt(100i64).compile();
assert!(compiled.is_list());        // (> price 100): op, name-ref, literal
assert_eq!(compiled.len(), 3);
```

## Building op nodes directly

For operations without a dedicated helper, assemble a node from an `Operation`
and its operands. This is what the masked-sum rewrite uses under the hood:

```rust
rayforce::set_global("c", &Value::vec(&[10i64, 20, 30, 40]))?;
rayforce::set_global("m", &Value::bool_vec(&[true, false, true, false]))?;

// sum(filter(c, m)) keeps [10, 30] -> 40
let e = Expr::op(Operation::Sum, vec![col("c").filter(col("m"))]);
assert_eq!(e.execute()?.as_i64()?, 40);
```

Continue with [Select](select.md) to put these expressions to work on a table.
