# Quick Start

This guide will walk you through building your first application with rayforce-rs.

## Basic Setup

Every rayforce-rs application starts by initializing the runtime:

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the RayforceDB runtime
    let ray = Rayforce::new()?;
    
    // Your code here...
    
    Ok(())
}
```

The `Rayforce::new()` call initializes the database engine. The runtime is automatically cleaned up when the `Rayforce` instance goes out of scope.

## Working with Scalars

RayforceDB has several scalar types, all available with the `Ray` prefix:

```rust
use rayforce::{RayI64, RayF64, RaySymbol, RayObj};

// Integer types
let a = RayI64::from_value(42);
let b = RayI64::from_value(100);

// Floating point
let pi = RayF64::from_value(3.14159);

// Symbols (interned strings)
let sym = RaySymbol::new("price");

// Generic object from primitives
let obj = RayObj::from(42_i64);
let obj2 = RayObj::from("hello");
```

## Creating Vectors

Vectors are homogeneous arrays of values:

```rust
use rayforce::RayVector;

// From an iterator
let prices: RayVector<i64> = RayVector::from_iter([100, 200, 300, 400]);

// From a slice
let quantities: RayVector<f64> = RayVector::from_iter([1.5, 2.0, 3.5]);

// Access length
println!("Count: {}", prices.len());
```

## Creating Lists

Lists are heterogeneous containers:

```rust
use rayforce::{RayList, RayObj};

let mut list = RayList::new();
list.push(RayObj::from(42_i64));
list.push(RayObj::from("hello"));
list.push(RayObj::from(3.14_f64));

// Access by index
let first = list.get(0);

// Iterate
for item in list.iter() {
    println!("{}", item);
}
```

## Creating Dictionaries

Dictionaries map keys to values:

```rust
use rayforce::{RayDict, RaySymbol, RayObj};

let dict = RayDict::from_pairs([
    (RaySymbol::new("name"), RayObj::from("Alice")),
    (RaySymbol::new("age"), RayObj::from(30_i64)),
    (RaySymbol::new("salary"), RayObj::from(75000_i64)),
]);

// Access by key
let name = dict.get(&RaySymbol::new("name"));
```

## Evaluating Expressions

RayforceDB uses a Lisp-like expression syntax:

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Simple arithmetic
    let sum = ray.eval("(+ 1 2 3)")?;
    println!("Sum: {}", sum);  // → 6
    
    // Nested expressions
    let complex = ray.eval("(* (+ 1 2) (- 10 5))")?;
    println!("Result: {}", complex);  // → 15
    
    // Vector operations
    let vec_sum = ray.eval("(sum [1 2 3 4 5])")?;
    println!("Vector sum: {}", vec_sum);  // → 15
    
    // Aggregations
    let avg = ray.eval("(avg [10 20 30 40 50])")?;
    println!("Average: {}", avg);  // → 30
    
    Ok(())
}
```

## Working with Tables

Tables are the core data structure for analytics:

```rust
use rayforce::{Rayforce, RayTable};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create a table via evaluation
    let employees = ray.eval(r#"
        (table [name dept salary]
            (list
                (list "Alice" "Bob" "Charlie")
                ['IT 'HR 'IT]
                [75000 65000 85000]))
    "#)?;
    
    println!("Employees:\n{}", employees);
    
    Ok(())
}
```

## Querying Tables

Perform SQL-like queries with a fluent API:

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create sample data
    ray.eval(r#"
        (set employees
            (table [name dept salary]
                (list
                    (list "Alice" "Bob" "Charlie" "David")
                    ['IT 'HR 'IT 'Sales]
                    [75000 65000 85000 70000])))
    "#)?;
    
    // SELECT with WHERE
    let high_earners = ray.eval(r#"
        (select {
            name: name
            salary: salary
            from: employees
            where: (> salary 70000)})
    "#)?;
    println!("High earners:\n{}", high_earners);
    
    // GROUP BY with aggregation
    let by_dept = ray.eval(r#"
        (select {
            avg_salary: (avg salary)
            headcount: (count name)
            from: employees
            by: dept})
    "#)?;
    println!("By department:\n{}", by_dept);
    
    Ok(())
}
```

## Complete Example

Here's a complete example putting it all together:

```rust
use rayforce::{Rayforce, RayVector, RayList, RayObj, RaySymbol};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize runtime
    let ray = Rayforce::new()?;
    println!("RayforceDB {} initialized", ray.version());
    
    // Create some data
    let prices: RayVector<i64> = RayVector::from_iter([100, 150, 200, 175, 225]);
    let symbols: RayVector<RaySymbol> = RayVector::from_iter([
        RaySymbol::new("AAPL"),
        RaySymbol::new("GOOGL"),
        RaySymbol::new("MSFT"),
        RaySymbol::new("AMZN"),
        RaySymbol::new("META"),
    ]);
    
    // Create a trades table
    let trades = ray.eval(r#"
        (table [symbol price quantity time]
            (list
                ['AAPL 'GOOGL 'MSFT 'AAPL 'GOOGL]
                [150.25 2800.50 300.75 151.00 2805.25]
                [100 50 200 150 75]
                [09:30:00 09:31:00 09:32:00 09:33:00 09:34:00]))
    "#)?;
    
    println!("Trades:\n{}", trades);
    
    // Calculate total value per symbol
    let totals = ray.eval(r#"
        (select {
            total_value: (sum (* price quantity))
            trade_count: (count symbol)
            from: trades
            by: symbol})
    "#)?;
    
    println!("\nTotals by symbol:\n{}", totals);
    
    Ok(())
}
```

## What's Next?

- **[API Reference](../api/overview.md)** - Complete API documentation
- **[Types](../api/types/scalars.md)** - Detailed type system guide
- **[Queries](../api/queries/select.md)** - Advanced query operations
- **[Examples](../examples/index.md)** - More code examples

