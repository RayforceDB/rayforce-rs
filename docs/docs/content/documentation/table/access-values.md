# :material-table-eye: Access Values

Once you have a [:octicons-table-24: Table](overview.md) you can inspect its
shape, pull out whole columns, or read individual cells.

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

## Shape and counts

| Method | Returns | Meaning |
|--------|---------|---------|
| `ncols()` | `usize` | number of columns |
| `nrows()` | `usize` | number of rows |
| `shape()` | `(usize, usize)` | `(rows, cols)` |
| `column_names()` | `Vec<String>` | column names, in order |

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    # let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
    assert_eq!(t.ncols(), 3);
    assert_eq!(t.nrows(), 3);
    assert_eq!(t.shape(), (3, 3));
    assert_eq!(t.column_names(), vec!["sym", "price", "size"]);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

!!! warning "Row count"
    Use `nrows()` for the number of rows. `Value::len()` on a table returns the
    number of internal slots (`2`), not the row count.

## Whole columns

Get a column by name with `column(name)`, by position with `column_at(idx)`, or
grab them all with `columns()`. Each returns the column as a `Value` vector.
Lookups that miss return an `Err`.

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    # let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
    let price = t.column("price")?;        // by name
    let size  = t.column_at(2)?;           // by index
    let all   = t.columns()?;              // Vec<Value>, in column order

    assert_eq!(all.len(), 3);
    assert!(t.column("nope").is_err());    // unknown name -> Err
    assert!(t.column_at(9).is_err());      // out of range -> Err
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## Reading a column (zero-copy)

For numeric columns, `as_slice::<T>()` borrows the column's storage directly as
a `&[T]` — no allocation, no copy. The element type `T` must match the column's
type (one of `u8 / i16 / i32 / i64 / f32 / f64`).

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    # let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
    let price: &[f64] = t.column("price")?.as_slice()?;
    assert_eq!(price, &[101.5, 202.0, 303.25]);

    let size: &[i64] = t.column_at(2)?.as_slice()?;
    assert_eq!(size, &[10, 20, 30]);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

!!! tip "Symbol and other columns"
    `as_slice::<T>()` only applies to the fixed-width numeric types. For symbol,
    boolean, or temporal columns, use the per-type accessors documented under
    [Data Types](../data-types/overview.md) (e.g. read each element with `get`,
    below, or `bool_slice()` for booleans).

## A single element

`get(i)` returns the element at row `i` of a column as a `Value`, which you can
then read with the matching `as_*` accessor.

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    # let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
    let sym0 = t.column("sym")?.get(0)?.as_sym()?;
    assert_eq!(sym0, "AAPL");

    let price1 = t.column("price")?.get(1)?.as_f64()?;
    assert_eq!(price1, 202.0);
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

Next: [slice rows with `head` / `tail` / `take`](transform.md).
