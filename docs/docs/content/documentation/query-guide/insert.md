# :material-table-plus: Insert

`Table` offers two append operations: `insert_row` for a single record and
`insert_columns` for a batch. Both return a new `Result<Table>`.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{Runtime, Table, Value};
    let _rt = Runtime::new()?;
    let t = trades(); // sym / price / size — see the Overview
    ```

## `insert_row` — a single record

`insert_row(&[Value])` appends one row. The slice holds one **atom** per column,
in column order, with matching types (`SYM`, `F64`, `I64` here):

```rust
let r = t.insert_row(&[
    Value::sym("TSLA"),
    Value::f64(250.0),
    Value::i64(60),
])?;
println!("{r}");
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
│ TSLA │ 250.0 │ 60                   │
├──────┴───────┴──────────────────────┤
│ 6 rows (6 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

!!! warning "Order and type must match the schema"
    The slice has one entry per column, in declaration order, and each atom's
    type must match its column. A mismatch is a `RayError`.

## `insert_columns` — a batch of rows

`insert_columns(&[Value])` appends several rows at once. Each slice element is a
**vector** for one column; all vectors must share the same length (the number of
new rows):

```rust
let r = t.insert_columns(&[
    Value::sym_vec(&["TSLA", "NVDA"]),
    Value::vec(&[250.0f64, 400.0]),
    Value::vec(&[60i64, 70]),
])?;
println!("{r}");
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
│ TSLA │ 250.0 │ 60                   │
│ NVDA │ 400.0 │ 70                   │
├──────┴───────┴──────────────────────┤
│ 7 rows (7 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

Continue with [Upsert](upsert.md) for key-aware insert-or-update.
