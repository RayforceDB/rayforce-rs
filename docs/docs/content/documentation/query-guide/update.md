# :material-table-sync: Update

`Table::update()` returns an `Update` builder for adding or rewriting columns.
Set a column to the result of an [expression](expressions.md) with `.set`,
optionally restrict the rows touched with `.filter`, and run with `.execute()`.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{col, Runtime, Table, Value};
    let _rt = Runtime::new()?;
    let t = trades(); // sym / price / size — see the Overview
    ```

## Builder methods

| Method | Purpose |
|--------|---------|
| `.set(name, expr)` | set/add a column from an expression |
| `.filter(pred)` | restrict the update to matching rows |
| `.execute()` | compile and run, returning `Result<Table>` |

## Add a computed column

`.set` with a new column name appends a derived column. Here `notional` is
`price * size`:

```rust
let r = t
    .update()
    .set("notional", col("price") * col("size"))
    .execute()?;
println!("{r}");
```

```text
┌──────┬───────┬──────┬───────────────┐
│ sym  │ price │ size │   notional    │
│ SYM  │  F64  │ I64  │      F64      │
├──────┼───────┼──────┼───────────────┤
│ AAPL │ 100.0 │ 10   │ 1000.0        │
│ MSFT │ 200.0 │ 20   │ 4000.0        │
│ AAPL │ 110.0 │ 30   │ 3300.0        │
│ GOOG │ 300.0 │ 40   │ 12000.0       │
│ MSFT │ 210.0 │ 50   │ 10500.0       │
├──────┴───────┴──────┴───────────────┤
│ 5 rows (5 shown) 4 columns (4 shown)│
└─────────────────────────────────────┘
```

## Rewrite an existing column

Using an existing name overwrites that column in place. Here we apply a 10%
markup to `price`:

```rust
let r = t.update().set("price", col("price") * 1.1f64).execute()?;
```

## Conditional update

Add `.filter` to update only the matching rows. Other rows keep their original
value:

```rust
let r = t
    .update()
    .set("price", col("price") * 1.1f64)
    .filter(col("sym").eq("MSFT"))
    .execute()?;
```

Continue with [Insert](insert.md) to append rows.
