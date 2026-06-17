# :material-filter: Where

Filter rows with `.filter(predicate)` on the [`Select`](select.md) builder. A
predicate is any [expression](expressions.md) that evaluates to a boolean mask.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{col, Runtime, Table, Value};
    let _rt = Runtime::new()?;
    let t = trades(); // sym / price / size — see the Overview
    ```

## A single filter

```rust
let r = t
    .select()
    .col("price")
    .filter(col("price").gt(150.0))
    .execute()?;
println!("{r}");
```

```text
┌─────────────────────────────────────┐
│                price                │
│                 F64                 │
├─────────────────────────────────────┤
│ 200.0                               │
│ 300.0                               │
│ 210.0                               │
├─────────────────────────────────────┤
│ 3 rows (3 shown) 1 columns (1 shown)│
└─────────────────────────────────────┘
```

## Multiple filters are AND-combined

Each `.filter` call narrows the result further — they combine with logical
**and**. The following keeps rows where `price > 150` **and** `sym == MSFT`:

```rust
let r = t
    .select()
    .filter(col("price").gt(150.0))
    .filter(col("sym").eq("MSFT"))
    .execute()?;
println!("{r}");
```

```text
┌──────┬───────┬──────────────────────┐
│ sym  │ price │         size         │
│ SYM  │  F64  │         I64          │
├──────┼───────┼──────────────────────┤
│ MSFT │ 200.0 │ 20                   │
│ MSFT │ 210.0 │ 50                   │
├──────┴───────┴──────────────────────┤
│ 2 rows (2 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

## Comparison methods

Remember that comparisons are **methods**, not Rust operators (see
[Expressions](expressions.md#comparison-methods)):

| Method | Keeps rows where |
|--------|------------------|
| `.eq(x)` / `.ne(x)` | equal / not equal |
| `.lt(x)` / `.le(x)` | less than / less-or-equal |
| `.gt(x)` / `.ge(x)` | greater than / greater-or-equal |

```rust
let cheap = t.select().filter(col("price").le(150.0)).execute()?;
```

!!! tip "Strings compare naturally"
    A bare `&str` in an expression is a string atom, so `col("sym").eq("MSFT")`
    matches a symbol column without any extra wrapping.

## `is_in`: membership

Keep rows whose value appears in a set:

```rust
let r = t
    .select()
    .filter(col("sym").is_in(Value::sym_vec(&["AAPL", "GOOG"])))
    .execute()?;
println!("{r}");
```

```text
┌──────┬───────┬──────────────────────┐
│ sym  │ price │         size         │
│ SYM  │  F64  │         I64          │
├──────┼───────┼──────────────────────┤
│ AAPL │ 100.0 │ 10                   │
│ AAPL │ 110.0 │ 30                   │
│ GOOG │ 300.0 │ 40                   │
├──────┴───────┴──────────────────────┤
│ 3 rows (3 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

## `like`: pattern match

Glob-match symbol or string columns. `*` matches any run of characters:

```rust
let r = t.select().filter(col("sym").like("M*")).execute()?;
// keeps the MSFT rows
```

## `within`: range masks

`.within([lo, hi])` produces an inclusive in-range mask:

```rust
let mask = col("price").within(Value::vec(&[150.0f64, 250.0]));
```

!!! warning "`within` is not a WHERE predicate (yet)"
    `.within` evaluates fine as an [expression mask](expressions.md#comparison-methods)
    and in [conditional aggregations](select.md#conditional-aggregation), but the
    WHERE compiler behind `select().filter(..)` does not lower it yet. To filter
    a numeric range in a `select`, combine two comparisons:

    ```rust
    let r = t
        .select()
        .filter(col("price").ge(150.0).and(col("price").le(250.0)))
        .execute()?;
    ```

## Logical combinations

Combine predicates inside one `.filter` with `.and` / `.or` (or the `&` / `|`
operators). These two are equivalent:

```rust
// method form
t.select().filter(col("price").gt(150.0).and(col("sym").eq("MSFT")));

// operator form
t.select().filter(col("price").gt(150.0) & col("sym").eq("MSFT"));
```

Use `.or` / `|` for disjunctions:

```rust
let r = t
    .select()
    .filter(col("sym").eq("AAPL").or(col("sym").eq("GOOG")))
    .execute()?;
```

Continue with [Group By](group-by.md) to aggregate the filtered rows.
