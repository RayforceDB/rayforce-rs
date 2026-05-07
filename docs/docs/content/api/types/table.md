# Table Type

`RayTable` wraps a `RAY_TABLE` value — the engine's columnar data
structure for analytical workloads.

For query construction see the dedicated **[Query Builder](../query.md)**
page; this page covers the table value itself: building it from Rust
data, accessing columns, binding under a name, and rendering.

## Building tables

### From a dict of `(name, column)` pairs

The most direct path from Rust data to a table:

```rust
use rayforce::{RayTable, RayVector, RaySymbol, RayType};

let employees = RayTable::from_dict([
    ("name",   RayVector::<RaySymbol>::from_iter(["Alice", "Bob", "Charlie"]).as_ray_obj().clone()),
    ("age",    RayVector::<i64>::from_iter([25i64, 30, 35]).as_ray_obj().clone()),
    ("salary", RayVector::<i64>::from_iter([50000i64, 60000, 70000]).as_ray_obj().clone()),
])?;

println!("{}", employees);
```

`from_dict` is implemented in terms of `ray_table_new(ncols)` followed
by one `ray_table_add_col(tbl, sym_id, col)` per column. Each column
ray-object is consumed by the table.

### Via `Rayforce::eval`

```rust
use rayforce::Rayforce;

let rf = Rayforce::new()?;

let table = rf.eval(r#"
    (table [name age salary]
        (list [`Alice `Bob `Charlie]
              [25 30 35]
              [50000 60000 70000]))
"#)?;
println!("{}", table);
```

### Resolving an existing table by name

`from_name` looks up a global binding interactively (it's backed by
`ray_sym_intern` + `ray_env_get` + `ray_retain`):

```rust
let employees = RayTable::from_name("employees")?;
```

If the binding doesn't exist or doesn't refer to a table you'll get
`KeyNotFound` or `TypeMismatch`.

## Table operations

### Schema introspection

```rust
let cols: Vec<String> = employees.columns()?;   // ordered column names
let n: usize          = employees.ncols();
let r: usize          = employees.len()?;       // row count
println!("{cols:?}  ({n} cols × {r} rows)");
```

### Accessing columns

```rust
// By name (interns the name and calls ray_table_get_col)
let salaries = employees.get_column("salary")?;

// By ordinal index (ray_table_get_col_idx)
let first_col = employees.get_column_idx(0)?;
```

Both return a freshly retained `RayObj`; the typed `RayVector<T>`
wrapper can be obtained via `RayVector::<i64>::from_ptr(salaries)?` if
you need slice access.

### Binding under a name

```rust
employees.save("employees")?;        // ray_env_set + ray_sym_intern
let again = RayTable::from_name("employees")?;
```

`save` makes the table reachable from Rayfall (`(select … from:
employees …)`).

## Querying tables

You have two equivalent options:

1. **Use the [`SelectQuery`](../query.md) / `UpdateQuery` / `InsertQuery` /
   `UpsertQuery` builders** — fluent, type-safe, renders Rayfall under
   the hood.
2. **Compose Rayfall source by hand** and dispatch through
   `Rayforce::eval`. The engine call is the same either way.

```rust
use rayforce::{Rayforce, RayTable, RayVector, RaySymbol, RayType,
                SelectQuery, Column, Operation};

let rf = Rayforce::new()?;

let trades = RayTable::from_dict([
    ("sym",   RayVector::<RaySymbol>::from_iter(["AAPL","MSFT","AAPL","GOOGL"]).as_ray_obj().clone()),
    ("price", RayVector::<f64>::from_iter([150.0, 300.0, 151.0, 2800.0]).as_ray_obj().clone()),
    ("qty",   RayVector::<i64>::from_iter([100i64, 50, 200, 25]).as_ray_obj().clone()),
])?;
trades.save("trades")?;

// SELECT with WHERE — via the builder
let result = SelectQuery::from("trades")
    .column("sym",   Column::new("sym"))
    .column("total", Column::new("price").mul(Column::new("qty")))
    .filter(Column::new("qty").gt(50))
    .execute(&rf)?;

// GROUP BY with aggregation
let summary = SelectQuery::from("trades")
    .column("total_qty", Operation::Sum(Column::new("qty").into_expr()))
    .column("avg_price", Operation::Avg(Column::new("price").into_expr()))
    .group_by(Column::new("sym"))
    .execute(&rf)?;

println!("{result}\n{summary}");
```

`RayTable::from_name(name)` lets you also start a query through the
1.0-style instance methods (`tbl.select(name) / .update(name) /
.insert(name) / .upsert(name, idx)`) — they're shortcuts for the
`*Query::from(name)` associated functions.

## Type reference

| Item | Description |
|------|-------------|
| `RayTable` | `RAY_TABLE` (98). `Clone` via `ray_retain`. |
| `RayTable::from_dict` | Build from `(name, column)` iterator. |
| `RayTable::from_name` | Resolve a global binding via `ray_env_get`. |
| `RayTable::from_ptr` | Wrap an existing `RayObj` (validates the type tag). |
| `columns()` | `Vec<String>` of column names. |
| `len()` | Row count (`ray_table_nrows`). |
| `ncols()` | Column count (`ray_table_ncols`). |
| `get_column(name)` | Owned `RayObj` for the column. |
| `get_column_idx(idx)` | Owned `RayObj` for the n-th column. |
| `save(name)` | Bind under `name` in the global environment. |
| `as_ray_obj()` | Borrow the underlying `RayObj`. |

## Display

`RayTable` itself renders schema-only via its inherent `Display`:

```text
Table[3 rows × 3 cols] ["name", "age", "salary"]
```

For a full data dump, render the underlying `RayObj`:

```rust
println!("{}", table.as_ray_obj());
```

That delegates to the engine's [`ray_fmt`](../ffi.md#display--debug)
formatter, which produces the REPL-style table with column headers
and row separators.

## Next steps

- **[Query Builder](../query.md)** — fluent SELECT / UPDATE / INSERT / UPSERT.
- **[Containers](containers.md)** — `RayVector` / `RayList` / `RayDict`.
- **[Scalars](scalars.md)** — atom types.
- **[FFI](../ffi.md)** — low-level helpers (`RayObj`, raw bindings).
