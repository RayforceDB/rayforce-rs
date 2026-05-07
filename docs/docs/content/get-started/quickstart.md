# Quick Start

This guide walks you through building your first application with
rayforce-rs against rayforce 2.0.

## Basic setup

Every rayforce-rs application starts by initializing the runtime:

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;
    // Your code here...
    Ok(())
}
```

`Rayforce::new()` initializes the engine. The runtime is automatically
torn down (`ray_runtime_destroy`) when the `Rayforce` instance goes out
of scope. Only one runtime can exist per process at a time.

## Working with scalars

RayforceDB has several scalar types, all available with the `Ray`
prefix:

```rust
use rayforce::{RayI64, RayF64, RaySymbol, RayString, RayObj};

// Integer types
let a = RayI64::new(42);
let b = RayI64::new(100);

// Floating point
let pi = RayF64::new(3.14159);

// Symbols (interned through ray_sym_intern)
let sym = RaySymbol::new("price");

// Strings (SSO atom — inline below 7 bytes, pool-backed otherwise)
let greeting = RayString::new("hello, rayforce");

// Generic object from primitives
let obj = RayObj::from(42i64);
let obj2 = RayObj::from("hello");
```

!!! note "What about `RayChar` / `C8`?"
    The single-character atom type was removed in rayforce 2.0. Use
    `RayU8::new(b'a')` or `RayString::new("a")` depending on the
    situation.

## Creating vectors

Vectors are homogeneous typed columns:

```rust
use rayforce::{RayVector, RaySymbol};

let prices: RayVector<i64> = RayVector::from_iter([100i64, 200, 300, 400]);
let quantities: RayVector<f64> = RayVector::from_iter([1.5, 2.0, 3.5]);
let symbols = RayVector::<RaySymbol>::from_iter(["AAPL", "GOOGL", "MSFT"]);

println!("Count: {}", prices.len());
```

`RayVector::<i64>::set(idx, value)` and `<f64>::set(idx, value)` mutate
elements via the engine's COW mechanism (`ray_vec_set`).

## Creating lists

Lists are heterogeneous boxed containers:

```rust
use rayforce::RayList;

let mut list = RayList::new();
list.push(42i64);
list.push("hello");
list.push(3.14f64);

if let Some(first) = list.get(0) {
    println!("first item: {}", first);
}
for item in list.iter() {
    println!("{}", item);
}
```

## Creating dictionaries

Dictionaries map symbol keys to values. `from_pairs` interns each name
into a symbol:

```rust
use rayforce::{RayDict, RayI64, RayString, RayType};

let dict = RayDict::from_pairs([
    ("name",   RayString::new("Alice").ptr().clone()),
    ("age",    RayI64::new(30).ptr().clone()),
    ("salary", RayI64::new(75000).ptr().clone()),
])?;

if let Some(name) = dict.get("name") {
    println!("name = {}", name);
}
println!("dict has {} keys", dict.len());
```

## Evaluating Rayfall expressions

RayforceDB's query language is **Rayfall** — a Lisp-like syntax. From
Rust you reach it via `Rayforce::eval`:

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;

    let sum = rf.eval("sum 1 2 3")?;
    println!("Sum: {}", sum);  // → 6

    let complex = rf.eval("(* (+ 1 2) (- 10 5))")?;
    println!("Result: {}", complex);  // → 15

    let vec_sum = rf.eval("sum [1 2 3 4 5]")?;
    println!("Vector sum: {}", vec_sum);  // → 15

    let avg = rf.eval("avg [10 20 30 40 50]")?;
    println!("Average: {}", avg);  // → 30
    Ok(())
}
```

Errors from the engine come back as
`RayforceError::Ray { code, message, kind }` where `code` is the short
tag (`"oom"`, `"type"`, `"range"`, …).

## Working with tables

Build a table from `(name, column)` pairs:

