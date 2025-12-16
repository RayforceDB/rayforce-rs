# Table Type

Tables are the primary data structure in RayforceDB for analytical workloads. A table is a dictionary mapping column names to vectors.

## Creating Tables

### Via Evaluation

The most flexible way to create tables:

```rust
use rayforce::Rayforce;

let ray = Rayforce::new()?;

let table = ray.eval(r#"
    (table [name age salary]
        (list
            (list "Alice" "Bob" "Charlie")
            [25 30 35]
            [50000 60000 70000]))
"#)?;

println!("{}", table);
```

### Table Structure

A table consists of:
- **Column names**: Symbols identifying each column
- **Column data**: Vectors of uniform type
- **Key columns**: Optional columns for indexing (keyed tables)

```
Table: employees
┌─────────┬─────┬────────┐
│ name    │ age │ salary │
├─────────┼─────┼────────┤
│ "Alice" │ 25  │ 50000  │
│ "Bob"   │ 30  │ 60000  │
│ "Charlie"│ 35  │ 70000  │
└─────────┴─────┴────────┘
```

## Table Operations

### Accessing Dimensions

```rust
use rayforce::Rayforce;

let ray = Rayforce::new()?;

let table = ray.eval(r#"
    (table [a b c] (list [1 2] [3 4] [5 6]))
"#)?;

// Number of rows
let rows = ray.eval("(count a)")?;

// Number of columns  
let cols = ray.eval("(count (cols table))")?;
```

### Accessing Columns

```rust
// Get column by name
let salaries = ray.eval("(. employees 'salary)")?;

// Get multiple columns
let subset = ray.eval("(. employees 'name 'salary)")?;
```

### Accessing Rows

```rust
// Get first row
let first = ray.eval("(first employees)")?;

// Get last row
let last = ray.eval("(last employees)")?;

// Get row by index
let row = ray.eval("(employees 1)")?;

// Get range of rows
let rows = ray.eval("(take 5 employees)")?;
```

## Column Reference

The `RayColumn` type references table columns in queries.

```rust
use rayforce::RayColumn;

let col = RayColumn::new("salary");
let col2 = RayColumn::new("department");
```

## Expression Building

The `RayExpression` type builds query expressions.

```rust
use rayforce::RayExpression;

// Simple column reference
let expr = RayExpression::column("salary");

// Arithmetic expression
let expr = RayExpression::from("(* salary 1.1)");

// Aggregation
let expr = RayExpression::from("(avg salary)");
```

## Keyed Tables

Keyed tables have one or more key columns for efficient lookups.

```rust
let ray = Rayforce::new()?;

// Create keyed table (first column is key)
let keyed = ray.eval(r#"
    (key (table [id name salary]
        (list [1 2 3]
              (list "Alice" "Bob" "Charlie")
              [50000 60000 70000])) 1)
"#)?;

// Lookup by key
let row = ray.eval("(keyed 2)")?;  // Get row where id=2
```

## Table I/O

### Saving Tables

```rust
// Save to binary format
ray.eval("(save 'data/employees.ray employees)")?;

// Save to CSV (if supported)
ray.eval("(save 'data/employees.csv employees)")?;
```

### Loading Tables

```rust
// Load from file
let loaded = ray.eval("(load 'data/employees.ray)")?;
```

## Table Manipulation

### Adding Rows

```rust
// Insert single row
ray.eval(r#"
    (insert employees (list "David" 28 55000))
"#)?;

// Insert multiple rows
ray.eval(r#"
    (insert employees (list
        (list "Eve" "Frank")
        [32 29]
        [62000 58000]))
"#)?;
```

### Updating Rows

```rust
// Update with condition
ray.eval(r#"
    (update {
        salary: (* salary 1.1)
        from: employees
        where: (> age 30)})
"#)?;
```

### Joining Tables

```rust
let result = ray.eval(r#"
    (left-join [dept_id] employees departments)
"#)?;
```

## Query Integration

Tables integrate with the query system:

```rust
use rayforce::Rayforce;

let ray = Rayforce::new()?;

// Create table
ray.eval(r#"
    (set trades (table [sym price qty time]
        (list
            ['AAPL 'MSFT 'AAPL 'GOOGL]
            [150.0 300.0 151.0 2800.0]
            [100 50 200 25]
            [09:30:00 09:31:00 09:32:00 09:33:00])))
"#)?;

// Select query
let result = ray.eval(r#"
    (select {
        sym: sym
        total: (* price qty)
        from: trades
        where: (> qty 50)})
"#)?;

// Aggregation
let summary = ray.eval(r#"
    (select {
        total_qty: (sum qty)
        avg_price: (avg price)
        from: trades
        by: sym})
"#)?;
```

## Performance Tips

### Column Order

Place frequently accessed columns first:

```rust
// Good: commonly queried columns first
let table = ray.eval(r#"
    (table [price qty sym metadata]
        (list prices quantities symbols metadata))
"#)?;
```

### Data Types

Use appropriate types for columns:

| Data | Recommended Type |
|------|------------------|
| IDs | `RayI64` |
| Prices | `RayF64` |
| Quantities | `RayI64` |
| Categories | `RaySymbol` |
| Text | `RayString` |
| Dates | `RayDate` |
| Timestamps | `RayTimestamp` |

### Indexing

Create keyed tables for frequent lookups:

```rust
// Keyed by first column
let keyed = ray.eval("(key table 1)")?;

// Keyed by multiple columns  
let keyed = ray.eval("(key table 2)")?;  // First 2 columns as key
```

## Next Steps

- **[Select Query](../queries/select.md)** - Querying tables
- **[Update Query](../queries/update.md)** - Modifying data
- **[Insert Query](../queries/insert.md)** - Adding data
- **[Joins](../queries/joins.md)** - Combining tables

