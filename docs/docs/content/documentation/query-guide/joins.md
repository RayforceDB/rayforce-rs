# :octicons-plus-24: Joins

`Table` provides three joins, each taking the right-hand table and a slice of
key column names: `inner_join(&other, &[on])`, `left_join(&other, &[on])`, and
`asof_join(&other, &[on])`. Each returns a new `Result<Table>`.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{Runtime, Table, Value};
    let _rt = Runtime::new()?;
    let t = trades(); // sym / price / size — see the Overview
    ```

## Inner join

`inner_join` keeps only rows whose key matches a row in the right table, and
appends the right table's non-key columns. Here we enrich each trade with its
`sector` from a reference table:

```rust
let rsym   = Value::sym_vec(&["AAPL", "MSFT", "GOOG"]);
let sector = Value::sym_vec(&["Tech", "Tech", "Search"]);
let reference = Table::new(&["sym", "sector"], &[rsym, sector])?;

let joined = t.inner_join(&reference, &["sym"])?;
println!("{joined}");
```

```text
┌──────┬───────┬──────┬───────────────┐
│ sym  │ price │ size │    sector     │
│ SYM  │  F64  │ I64  │      SYM      │
├──────┼───────┼──────┼───────────────┤
│ AAPL │ 100.0 │ 10   │ Tech          │
│ MSFT │ 200.0 │ 20   │ Tech          │
│ AAPL │ 110.0 │ 30   │ Tech          │
│ GOOG │ 300.0 │ 40   │ Search        │
│ MSFT │ 210.0 │ 50   │ Tech          │
├──────┴───────┴──────┴───────────────┤
│ 5 rows (5 shown) 4 columns (4 shown)│
└─────────────────────────────────────┘
```

!!! tip "Join on several columns"
    Pass more than one key: `t.inner_join(&other, &["sym", "date"])?`.

## Left join

`left_join` keeps **every** left row. Where the right table has no match, the
added columns are filled with nulls. Note the reference table below omits `GOOG`,
so its `sector` is null:

```rust
let rsym   = Value::sym_vec(&["AAPL", "MSFT"]);
let sector = Value::sym_vec(&["Tech", "Tech"]);
let reference = Table::new(&["sym", "sector"], &[rsym, sector])?;

let joined = t.left_join(&reference, &["sym"])?;
println!("{joined}");
```

```text
┌──────┬───────┬──────┬───────────────┐
│ sym  │ price │ size │    sector     │
│ SYM  │  F64  │ I64  │      SYM      │
├──────┼───────┼──────┼───────────────┤
│ AAPL │ 100.0 │ 10   │ Tech          │
│ MSFT │ 200.0 │ 20   │ Tech          │
│ AAPL │ 110.0 │ 30   │ Tech          │
│ GOOG │ 300.0 │ 40   │               │
│ MSFT │ 210.0 │ 50   │ Tech          │
├──────┴───────┴──────┴───────────────┤
│ 5 rows (5 shown) 4 columns (4 shown)│
└─────────────────────────────────────┘
```

## As-of join

`asof_join` is a value-ordered join: each left row is matched with the *most
recent* right row at or before it, per the remaining keys. The last column in
`on` is the ordering column (typically time); any earlier keys are matched
exactly. Each left trade picks up the latest quote for its symbol:

```rust
let tsym   = Value::sym_vec(&["AAPL", "AAPL", "GOOG"]);
let ttime  = Value::vec(&[100i64, 200, 150]);
let tprice = Value::vec(&[10.0f64, 20.0, 30.0]);
let trades = Table::new(&["sym", "time", "price"], &[tsym, ttime, tprice])?;

let qsym  = Value::sym_vec(&["AAPL", "AAPL", "GOOG"]);
let qtime = Value::vec(&[50i64, 150, 100]);
let qbid  = Value::vec(&[9.0f64, 11.0, 29.0]);
let quotes = Table::new(&["sym", "time", "bid"], &[qsym, qtime, qbid])?;

let joined = trades.asof_join(&quotes, &["sym", "time"])?;
println!("{joined}");
```

```text
┌──────┬──────┬───────┬───────────────┐
│ sym  │ time │ price │      bid      │
│ SYM  │ I64  │  F64  │      F64      │
├──────┼──────┼───────┼───────────────┤
│ AAPL │ 100  │ 10.0  │ 9.0           │
│ AAPL │ 200  │ 20.0  │ 11.0          │
│ GOOG │ 150  │ 30.0  │ 29.0          │
├──────┴──────┴───────┴───────────────┤
│ 3 rows (3 shown) 4 columns (4 shown)│
└─────────────────────────────────────┘
```

For the `AAPL` trade at `time = 100`, the most recent `AAPL` quote at or before
`100` is the one at `time = 50` (`bid = 9.0`); the trade at `200` picks up the
quote at `150` (`bid = 11.0`).

!!! note "Window joins are not yet available"
    The Rust bindings do not implement window joins. Only `inner_join`,
    `left_join`, and `asof_join` are available today.

This completes the query guide. Return to the [Overview](overview.md) for the
full map, or revisit [Expressions](expressions.md) for the DSL building blocks.