```rust
use rayforce::{RayTable, RayVector, RaySymbol, RayType};

let employees = RayTable::from_dict([
    ("name",   RayVector::<RaySymbol>::from_iter(["Alice", "Bob", "Charlie"]).as_ray_obj().clone()),
    ("dept",   RayVector::<RaySymbol>::from_iter(["IT", "HR", "IT"]).as_ray_obj().clone()),
    ("salary", RayVector::<i64>::from_iter([75000i64, 65000, 85000]).as_ray_obj().clone()),
])?;

println!("rows: {}, cols: {}", employees.len()?, employees.ncols());
println!("columns: {:?}", employees.columns()?);
let salaries = employees.get_column("salary")?;
println!("salaries: {}", salaries);
```

You can also build a table directly inside Rayfall via `eval`:

```rust
let employees = rf.eval(r#"
    (table [name dept salary]
        (list [`Alice `Bob `Charlie]
              [`IT `HR `IT]
              [75000 65000 85000]))
"#)?;
println!("{}", employees);
```

## Querying tables

Two equivalent paths: the **fluent query builder**
([`SelectQuery`](../api/query.md) / `UpdateQuery` / `InsertQuery` /
`UpsertQuery`) or hand-written Rayfall source through
`Rayforce::eval`. The builder renders Rayfall under the hood, so the
underlying engine call is identical.

```rust
use rayforce::{
    Rayforce, RayTable, RayVector, RaySymbol, RayType,
    SelectQuery, Column, Operation,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;

    let employees = RayTable::from_dict([
        ("name",   RayVector::<RaySymbol>::from_iter(["Alice", "Bob", "Charlie", "David"]).as_ray_obj().clone()),
        ("dept",   RayVector::<RaySymbol>::from_iter(["IT", "HR", "IT", "Sales"]).as_ray_obj().clone()),
        ("salary", RayVector::<i64>::from_iter([75000i64, 65000, 85000, 70000]).as_ray_obj().clone()),
    ])?;
    employees.save("employees")?;   // bind under the name "employees"

    // SELECT with WHERE — via the builder
    let high_earners = SelectQuery::from("employees")
        .columns(["name", "salary"])
        .filter(Column::new("salary").gt(70000))
        .execute(&rf)?;
    println!("high earners:\n{}", high_earners);

    // GROUP BY with aggregation
    let by_dept = SelectQuery::from("employees")
        .column("avg_salary", Operation::Avg(Column::new("salary").into_expr()))
        .column("headcount", Operation::Count(Column::new("name").into_expr()))
        .group_by(Column::new("dept"))
        .execute(&rf)?;
    println!("by department:\n{}", by_dept);
    Ok(())
}
```

If you prefer to write Rayfall directly, the same queries through
`eval`:

```rust
let high_earners = rf.eval(
    "(select {name: name salary: salary from: employees where: (> salary 70000)})"
)?;
```

`RayTable::save(name)` is backed by `ray_env_set(ray_sym_intern(name),
table)`, the same mechanism the runtime uses to bind any global value.

## Complete example

Putting it all together:

```rust
use rayforce::{Rayforce, RayTable, RayVector, RaySymbol, RayType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;
    println!("RayforceDB {} initialised", rf.version());

    // Build a trades table from Rust data
    let trades = RayTable::from_dict([
        ("symbol",   RayVector::<RaySymbol>::from_iter(["AAPL","GOOGL","MSFT","AAPL","GOOGL"]).as_ray_obj().clone()),
        ("price",    RayVector::<f64>::from_iter([150.25, 2800.50, 300.75, 151.00, 2805.25]).as_ray_obj().clone()),
        ("quantity", RayVector::<i64>::from_iter([100i64, 50, 200, 150, 75]).as_ray_obj().clone()),
    ])?;
    trades.save("trades")?;

    // Per-symbol totals — query expressed in Rayfall
    let totals = rf.eval(r#"
        (select {from:trades by: symbol
                 total_value:(sum (* price quantity))
                 trade_count:(count symbol)})
    "#)?;
    println!("\nTotals by symbol:\n{}", totals);
    Ok(())
}
```

## What's next?

- **[API Reference](../api/overview.md)** — full API documentation.
- **[Types](../api/types/scalars.md)** — detailed type system guide.
- **[Examples](../examples/index.md)** — more code samples.
