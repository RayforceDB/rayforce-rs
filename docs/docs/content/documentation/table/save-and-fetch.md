# :material-content-save: Save and Fetch (CSV)

A [:octicons-table-24: Table](overview.md) can be written to and read back from
CSV. Writing is schema-free; reading requires you to declare the type of each
column up front.

## Writing — `write_csv`

`write_csv(path)` serializes the table to a CSV file at `path`.

```rust
use rayforce::{Runtime, Table, Value};

let _rt = Runtime::new()?;

let t = Table::new(
    &["sym", "price", "size"],
    &[
        Value::sym_vec(&["AAPL", "MSFT", "GOOG"]),
        Value::vec(&[101.5f64, 202.0, 303.25]),
        Value::vec(&[10i64, 20, 30]),
    ],
)?;

t.write_csv("/tmp/trades.csv")?;
# Ok::<(), rayforce::RayError>(())
```

## Reading — `Table::read_csv`

`Table::read_csv(&[type_tokens], path)` reads a CSV, parsing each column
according to the type token at the same position. There must be one token per
column.

```rust
# use rayforce::{Runtime, Table, Value};
# let _rt = Runtime::new()?;
# let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
# t.write_csv("/tmp/trades.csv")?;
let loaded = Table::read_csv(&["SYMBOL", "F64", "I64"], "/tmp/trades.csv")?;

assert_eq!(loaded.shape(), (3, 3));
assert_eq!(loaded.column("price")?.as_slice::<f64>()?, &[101.5, 202.0, 303.25]);
# Ok::<(), rayforce::RayError>(())
```

## Type tokens

Tokens are conventionally written in UPPERCASE, but matching is
case-insensitive (`"i64"` works as well as `"I64"`).

| Token | Column type |
|-------|-------------|
| `B8` | boolean |
| `U8` | byte |
| `I16` | 16-bit integer |
| `I32` | 32-bit integer |
| `I64` | 64-bit integer |
| `F32` | 32-bit float |
| `F64` | 64-bit float |
| `DATE` | date |
| `TIME` | time |
| `TIMESTAMP` | timestamp |
| `SYMBOL` | symbol |
| `STR` | string |
| `GUID` | GUID |

### Aliases

| Alias | Resolves to |
|-------|-------------|
| `SYM` | `SYMBOL` |
| `STRING` | `STR` |
| `BOOL`, `BOOLEAN` | `B8` |

```rust
# use rayforce::{Runtime, Table};
# let _rt = Runtime::new()?;
// these schemas are equivalent
# let _ = || -> rayforce::Result<()> {
let a = Table::read_csv(&["SYM", "F64", "I64"], "/tmp/trades.csv")?;
let b = Table::read_csv(&["symbol", "f64", "i64"], "/tmp/trades.csv")?;
# let _ = (a, b); Ok(()) };
# Ok::<(), rayforce::RayError>(())
```

!!! tip "Reading order"
    `read_csv` matches tokens to columns by position, so the token order must
    follow the CSV's column order. See [Data Types](../data-types/overview.md)
    for what each token represents.

For columnar on-disk storage, see [Splayed and Parted](splayed-and-parted.md).
