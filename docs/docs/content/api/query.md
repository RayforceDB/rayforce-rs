# Query Builder

A fluent, type-safe builder for `SELECT` / `UPDATE` / `INSERT` /
`UPSERT` queries. The builder synthesises Rayfall source from your
Rust expressions and dispatches it through
[`Rayforce::eval`](overview.md#evaluating-expressions).

```rust
use rayforce::{Rayforce, RayTable, SelectQuery, Column, Operation};

let rf = Rayforce::new()?;
rf.eval("(set trades (table [sym price volume] \
    (list [AAPL GOOG MSFT AAPL] [101.0 99.5 250.0 102.0] [100 200 300 400])))")?;

let result = SelectQuery::from("trades")
    .column("sym", Column::new("sym"))
    .column("avg_price", Operation::Avg(Column::new("price").into_expr()))
    .filter(Column::new("price").gt(100))
    .group_by(Column::new("sym"))
    .desc("avg_price")
    .take(10)
    .execute(&rf)?;

println!("{result}");
```

The query is rendered as:

```text
(select {sym: sym avg_price: (avg price) from: trades \
         where: (> price 100) by: sym desc: avg_price take: 10})
```

## How it works

In rayforce 2.x the C entry points `ray_select` / `ray_update` /
`ray_insert` / `ray_upsert` are *special forms* — the values inside
the `from:` / `where:` / `by:` dict stay unevaluated until the
evaluator gets to them. Building a correct AST from outside the
engine is awkward, so the Rust layer renders a Rayfall source string
and calls `Rayforce::eval` instead. The end result is the same query;
the builder is the syntax sugar.

You can always inspect the rendered source via `to_rayfall()`:

```rust
let q = SelectQuery::from("t").filter(Column::new("price").gt(100));
println!("{}", q.to_rayfall());
// (select {from: t where: (> price 100)})
```

## `Column`

A reference to a column by name. Inside a Rayfall query body a bare
symbol resolves to the matching column, which is exactly what
`Column` renders.

```rust
use rayforce::Column;

let c = Column::new("price");

// Comparison → boolean Expression
let e1 = c.clone().gt(100);                  // (> price 100)
let e2 = c.clone().eq(Column::new("avg"));    // (== price avg)
let e3 = c.clone().is_in(rayforce::Expression::raw("[100 200 300]"));
                                              // (in price [100 200 300])

// Arithmetic
let e4 = Column::new("price").mul(Column::new("volume"));
                                              // (* price volume)

// Aggregate shortcuts
let e5 = Column::new("volume").sum();         // (sum volume)
let e6 = Column::new("price").avg();          // (avg price)
let e7 = Column::new("sym").count();          // (count sym)
let e8 = Column::new("sym").distinct();       // (distinct sym)
```

Available method families:

| Family | Methods |
|---|---|
| Comparison | `eq` `ne` `gt` `ge` `lt` `le` `is_in` |
| Arithmetic | `add` `sub` `mul` `div` |
| Aggregate | `sum` `avg` `min` `max` `count` `first` `last` `distinct` |

Each comparison/arithmetic method takes anything `Into<Expression>`
— `Column`, `Expression`, `i64` / `f64` / `bool` / `&str` literals
all work directly.

## `Expression`

A piece of Rayfall source. Build leaves with the `lit_*`
constructors; combine with `Expression::call`, the `Column`
operators, or the [`Operation`](#operation) factory.

```rust
use rayforce::Expression;

let i = Expression::lit_i64(42);            // 42
let f = Expression::lit_f64(1.5);           // 1.5
let b = Expression::lit_bool(true);         // 1b
let s = Expression::lit_str("a\"b");        // "a\"b"  (escaped)
let y = Expression::lit_sym("AAPL");        // 'AAPL

// Arbitrary call form
let pred = Expression::call("between",
    &[Expression::lit_i64(0),
      Column::new("score").into_expr(),
      Expression::lit_i64(100)]);
// → (between 0 score 100)

// Chainable boolean composition
let pred = Column::new("price").gt(100)
    .and(Column::new("volume").lt(450))
    .or(Column::new("sym").eq(Expression::lit_sym("AAPL")));
```

If you need a Rayfall fragment the builder doesn't model directly,
use `Expression::raw("…")` — the contents are pasted verbatim.

## `Operation`

A factory for a handful of common aggregate / unary calls. Each
variant maps to one Rayfall operator name.

```rust
use rayforce::{Operation, Column};

Operation::Sum(Column::new("qty").into_expr());      // (sum qty)
Operation::Avg(Column::new("price").into_expr());    // (avg price)
Operation::Custom("ratio".into(),
    vec![Column::new("a").into_expr(),
         Column::new("b").into_expr()]);             // (ratio a b)
```

| Variant | Renders |
|---|---|
| `Sum / Avg / Min / Max / Count / First / Last` | `(name expr)` |
| `Abs / Neg / Not` | `(name expr)` |
| `And(a, b) / Or(a, b)` | `(name a b)` |
| `Custom(name, args)` | `(name a b ...)` |

`Operation` and the per-`Column` aggregate methods cover the same
ground in different styles — pick whichever reads cleaner at the
call site.

## `SelectQuery`

```rust
use rayforce::{SelectQuery, Column, Operation};

SelectQuery::from("trades")
    // Projected columns: name → expression.
    .column("sym",   Column::new("sym"))
    .column("notional", Column::new("price").mul(Column::new("volume")))
    // Or bulk-add identity-projected columns:
    //   .columns(["sym", "price", "volume"])
    // Filtering.
    .filter(Column::new("price").gt(100))
    // Group-by (one expression per call; chain for composite keys).
    .group_by(Column::new("sym"))
    // Sort: chain asc/desc; sort priority follows the call order.
    .desc("notional")
    // Limiting.
    .take(10)
    // Render: get the Rayfall source.
    .to_rayfall();
```

| Method | Effect |
|---|---|
| `from(table)` | Start a builder against the named global table. |
| `column(name, expr)` | Add `name: expr` to the projection. |
| `columns(iter)` | Bulk identity projection — each name renders as `name: name`. |
| `filter(pred)` | `where: pred`. |
| `group_by(key)` | `by: key` (chain for composite keys). |
| `asc(col)` / `desc(col)` | `asc: col` / `desc: col` (chain). |
| `take(n)` | `take: n`. |
| `to_rayfall()` | Render to source. |
| `execute(rf)` | Render and dispatch through `Rayforce::eval`. |

## `UpdateQuery`

```rust
use rayforce::{UpdateQuery, Column, Expression};

UpdateQuery::from("trades")
    .set("price", Column::new("price").mul(Expression::lit_f64(1.05)))
    .set("flagged", Expression::lit_bool(true))
    .filter(Column::new("volume").gt(1000))
    .execute(&rf)?;
// (update {price: (* price 1.05) flagged: 1b from: 'trades where: (> volume 1000)})
```

The `from:` value is rendered as a quoted symbol (`'trades`) so the
engine **mutates the table in place**. To produce a new table without
touching the original, render through `eval` with `(set new …)`:

```rust
rf.eval(&format!("(set bumped {})", update_q.to_rayfall().replace("'trades", "trades")))?;
```

| Method | Effect |
|---|---|
| `from(table)` | Bound name of the table to update. |
| `set(col, expr)` | `col: expr` assignment. Accepts anything `Into<Expression>`. |
| `filter(pred)` | `where: pred`. |
| `group_by(key)` | `by: key` (rare; for grouped conditional updates). |

## `InsertQuery`

```rust
use rayforce::{InsertQuery, Expression};

InsertQuery::into_table("trades")
    .rows(Expression::raw("(list 'TSLA 199.0 500)"))   // single row
    .execute(&rf)?;

InsertQuery::into_table("trades")
    .rows(Expression::raw("(list ['TSLA 'NVDA] [199.0 460.0] [500 600])"))
    .execute(&rf)?;
// (insert 'trades …)
```

Common row payload shapes:

| Shape | Meaning |
|---|---|
| `(list v1 v2 ...)` | Single row in column order. |
| `(list [v...] [v...] [v...])` | Multiple rows in column order. |
| `(dict [Col Col Col] (list ...))` | Reordered or partial columns. |
| `(table [...] (list ...))` | Append from another table. |

## `UpsertQuery`

```rust
use rayforce::{UpsertQuery, Expression};

// Match on column index 1 (e.g. the Name column of [ID Name Value]).
UpsertQuery::into_table("employees", 1)
    .rows(Expression::raw("(list 4 'Dave 40.0)"))
    .execute(&rf)?;
// (upsert 'employees 1 (list 4 'Dave 40.0))
```

The second argument to `into_table` is the **0-based key column
index**. Rows whose key matches an existing row update that row;
unmatched rows are inserted.

## `RayTable` instance methods

For a table loaded via [`RayTable::from_name`](types/table.md), the
1.0-style instance methods are also available — they hand you the
same builders, parameterised on the bound name:

```rust
use rayforce::RayTable;

let t = RayTable::from_name("trades")?;
let q = t.select("trades").filter(Column::new("price").gt(100));
let r = q.execute(&rf)?;
```

These exist purely for ergonomic parity with the 1.0 surface; they're
direct shortcuts for `SelectQuery::from(name)` etc. The 1.0 module
could omit the name because tables carried a global-name reference
internally; 2.x tables are anonymous values, so the bound name is
passed explicitly.

## What didn't make it back

- `Expression::compile() -> RayObj` — the 1.0 method returned an
  evaluated `ray_t`. It relied on `get_internal_function`, which is
  no longer in the 2.x public API. Use `to_rayfall()` and dispatch
  through `Rayforce::eval` instead.
- The 1.0 `Operation` enum had ~50 variants used purely as a name →
  `get_internal_function` lookup table. The new `Operation` is a
  much smaller Rayfall-fragment factory with the variants you'd
  typically use inside a query body.

## Next steps

- **[Tables](types/table.md)** — building tables from Rust data.
- **[FFI](ffi.md)** — what `Rayforce::eval` does under the hood.
- **[Examples](../examples/index.md)** — more code samples.
