# :material-database-eye-outline: Query Guide

Rayforce gives you a small, composable query DSL for working with
[`Table`](../table/overview.md)s. You describe *what* you want with an
expression tree (`Expr`), the engine compiles it to a Rayfall program, and
`.execute()` runs it and hands you back a materialized `Table`.

!!! note "Assume a live runtime"
    Every example in this guide assumes a single live [`Runtime`](../../get-started/overview.md)
    and the trades table built below.

    ```rust
    use rayforce::{col, lit, sum, avg, count, min, max, Runtime, Table, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
    ```

## The query model

A query is built in three steps:

1. **Build an `Expr` tree.** [`col("price")`](expressions.md) references a column,
   `lit(v)` wraps a literal, and operators/methods combine them:
   `col("price") * col("size")`, `col("price").gt(150.0)`.
2. **Compile.** A builder (`Table::select()` / `Table::update()`) or a bare
   `Expr` lowers the tree to engine bytecode. You never call the compiler
   directly in normal use.
3. **Evaluate.** `.execute()` runs the compiled program and returns a
   `Result<Table>` (for builders) or a `Result<Value>` (for a standalone `Expr`).

```rust
// What you want: rows where price > 150, projecting sym and price.
let result = t
    .select()
    .cols(["sym", "price"])
    .filter(col("price").gt(150.0))
    .execute()?;
```

Nothing runs until `.execute()`. Up to that point you are only assembling a
description of the work.

## The example table

The whole guide reuses one small trades table — five rows of `sym`, `price`,
`size`:

```rust
fn trades() -> Table {
    let sym   = Value::sym_vec(&["AAPL", "MSFT", "AAPL", "GOOG", "MSFT"]);
    let price = Value::vec(&[100.0f64, 200.0, 110.0, 300.0, 210.0]);
    let size  = Value::vec(&[10i64, 20, 30, 40, 50]);
    Table::new(&["sym", "price", "size"], &[sym, price, size]).unwrap()
}

let t = trades();
println!("{t}");
```

```text
┌──────┬───────┬──────────────────────┐
│ sym  │ price │         size         │
│ SYM  │  F64  │         I64          │
├──────┼───────┼──────────────────────┤
│ AAPL │ 100.0 │ 10                   │
│ MSFT │ 200.0 │ 20                   │
│ AAPL │ 110.0 │ 30                   │
│ GOOG │ 300.0 │ 40                   │
│ MSFT │ 210.0 │ 50                   │
├──────┴───────┴──────────────────────┤
│ 5 rows (5 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

## The fluent builders

A `Table` exposes two query entry points, each returning a chainable builder:

| Entry point | Builder | Used for |
|-------------|---------|----------|
| `t.select()` | [`Select`](select.md) | projecting, filtering, grouping, ordering, aggregating |
| `t.update()` | [`Update`](update.md) | adding or rewriting columns in place |

The `Select` builder methods are:

| Method | Purpose |
|--------|---------|
| `.col(name)` / `.cols([names])` | project existing columns |
| `.agg(name, expr)` | add a computed/aggregated column |
| `.filter(pred)` | keep matching rows ([WHERE](where.md), AND-combined) |
| `.by(name)` / `.by_expr(name, expr)` | [group](group-by.md) before aggregating |
| `.order_by([cols], desc)` | [sort](order-by.md) the result |
| `.execute()` | compile and run, returning `Result<Table>` |

Row- and table-level operations live directly on `Table`:
[`insert_row`](insert.md) / [`insert_columns`](insert.md),
[`upsert_columns`](upsert.md), and the
[`inner_join` / `left_join` / `asof_join`](joins.md) family.

## Operators vs. comparison methods

This is the single most important rule of the DSL. **Arithmetic and logic use
Rust operator overloads; comparisons are methods.** Rust's `==`, `<`, `>` cannot
return a custom type, so they cannot produce an `Expr` — the DSL exposes them as
named methods instead.

| You want | Write | Not |
|----------|-------|-----|
| add | `col("a") + 1i64` | — |
| subtract / multiply / divide | `col("a") - col("b")`, `col("price") * col("size")` | — |
| modulo | `col("a") % 2i64` | — |
| logical and | `a.and(b)` or `a & b` | `a && b` |
| logical or | `a.or(b)` or `a \| b` | `a \|\| b` |
| negate / not | `-col("a")`, `!col("flag")` | — |
| equal | `col("sym").eq("MSFT")` | `col("sym") == "MSFT"` |
| not equal | `col("a").ne(0i64)` | `col("a") != 0` |
| less / less-or-equal | `col("a").lt(10i64)`, `.le(10i64)` | `col("a") < 10` |
| greater / greater-or-equal | `col("a").gt(10i64)`, `.ge(10i64)` | `col("a") > 10` |
| membership | `col("sym").is_in(set)` | — |
| pattern match | `col("sym").like("M*")` | — |

!!! tip "Bare `&str` is a string atom in expressions"
    Inside an expression, a plain string slice compiles to a *string* atom, so
    `col("sym").eq("MSFT")` matches symbol columns naturally. (In the general
    [`ToValue`](../data-types/values.md) path a `&str` is a *symbol* — the rule
    flips only inside the DSL.)

## A worked example: multi-aggregation select

Group the trades by `sym` and compute five statistics per group in one pass:

```rust
let stats = t
    .select()
    .agg("total_size", sum(col("size")))
    .agg("avg_price",  avg(col("price")))
    .agg("n",          count(col("price")))
    .agg("hi",         max(col("price")))
    .agg("lo",         min(col("price")))
    .by("sym")
    .execute()?;

println!("{stats}");
```

```text
┌──────┬────────────┬───────────┬─────┬───────┬───────┐
│ sym  │ total_size │ avg_price │  n  │  hi   │  lo   │
│ SYM  │    I64     │    F64    │ I64 │  F64  │  F64  │
├──────┼────────────┼───────────┼─────┼───────┼───────┤
│ AAPL │ 40         │ 105.0     │ 2   │ 110.0 │ 100.0 │
│ MSFT │ 70         │ 205.0     │ 2   │ 210.0 │ 200.0 │
│ GOOG │ 40         │ 300.0     │ 1   │ 300.0 │ 300.0 │
├──────┴────────────┴───────────┴─────┴───────┴───────┤
│ 3 rows (3 shown) 6 columns (6 shown)                │
└─────────────────────────────────────────────────────┘
```

## Where to next

- [Expressions](expressions.md) — `col`/`lit`, operators, and every method.
- [Select](select.md) — projection, computed columns, aggregation collapse.
- [Where](where.md) — filtering rows.
- [Group By](group-by.md) and [Order By](order-by.md).
- [Update](update.md), [Insert](insert.md), [Upsert](upsert.md), [Joins](joins.md).
