# :material-view-column: Splayed and Parted

For larger datasets, a [:octicons-table-24: Table](overview.md) can be stored on
disk **column by column** (a *splayed* table) or split across a directory tree
of partitions (a *parted* table). Each column lives in its own file, so reads
can touch only the columns they need.

## Save splayed — `save_splayed`

`save_splayed(dir, sym_path)` writes the table into directory `dir`, one file
per column. The second argument is an optional path to a **symbol file** that
enumerated symbol columns are written against.

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

// this table has a symbol column, so supply a symfile path
t.save_splayed("/tmp/db/trades", Some("/tmp/db/sym"))?;
# Ok::<(), rayforce::RayError>(())
```

!!! warning "Symbol columns need a symfile"
    If the table contains any **symbol** column, you must pass an explicit
    `Some(path)` for the symbol file — symbols are stored as enumerations against
    it. A table with only numeric columns can pass `None`. Use the **same**
    symfile path when you load the table back.

```rust
# use rayforce::{Runtime, Table, Value};
# let _rt = Runtime::new()?;
// numeric-only table: no symfile required
let nums = Table::new(
    &["a", "b"],
    &[Value::vec(&[1i64, 2, 3]), Value::vec(&[1.0f64, 2.0, 3.0])],
)?;
nums.save_splayed("/tmp/db/nums", None)?;
# Ok::<(), rayforce::RayError>(())
```

## Load splayed — `Table::load_splayed`

`Table::load_splayed(dir, sym_path)` reads a splayed table back. Pass the same
`sym_path` you saved with (or `None` for numeric-only tables).

```rust
# use rayforce::{Runtime, Table, Value};
# let _rt = Runtime::new()?;
# let t = Table::new(&["sym","price","size"], &[Value::sym_vec(&["AAPL","MSFT","GOOG"]), Value::vec(&[101.5f64,202.0,303.25]), Value::vec(&[10i64,20,30])])?;
# t.save_splayed("/tmp/db/trades", Some("/tmp/db/sym"))?;
let loaded = Table::load_splayed("/tmp/db/trades", Some("/tmp/db/sym"))?;

assert_eq!(loaded.shape(), (3, 3));
assert_eq!(loaded.column("size")?.as_slice::<i64>()?, &[10, 20, 30]);
# Ok::<(), rayforce::RayError>(())
```

## Load parted — `Table::load_parted`

A *parted* (partitioned) table is a directory whose top level is split into
partitions (for example, one subdirectory per date), each holding a splayed copy
of the table. `Table::load_parted(root, name)` loads the named table across all
partitions under `root` into a single table.

```rust
# use rayforce::{Runtime, Table};
# let _rt = Runtime::new()?;
# let _ = || -> rayforce::Result<()> {
let trades = Table::load_parted("/tmp/pdb", "trades")?;
println!("{} rows across all partitions", trades.nrows());
# Ok(()) };
# Ok::<(), rayforce::RayError>(())
```

!!! note "Round-trip safely"
    Save and load with matching paths and symfiles. For a single-file,
    human-readable format instead, see [Save and Fetch (CSV)](save-and-fetch.md).
