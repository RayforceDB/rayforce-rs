# :material-table-merge-cells: Upsert

`upsert_columns(key_columns: i64, &[Value])` is a key-aware
[insert](insert.md): rows whose key already exists are **updated**, and rows
with a new key are **appended**.

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Table, Value};
    let _rt = Runtime::new()?;
    ```

## The `key_columns` argument

`key_columns` is the number of **leading** columns that form the match key.
Rayforce looks at those columns to decide each incoming row's fate:

- **match found** → the remaining columns of that row are overwritten;
- **no match** → the whole row is appended.

With `key_columns: 1`, the first column is the key. With `2`, the first two
columns together form a composite key, and so on.

## Example

Start from a small keyed table:

```rust
let k = Value::sym_vec(&["a", "b"]);
let v = Value::vec(&[1i64, 2]);
let kt = Table::new(&["k", "v"], &[k, v])?;
```

Upsert two rows keyed on the first column. Key `a` already exists (so its `v` is
updated to `9`); key `c` is new (so it is appended). Key `b` is untouched:

```rust
let r = kt.upsert_columns(1, &[
    Value::sym_vec(&["a", "c"]),
    Value::vec(&[9i64, 3]),
])?;
println!("{r}");
```

```text
┌─────┬───────────────────────────────┐
│  k  │               v               │
│ SYM │              I64              │
├─────┼───────────────────────────────┤
│ a   │ 9                             │
│ b   │ 2                             │
│ c   │ 3                             │
├─────┴───────────────────────────────┤
│ 3 rows (3 shown) 2 columns (2 shown)│
└─────────────────────────────────────┘
```

!!! warning "Column layout must match"
    Each `Value` in the slice is a vector for one column, in declaration order,
    and the key columns come first. As with [insert](insert.md), all vectors
    must share the same length and match their column types.

Continue with [Joins](joins.md) to combine tables.
