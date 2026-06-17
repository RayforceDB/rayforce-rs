# :octicons-sort-desc-24: Order By

Sort a result with `.order_by([cols], desc)` on the [`Select`](select.md)
builder. The first argument is the list of columns to sort by; the second is a
`bool` — `true` for descending, `false` for ascending.

!!! note "Assume a live runtime and the trades table"
    ```rust
    use rayforce::{col, Runtime, Table, Value};
    let _rt = Runtime::new()?;
    let t = trades(); // sym / price / size — see the Overview
    ```

## Descending sort

```rust
let r = t
    .select()
    .col("price")
    .order_by(["price"], true) // descending
    .execute()?;
println!("{r}");
```

```text
┌─────────────────────────────────────┐
│                price                │
│                 F64                 │
├─────────────────────────────────────┤
│ 300.0                               │
│ 210.0                               │
│ 200.0                               │
│ 110.0                               │
│ 100.0                               │
├─────────────────────────────────────┤
│ 5 rows (5 shown) 1 columns (1 shown)│
└─────────────────────────────────────┘
```

## Ascending sort

Pass `false` for ascending order:

```rust
let r = t.select().col("price").order_by(["price"], false).execute()?;
```

## Multiple sort columns

Provide more than one column to sort by the first, breaking ties with the next:

```rust
let r = t
    .select()
    .cols(["sym", "price"])
    .order_by(["sym", "price"], false)
    .execute()?;
```

## Chaining with other operations

`.order_by` composes with [filtering](where.md), [grouping](group-by.md), and
projection. It applies to the final result:

```rust
let r = t
    .select()
    .cols(["sym", "price"])
    .filter(col("price").gt(150.0))
    .order_by(["price"], true)
    .execute()?;
```

Continue with [Update](update.md) to modify table columns.
