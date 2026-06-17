# :material-select-search: Select

`Table::select()` returns a `Select` builder. Chain projections, computed
columns, filters, grouping, and ordering, then call `.execute()` to get a
`Result<Table>`.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{col, sum, avg, count, min, max, Runtime, Table, Value};
    let _rt = Runtime::new()?;
    let t = trades(); // sym / price / size — see the Overview
    ```

## Builder methods

| Method | Purpose |
|--------|---------|
| `.col(name)` | project one existing column |
| `.cols([names])` | project several existing columns |
| `.agg(name, expr)` | add a computed or aggregated column |
| `.filter(pred)` | keep matching rows ([Where](where.md), AND-combined) |
| `.by(name)` / `.by_expr(name, expr)` | [group](group-by.md) before aggregating |
| `.order_by([cols], desc)` | [sort](order-by.md) the result |
| `.execute()` | compile and run, returning `Result<Table>` |

## Selecting columns

Project existing columns with `.col` (one) or `.cols` (several):

```rust
let r = t.select().cols(["sym", "price"]).execute()?;
println!("{r}");
```

```text
┌──────┬──────────────────────────────┐
│ sym  │            price             │
│ SYM  │             F64              │
├──────┼──────────────────────────────┤
│ AAPL │ 100.0                        │
│ MSFT │ 200.0                        │
│ AAPL │ 110.0                        │
│ GOOG │ 300.0                        │
│ MSFT │ 210.0                        │
├──────┴──────────────────────────────┤
│ 5 rows (5 shown) 2 columns (2 shown)│
└─────────────────────────────────────┘
```

## Select * (no projection)

A `select()` with no `.col`/`.cols`/`.agg` projects **every** column — the SQL
`SELECT *`. It is handy as the base for a pure `.filter`:

```rust
let r = t.select().filter(col("sym").eq("MSFT")).execute()?;
// all three columns, only the MSFT rows
```

## Computed columns

`.agg(name, expr)` names the result of any [expression](expressions.md). When
the expression is element-wise (not an aggregation), you get a derived column
with the same number of rows:

```rust
let r = t
    .select()
    .col("sym")
    .agg("notional", col("price") * col("size"))
    .execute()?;
```

## Aggregations

Use the [aggregation free functions or methods](expressions.md#aggregation-methods-and-free-functions)
inside `.agg`. Without `.by`, an **all-aggregation** select collapses to a
single summary row:

```rust
let r = t.select().agg("total", sum(col("size"))).execute()?;
println!("{r}");
```

```text
┌─────────────────────────────────────┐
│                total                │
│                 I64                 │
├─────────────────────────────────────┤
│ 150                                 │
├─────────────────────────────────────┤
│ 1 rows (1 shown) 1 columns (1 shown)│
└─────────────────────────────────────┘
```

Add [`.by`](group-by.md) to compute one row per group, and stack several
`.agg` calls to compute many statistics in one pass:

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

## Conditional aggregation

To aggregate only the rows that match a predicate, filter inside the
aggregation: `sum(col("x").filter(pred))`. The builder rewrites this into an
efficient masked sum automatically. Here, per symbol, we sum only the **buy**
side alongside the unconditional total:

```rust
let sym  = Value::sym_vec(&["AAPL", "AAPL", "MSFT", "MSFT"]);
let side = Value::sym_vec(&["B", "S", "B", "S"]);
let qty  = Value::vec(&[10i64, 5, 20, 8]);
let orders = Table::new(&["sym", "side", "qty"], &[sym, side, qty])?;

let r = orders
    .select()
    .agg("buy_qty",   sum(col("qty").filter(col("side").eq("B"))))
    .agg("total_qty", sum(col("qty")))
    .by("sym")
    .execute()?;
println!("{r}");
```

```text
┌──────┬─────────┬────────────────────┐
│ sym  │ buy_qty │     total_qty      │
│ SYM  │   I64   │        I64         │
├──────┼─────────┼────────────────────┤
│ AAPL │ 10      │ 15                 │
│ MSFT │ 20      │ 28                 │
├──────┴─────────┴────────────────────┤
│ 2 rows (2 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

## Filtering and ordering

`.filter` keeps matching rows (multiple calls are AND-combined) and
`.order_by` sorts the output. Both compose with everything above:

```rust
let r = t
    .select()
    .cols(["sym", "price"])
    .filter(col("price").gt(150.0))
    .order_by(["price"], true) // descending
    .execute()?;
```

See [Where](where.md) and [Order By](order-by.md) for the full treatment.
