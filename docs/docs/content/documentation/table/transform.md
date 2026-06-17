# :material-table-edit: Transform

The simplest transforms slice rows off a [:octicons-table-24: Table](overview.md).
`head`, `tail`, and `take` each return a new `Table` and leave the original
untouched.

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
# Ok::<(), rayforce::RayError>(())
```

## `head(n)` — first `n` rows

```rust
# use rayforce::{Runtime, Table, Value};
# let _rt = Runtime::new()?;
# let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
let top = t.head(2)?;
assert_eq!(top.nrows(), 2);
println!("{top}");
# Ok::<(), rayforce::RayError>(())
```

```text
┌──────┬───────┬──────────────────────┐
│ sym  │ price │         size         │
│ SYM  │  F64  │         I64          │
├──────┼───────┼──────────────────────┤
│ AAPL │ 101.5 │ 10                   │
│ MSFT │ 202.0 │ 20                   │
├──────┴───────┴──────────────────────┤
│ 2 rows (2 shown) 3 columns (3 shown)│
└─────────────────────────────────────┘
```

## `tail(n)` — last `n` rows

```rust
# use rayforce::{Runtime, Table, Value};
# let _rt = Runtime::new()?;
# let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
let bottom = t.tail(2)?;
assert_eq!(bottom.column("sym")?.get(0)?.as_sym()?, "MSFT");
# Ok::<(), rayforce::RayError>(())
```

## `take(n)` — `n` rows, signed

`take(n)` takes `n` rows. The count is an `i64`: a **positive** `n` takes from
the front (like `head`), and a **negative** `n` takes from the end (like
`tail`).

```rust
# use rayforce::{Runtime, Table, Value};
# let _rt = Runtime::new()?;
# let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
let first_two = t.take(2)?;     // same rows as head(2)
let last_two  = t.take(-2)?;    // same rows as tail(2)

assert_eq!(first_two.column("sym")?.get(0)?.as_sym()?, "AAPL");
assert_eq!(last_two.column("sym")?.get(0)?.as_sym()?, "MSFT");
# Ok::<(), rayforce::RayError>(())
```

!!! note "`head`/`tail`/`take` all take `i64`"
    All three accept an `i64`. `head(n)` and `tail(n)` use the magnitude of `n`;
    `take(n)` additionally interprets the sign to pick the front or the back.

## Beyond row slicing

For richer reshaping — choosing and renaming columns, filtering rows, grouped
aggregations, computed/updated columns, and joins — use the query DSL:

- [Select](../query-guide/select.md) — project and aggregate columns
- [Where](../query-guide/where.md) — filter rows by predicate
- [Group By](../query-guide/group-by.md) — aggregate by key
- [Update](../query-guide/update.md) — add or modify columns
- [Joins](../query-guide/joins.md) — inner, left, and as-of joins

See the [Query Guide overview](../query-guide/overview.md) to get started.
