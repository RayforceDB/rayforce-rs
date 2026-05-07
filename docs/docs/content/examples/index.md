# Examples

Code examples demonstrating rayforce-rs against rayforce 2.x.

Queries can be built fluently with
[`SelectQuery`](../api/query.md) / `UpdateQuery` / `InsertQuery` /
`UpsertQuery`, or written as Rayfall source and dispatched through
`Rayforce::eval` — both paths hit the same engine entry point.

## Basic examples

### Hello world

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;
    let result = rf.eval("sum 1 2 3")?;
    println!("1 + 2 + 3 = {}", result);
    Ok(())
}
```

### Working with types

```rust
use rayforce::{
    Rayforce, RayI64, RayF64, RaySymbol, RayString,
    RayVector, RayList, RayDict, RayType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _rf = Rayforce::new()?;

    // Scalars
    let price  = RayI64::new(150);
    let ratio  = RayF64::new(1.5);
    let symbol = RaySymbol::new("AAPL");
    let name   = RayString::new("Alice");
    println!("Price: {price}\nRatio: {ratio}\nSymbol: {symbol}\nName: {name}");

    // Typed columns
    let prices: RayVector<i64> = RayVector::from_iter([100i64, 150, 200]);
    let quantities: RayVector<f64> = RayVector::from_iter([10.0, 20.0, 30.0]);
    let symbols = RayVector::<RaySymbol>::from_iter(["AAPL", "GOOG", "MSFT"]);
    println!("prices: {prices}");
    println!("quantities: {quantities}");
    println!("({} symbols)", symbols.len());

    // Heterogeneous list
    let mut list = RayList::new();
    list.push(42i64);
    list.push("hello");
    list.push(3.14f64);
    println!("List length: {}", list.len());

    // Dict (string keys are interned to symbols)
    let dict = RayDict::from_pairs([
        ("name", RayString::new("Alice").ptr().clone()),
        ("age",  RayI64::new(30).ptr().clone()),
    ])?;
    println!("dict has {} keys", dict.len());
    Ok(())
}
```

## Tables

### Building a table from Rust data

```rust
use rayforce::{RayTable, RayVector, RaySymbol, RayType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let employees = RayTable::from_dict([
        ("id",        RayVector::<i64>::from_iter([1i64, 2, 3, 4, 5]).as_ray_obj().clone()),
        ("name",      RayVector::<RaySymbol>::from_iter(["Alice", "Bob", "Charlie", "David", "Eve"]).as_ray_obj().clone()),
        ("dept",      RayVector::<RaySymbol>::from_iter(["Engineering", "Sales", "Engineering", "HR", "Engineering"]).as_ray_obj().clone()),
        ("salary",    RayVector::<i64>::from_iter([85000i64, 65000, 95000, 55000, 78000]).as_ray_obj().clone()),
    ])?;

    println!("Employees ({} rows × {} cols):\n{}",
        employees.len()?,
        employees.ncols(),
        employees);
    Ok(())
}
```

### Building a table inside Rayfall

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;

    let employees = rf.eval(r#"
        (table [id name dept salary hire_date]
            (list
                [1 2 3 4 5]
                [`Alice `Bob `Charlie `David `Eve]
                [`Engineering `Sales `Engineering `HR `Engineering]
                [85000 65000 95000 55000 78000]
                [2020.01.15 2019.06.20 2018.03.10 2021.09.01 2022.02.28]))
    "#)?;
    println!("{employees}");
    Ok(())
}
```

### Querying tables with the builder

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
    employees.save("employees")?;     // bind under the name "employees"

    // Filter
    let high_earners = SelectQuery::from("employees")
        .columns(["name", "salary"])
        .filter(Column::new("salary").gt(70000))
        .execute(&rf)?;
    println!("High earners:\n{high_earners}\n");

    // Aggregate
    let by_dept = SelectQuery::from("employees")
        .column("count",        Operation::Count(Column::new("name").into_expr()))
        .column("avg_salary",   Operation::Avg(Column::new("salary").into_expr()))
        .column("total_salary", Operation::Sum(Column::new("salary").into_expr()))
        .group_by(Column::new("dept"))
        .execute(&rf)?;
    println!("By department:\n{by_dept}\n");
    Ok(())
}
```

### Remote queries via IPC

```rust
use rayforce::{Connection, SelectQuery, Column};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Rayforce server. Auth is optional:
    //   Connection::connect_with_auth(host, port, Some(user), password).
    let conn = Connection::connect("127.0.0.1", 5000)?;

    // Build a query locally and ship its rendered Rayfall to the server.
    let q = SelectQuery::from("trades").filter(Column::new("price").gt(100));
    let result = conn.execute(&q.to_rayfall())?;
    println!("{result}");

    // The connection closes automatically when `conn` is dropped.
    Ok(())
}
```

## Financial data

### Trade analysis

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;

    rf.eval(r#"
        (set trades (table [trade_id sym time price qty side]
            (list
                [1 2 3 4 5 6 7 8]
                [`AAPL `MSFT `AAPL `GOOGL `MSFT `AAPL `GOOGL `MSFT]
                [09:30:01 09:30:05 09:31:00 09:31:30 09:32:00 09:32:15 09:33:00 09:33:30]
                [150.00 300.00 150.50 2800.00 301.00 149.75 2805.00 299.50]
                [100 50 200 25 75 150 30 100]
                [`buy `buy `sell `buy `sell `buy `sell `buy])))
    "#)?;
    println!("All trades:\n{}\n", rf.eval("trades")?);

    // Summary by symbol — VWAP and totals.
    let summary = rf.eval(r#"
        (select {from:trades by: sym
                 trade_count:(count trade_id)
                 total_qty:(sum qty)
                 total_value:(sum (* price qty))
                 avg_price:(avg price)
                 vwap:(% (sum (* price qty)) (sum qty))})
    "#)?;
    println!("Summary by symbol:\n{summary}\n");

    // Buy vs sell
    let by_side = rf.eval(r#"
        (select {from:trades by: side
                 count:(count trade_id)
                 value:(sum (* price qty))})
    "#)?;
    println!("By side:\n{by_side}");
    Ok(())
}
```

