# :material-group: Group By

Group rows before aggregating with `.by(name)` (group by an existing column) or
`.by_expr(name, expr)` (group by a computed key). Each `.agg` is then evaluated
once per group.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{col, sum, avg, max, Runtime, Table, Value};
    Runtime::scope(|_rt| {
        let t = trades(); // sym / price / size — see the Overview
        Ok(())
    })?;
    ```

## Group by a column

`.by("sym")` produces one row per distinct symbol. The group key becomes the
first column of the result:

```rust
let r = t
    .select()
    .agg("total", sum(col("size")))
    .by("sym")
    .execute()?;
println!("{r}");
```

```text
┌──────┬──────────────────────────────┐
│ sym  │            total             │
│ SYM  │             I64              │
├──────┼──────────────────────────────┤
│ AAPL │ 40                           │
│ MSFT │ 70                           │
│ GOOG │ 40                           │
├──────┴──────────────────────────────┤
│ 3 rows (3 shown) 2 columns (2 shown)│
└─────────────────────────────────────┘
```

## Multiple aggregations per group

Stack `.agg` calls to compute several statistics in a single pass:

```rust
let r = t
    .select()
    .agg("total_size", sum(col("size")))
    .agg("avg_price",  avg(col("price")))
    .agg("hi",         max(col("price")))
    .by("sym")
    .execute()?;
```

```text
┌──────┬────────────┬───────────┬───────┐
│ sym  │ total_size │ avg_price │  hi   │
│ SYM  │    I64     │    F64    │  F64  │
├──────┼────────────┼───────────┼───────┤
│ AAPL │ 40         │ 105.0     │ 110.0 │
│ MSFT │ 70         │ 205.0     │ 210.0 │
│ GOOG │ 40         │ 300.0     │ 300.0 │
├──────┴────────────┴───────────┴───────┤
│ 3 rows (3 shown) 4 columns (4 shown)  │
└───────────────────────────────────────┘
```

## Group by a computed key

`.by_expr(name, expr)` groups by the result of any [expression](expressions.md),
naming the new key column. Here we bucket trades by whether they are expensive
(`price > 150`) and sum the notional in each bucket:

```rust
let r = t
    .select()
    .agg("notional", sum(col("price") * col("size")))
    .by_expr("expensive", col("price").gt(150.0))
    .execute()?;
println!("{r}");
```

```text
┌───────────┬─────────────────────────┐
│ expensive │        notional         │
│    B8     │           F64           │
├───────────┼─────────────────────────┤
│ false     │ 4300.0                  │
│ true      │ 26500.0                 │
├───────────┴─────────────────────────┤
│ 2 rows (2 shown) 2 columns (2 shown)│
└─────────────────────────────────────┘
```

!!! tip "Aggregation without `by` collapses to one row"
    An all-aggregation `select` with no `.by` returns a single summary row over
    the whole table — see [Select](select.md#aggregations).

Continue with [Order By](order-by.md) to sort grouped results.
