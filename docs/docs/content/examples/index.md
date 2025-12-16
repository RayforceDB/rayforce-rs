# Examples

Code examples demonstrating rayforce-rs usage.

## Basic Examples

### Hello World

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    let result = ray.eval("(+ 1 2 3)")?;
    println!("1 + 2 + 3 = {}", result);
    
    Ok(())
}
```

### Working with Types

```rust
use rayforce::{
    Rayforce, RayI64, RayF64, RaySymbol, 
    RayVector, RayList, RayDict, RayObj
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Scalars
    let price = RayI64::from_value(150);
    let ratio = RayF64::from_value(1.5);
    let symbol = RaySymbol::new("AAPL");
    
    println!("Price: {}", price);
    println!("Ratio: {}", ratio);
    println!("Symbol: {}", symbol);
    
    // Vectors
    let prices: RayVector<i64> = RayVector::from_iter([100, 150, 200]);
    let quantities: RayVector<f64> = RayVector::from_iter([10.0, 20.0, 30.0]);
    
    println!("Prices: {}", prices);
    
    // Lists
    let mut list = RayList::new();
    list.push(RayObj::from(42_i64));
    list.push(RayObj::from("hello"));
    list.push(RayObj::from(3.14_f64));
    
    println!("List length: {}", list.len());
    
    // Dictionaries
    let dict = RayDict::from_pairs([
        (RaySymbol::new("name"), RayObj::from("Alice")),
        (RaySymbol::new("age"), RayObj::from(30_i64)),
    ]);
    
    println!("Dict keys: {:?}", dict.keys());
    
    Ok(())
}
```

## Table Operations

### Creating Tables

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create employees table
    let employees = ray.eval(r#"
        (table [id name dept salary hire_date]
            (list
                [1 2 3 4 5]
                (list "Alice" "Bob" "Charlie" "David" "Eve")
                ['Engineering 'Sales 'Engineering 'HR 'Engineering]
                [85000 65000 95000 55000 78000]
                [2020.01.15 2019.06.20 2018.03.10 2021.09.01 2022.02.28]))
    "#)?;
    
    println!("Employees table:");
    println!("{}", employees);
    
    Ok(())
}
```

### Querying Tables

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create and store table
    ray.eval(r#"
        (set employees (table [name dept salary]
            (list
                (list "Alice" "Bob" "Charlie" "David")
                ['IT 'HR 'IT 'Sales]
                [75000 65000 85000 70000])))
    "#)?;
    
    // Filter
    println!("High earners:");
    let high_earners = ray.eval(r#"
        (select {
            name: name
            salary: salary
            from: employees
            where: (> salary 70000)})
    "#)?;
    println!("{}\n", high_earners);
    
    // Aggregate
    println!("By department:");
    let by_dept = ray.eval(r#"
        (select {
            dept: dept
            count: (count name)
            avg_salary: (avg salary)
            total_salary: (sum salary)
            from: employees
            by: dept})
    "#)?;
    println!("{}", by_dept);
    
    Ok(())
}
```

## Financial Data Example

### Trade Analysis

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create trades table
    ray.eval(r#"
        (set trades (table [trade_id sym time price qty side]
            (list
                [1 2 3 4 5 6 7 8]
                ['AAPL 'MSFT 'AAPL 'GOOGL 'MSFT 'AAPL 'GOOGL 'MSFT]
                [09:30:01 09:30:05 09:31:00 09:31:30 09:32:00 09:32:15 09:33:00 09:33:30]
                [150.00 300.00 150.50 2800.00 301.00 149.75 2805.00 299.50]
                [100 50 200 25 75 150 30 100]
                ['buy 'buy 'sell 'buy 'sell 'buy 'sell 'buy])))
    "#)?;
    
    println!("All trades:");
    println!("{}\n", ray.eval("trades")?);
    
    // Trade summary by symbol
    let summary = ray.eval(r#"
        (select {
            sym: sym
            trade_count: (count trade_id)
            total_qty: (sum qty)
            total_value: (sum (* price qty))
            avg_price: (avg price)
            vwap: (% (sum (* price qty)) (sum qty))
            from: trades
            by: sym})
    "#)?;
    
    println!("Summary by symbol:");
    println!("{}\n", summary);
    
    // Buy vs Sell
    let by_side = ray.eval(r#"
        (select {
            side: side
            count: (count trade_id)
            value: (sum (* price qty))
            from: trades
            by: side})
    "#)?;
    
    println!("By side:");
    println!("{}", by_side);
    
    Ok(())
}
```

## Time Series Example

### OHLC Calculation

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create tick data
    ray.eval(r#"
        (set ticks (table [sym time price]
            (list
                ['AAPL 'AAPL 'AAPL 'AAPL 'AAPL 'AAPL 'AAPL 'AAPL]
                [09:30:00 09:30:15 09:30:30 09:30:45 09:31:00 09:31:15 09:31:30 09:31:45]
                [150.00 150.25 150.10 150.50 150.45 150.30 150.60 150.55])))
    "#)?;
    
    // Group by minute for OHLC
    // Note: This is simplified - real OHLC would use proper time bucketing
    let ohlc = ray.eval(r#"
        (select {
            open: (first price)
            high: (max price)
            low: (min price)
            close: (last price)
            from: ticks})
    "#)?;
    
    println!("OHLC:");
    println!("{}", ohlc);
    
    Ok(())
}
```

## Join Example

### Orders and Customers

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create customers
    ray.eval(r#"
        (set customers (table [cust_id name region tier]
            (list
                [1 2 3]
                (list "Acme Corp" "Beta Inc" "Gamma LLC")
                ['East 'West 'East]
                ['Gold 'Silver 'Gold])))
    "#)?;
    
    // Create orders
    ray.eval(r#"
        (set orders (table [order_id cust_id product qty price]
            (list
                [1001 1002 1003 1004 1005]
                [1 2 1 3 2]
                (list "Widget" "Gadget" "Gizmo" "Widget" "Gadget")
                [10 5 3 20 8]
                [99.90 149.95 44.97 199.80 119.96])))
    "#)?;
    
    println!("Orders:");
    println!("{}\n", ray.eval("orders")?);
    
    println!("Customers:");
    println!("{}\n", ray.eval("customers")?);
    
    // Join orders with customers
    let enriched = ray.eval(r#"
        (left-join [cust_id] orders customers)
    "#)?;
    
    println!("Enriched orders:");
    println!("{}\n", enriched);
    
    // Revenue by customer tier
    let by_tier = ray.eval(r#"
        (select {
            tier: tier
            order_count: (count order_id)
            total_revenue: (sum price)
            from: (left-join [cust_id] orders customers)
            by: tier})
    "#)?;
    
    println!("Revenue by tier:");
    println!("{}", by_tier);
    
    Ok(())
}
```

## Running Examples

All examples are in the `examples/` directory. Run with:

```bash
# Run specific example
cargo run --example basic

# Run with release optimizations
cargo run --release --example basic
```

## More Resources

- **[API Reference](../api/overview.md)** - Complete API documentation
- **[Get Started](../get-started/overview.md)** - Installation and setup
- **[GitHub](https://github.com/RayforceDB/rayforce-rs)** - Source code and issues