## Time series

### OHLC calculation

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;

    rf.eval(r#"
        (set ticks (table [sym time price]
            (list
                [`AAPL `AAPL `AAPL `AAPL `AAPL `AAPL `AAPL `AAPL]
                [09:30:00 09:30:15 09:30:30 09:30:45 09:31:00 09:31:15 09:31:30 09:31:45]
                [150.00 150.25 150.10 150.50 150.45 150.30 150.60 150.55])))
    "#)?;

    // Single-bucket OHLC for the whole window. Real time-bucketing
    // would group by `(xbar 60 time)` or similar.
    let ohlc = rf.eval(r#"
        (select {from:ticks
                 open:(first price) high:(max price)
                 low:(min price)   close:(last price)})
    "#)?;
    println!("OHLC:\n{ohlc}");
    Ok(())
}
```

## Joins

### Orders and customers

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rf = Rayforce::new()?;

    rf.eval(r#"
        (set customers (table [cust_id name region tier]
            (list
                [1 2 3]
                [`Acme `Beta `Gamma]
                [`East `West `East]
                [`Gold `Silver `Gold])))
    "#)?;

    rf.eval(r#"
        (set orders (table [order_id cust_id product qty price]
            (list
                [1001 1002 1003 1004 1005]
                [1 2 1 3 2]
                [`Widget `Gadget `Gizmo `Widget `Gadget]
                [10 5 3 20 8]
                [99.90 149.95 44.97 199.80 119.96])))
    "#)?;

    println!("Orders:\n{}\n", rf.eval("orders")?);
    println!("Customers:\n{}\n", rf.eval("customers")?);

    let enriched = rf.eval("(left-join [`cust_id] orders customers)")?;
    println!("Enriched orders:\n{enriched}\n");

    let by_tier = rf.eval(r#"
        (select {from:(left-join [`cust_id] orders customers)
                 by: tier
                 order_count:(count order_id)
                 total_revenue:(sum price)})
    "#)?;
    println!("Revenue by tier:\n{by_tier}");
    Ok(())
}
```

## Running examples

The crate ships two ready-to-run examples in `examples/`:

```bash
cargo run --example basic     # happy-path smoke test (scalars, vectors, lists, dicts, eval)
cargo run --example repl      # minimal read → eval → print loop, :q to quit

# Release build:
cargo run --release --example basic
```

Iterating against a local rayforce checkout? Set
`RAYFORCE_GITHUB=file:///path/to/rayforce` to avoid hitting GitHub.

## More resources

- **[API Reference](../api/overview.md)** — full API docs.
- **[Get Started](../get-started/overview.md)** — installation and setup.
- **[GitHub](https://github.com/RayforceDB/rayforce-rs)** — source and issues.
