# :octicons-table-24: Table

A `Table` is a named collection of equal-length column vectors — a schema
(the ordered column names) plus the data (one typed [:material-vector-line: Vector](../data-types/vector.md)
per column). Every column carries its own [data type](../data-types/overview.md),
and every column must have the same number of rows.

```rust
use rayforce::{Runtime, Table, Value};

Runtime::scope(|_rt| {
    let t = Table::new(
        &["sym", "price", "size"],
        &[
            Value::sym_vec(&["AAPL", "MSFT", "GOOG"]),
            Value::vec(&[101.5f64, 202.0, 303.25]),
            Value::vec(&[10i64, 20, 30]),
        ],
    )?;
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## A newtype over `Value`

`Table` is a thin newtype wrapper around a [`Value`](../data-types/values.md)
whose underlying handle is a table. This makes the two cheap to move between:
borrow the inner value with `as_value()`, take ownership with `into_value()`,
or turn a table-shaped `Value` back into a `Table` with `as_table()` /
`Table::from_value`. See [Create a Table](create.md#interoperating-with-value)
for the full set of conversions.

!!! warning "Use `nrows()` for the row count"
    Because `Table` wraps a `Value`, calling `Value::len()` on a table returns
    the number of internal slots (`2`), **not** the row count. Always use
    [`Table::nrows()`](access-values.md#shape-and-counts) for rows.

## Display prints a grid

`Table` implements `Display`, so `println!("{t}")` renders a formatted grid
with column names, per-column types, and a row/column footer:

```text
┌──────┬────────┬─────────────────────┐
│ sym  │ price  │        size         │
│ SYM  │  F64   │         I64         │
├──────┼────────┼─────────────────────┤
│ AAPL │ 101.5  │ 10                  │
│ MSFT │ 202.0  │ 20                  │
│ GOOG │ 303.25 │ 30                  │
├──────┴────────┴─────────────────────┤
│ 3 rows (3 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

## `Clone`

`Table` is `Clone`. Cloning is cheap — it bumps a reference count on the shared
backing buffers rather than copying the data, so you can freely keep a copy
before handing a table off to a transform or query.

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    # let t = Table::new(&["x"], &[Value::vec(&[1i64, 2, 3])])?;
    let snapshot = t.clone();
    let trimmed = t.head(2)?;        // original is untouched
    assert_eq!(snapshot.nrows(), 3);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## What you can do with a table

| Task | Page |
|------|------|
| Build a table from typed columns | [Create a Table](create.md) |
| Read shapes, columns, and individual cells | [Access Values](access-values.md) |
| Slice rows with `head` / `tail` / `take` | [Transform](transform.md) |
| Round-trip to and from CSV | [Save and Fetch](save-and-fetch.md) |
| Persist as splayed / partitioned tables | [Splayed and Parted](splayed-and-parted.md) |
| Select, filter, group, update, and join | [Query Guide](../query-guide/overview.md) |

!!! tip "Querying"
    Richer reshaping — projections, filters, grouped aggregations, updates, and
    joins — lives in the [Query Guide](../query-guide/overview.md). This section
    covers the table value itself.
