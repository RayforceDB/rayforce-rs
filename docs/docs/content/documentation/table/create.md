# :material-table-plus: Create a Table

Build a [:octicons-table-24: Table](overview.md) from a list of column names and
a matching list of typed column [:material-vector-line: Vectors](../data-types/vector.md).

## `Table::new`

`Table::new(&[names], &[columns])` takes the column names and one `Value` vector
per column. The columns are matched to names by position.

```rust
use rayforce::{Runtime, Table, Value};

Runtime::scope(|_rt| {
    let t = Table::new(
        &["sym", "price", "size"],
        &[
            Value::sym_vec(&["AAPL", "MSFT", "GOOG"]),  // symbol column
            Value::vec(&[101.5f64, 202.0, 303.25]),     // f64 column
            Value::vec(&[10i64, 20, 30]),               // i64 column
        ],
    )?;

    assert_eq!(t.shape(), (3, 3));   // (rows, cols)
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## Building columns

Columns are ordinary vectors. The two you'll reach for most are:

- `Value::vec(&[T])` for numeric columns, where `T` is one of
  `u8 / i16 / i32 / i64 / f32 / f64`.
- `Value::sym_vec(&[S])` for interned symbol columns (e.g. tickers, tags).

```rust
# use rayforce::Value;
let ints   = Value::vec(&[1i64, 2, 3]);
let floats = Value::vec(&[1.5f64, 2.5, 3.5]);
let syms   = Value::sym_vec(&["a", "b", "c"]);
```

!!! warning "Annotate integer literals"
    Integer literals default to `i32` in Rust. For an `i64` column, write
    `Value::vec(&[1i64, 2, 3])` — otherwise you get an `I32` column. See
    [Integers](../data-types/integers.md) for the full type list, and
    [Strings](../data-types/string.md) for `Value::str_vec` if you need real
    string columns rather than symbols.

## Validation

`Table::new` validates the inputs and returns a `Result`. It fails if:

- the number of names does not equal the number of columns, or
- the columns do not all have the same number of rows.

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    // mismatched row counts -> Err
    let bad = Table::new(
        &["a", "b"],
        &[Value::vec(&[1i64, 2, 3]), Value::vec(&[10i64, 20])],
    );
    assert!(bad.is_err());
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

## Interoperating with `Value`

`Table` is a newtype over [`Value`](../data-types/values.md), so you can convert
freely in both directions:

| Method | Direction | Notes |
|--------|-----------|-------|
| `Table::from_value(v)` | `Value` → `Table` | `Err` if `v` is not a table |
| `value.as_table()` | `Value` → `Table` | same check, called on the value |
| `table.as_value()` | `Table` → `&Value` | borrow the inner handle |
| `table.into_value()` | `Table` → `Value` | take ownership |

```rust
# use rayforce::{Runtime, Table, Value};
Runtime::scope(|_rt| {
    # let t = Table::new(&["x"], &[Value::vec(&[1i64, 2, 3])])?;
    let v: Value = t.clone().into_value();
    assert!(v.is_table());

    let back: Table = v.as_table()?;             // or Table::from_value(v)
    assert_eq!(back.nrows(), 3);

    // non-table values cannot become a Table
    assert!(Value::i64(5).as_table().is_err());
    Ok(())
})?;
# Ok::<(), rayforce::RayError>(())
```

Next: [access columns and values](access-values.md).
