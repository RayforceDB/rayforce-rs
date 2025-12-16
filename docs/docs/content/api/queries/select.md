# Select Query

The SELECT query retrieves and transforms data from tables.

## Basic Syntax

```rust
ray.eval(r#"
    (select {
        col1: expr1
        col2: expr2
        from: table
        where: condition
        by: group_columns})
"#)?;
```

## Simple Selection

### Select All Columns

```rust
let ray = Rayforce::new()?;

// Create sample table
ray.eval(r#"
    (set employees (table [name dept salary]
        (list
            (list "Alice" "Bob" "Charlie")
            ['IT 'HR 'IT]
            [75000 65000 85000])))
"#)?;

// Select all
let result = ray.eval("employees")?;
```

### Select Specific Columns

```rust
let result = ray.eval(r#"
    (select {
        name: name
        salary: salary
        from: employees})
"#)?;
```

## Filtering with WHERE

### Simple Conditions

```rust
// Greater than
let high_earners = ray.eval(r#"
    (select {
        name: name
        salary: salary
        from: employees
        where: (> salary 70000)})
"#)?;

// Equality
let it_dept = ray.eval(r#"
    (select {
        name: name
        from: employees
        where: (= dept 'IT)})
"#)?;
```

### Compound Conditions

```rust
// AND condition
let result = ray.eval(r#"
    (select {
        name: name
        from: employees
        where: (& (= dept 'IT) (> salary 70000))})
"#)?;

// OR condition
let result = ray.eval(r#"
    (select {
        name: name
        from: employees
        where: (| (= dept 'IT) (= dept 'HR))})
"#)?;
```

### IN clause

```rust
let result = ray.eval(r#"
    (select {
        name: name
        from: employees
        where: (in dept ['IT 'Sales])})
"#)?;
```

## Computed Columns

### Arithmetic

```rust
let result = ray.eval(r#"
    (select {
        name: name
        salary: salary
        bonus: (* salary 0.1)
        total: (+ salary (* salary 0.1))
        from: employees})
"#)?;
```

### String Operations

```rust
let result = ray.eval(r#"
    (select {
        upper_name: (upper name)
        name_len: (count name)
        from: employees})
"#)?;
```

## Aggregation

### Basic Aggregates

```rust
// Sum
let total = ray.eval(r#"
    (select {
        total_salary: (sum salary)
        from: employees})
"#)?;

// Average
let avg = ray.eval(r#"
    (select {
        avg_salary: (avg salary)
        from: employees})
"#)?;

// Count
let count = ray.eval(r#"
    (select {
        emp_count: (count name)
        from: employees})
"#)?;

// Min/Max
let stats = ray.eval(r#"
    (select {
        min_salary: (min salary)
        max_salary: (max salary)
        from: employees})
"#)?;
```

### Multiple Aggregates

```rust
let summary = ray.eval(r#"
    (select {
        count: (count name)
        total: (sum salary)
        average: (avg salary)
        minimum: (min salary)
        maximum: (max salary)
        from: employees})
"#)?;
```

## GROUP BY

### Single Column Grouping

```rust
let by_dept = ray.eval(r#"
    (select {
        dept: dept
        count: (count name)
        avg_salary: (avg salary)
        from: employees
        by: dept})
"#)?;
```

### Multiple Column Grouping

```rust
let by_dept_year = ray.eval(r#"
    (select {
        dept: dept
        year: year
        count: (count name)
        from: employees
        by: [dept year]})
"#)?;
```

### Aggregation with Filter

```rust
let result = ray.eval(r#"
    (select {
        dept: dept
        high_earner_count: (count name)
        from: employees
        where: (> salary 70000)
        by: dept})
"#)?;
```

## Ordering

### Order By

```rust
// Ascending
let ordered = ray.eval(r#"
    (asc (select {
        name: name
        salary: salary
        from: employees}) 'salary)
"#)?;

// Descending
let ordered = ray.eval(r#"
    (desc (select {
        name: name
        salary: salary
        from: employees}) 'salary)
"#)?;
```

## Limiting Results

### Take/Drop

```rust
// First N rows
let top5 = ray.eval("(take 5 employees)")?;

// Skip first N rows
let rest = ray.eval("(drop 5 employees)")?;

// Combine for pagination
let page2 = ray.eval("(take 10 (drop 10 employees))")?;
```

## Subqueries

### Nested Select

```rust
let result = ray.eval(r#"
    (select {
        dept: dept
        total: (sum salary)
        from: (select {
            dept: dept
            salary: salary
            from: employees
            where: (> salary 50000)})
        by: dept})
"#)?;
```

## Complete Example

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create trades table
    ray.eval(r#"
        (set trades (table [sym price qty time trader]
            (list
                ['AAPL 'MSFT 'AAPL 'GOOGL 'MSFT 'AAPL]
                [150.0 300.0 151.0 2800.0 301.0 149.5]
                [100 50 200 25 75 150]
                [09:30:00 09:31:00 09:32:00 09:33:00 09:34:00 09:35:00]
                (list "Alice" "Bob" "Alice" "Charlie" "Bob" "Alice"))))
    "#)?;
    
    // Complex query
    let result = ray.eval(r#"
        (select {
            sym: sym
            trade_count: (count price)
            total_qty: (sum qty)
            total_value: (sum (* price qty))
            avg_price: (avg price)
            from: trades
            where: (> qty 50)
            by: sym})
    "#)?;
    
    println!("Trade Summary:\n{}", result);
    
    Ok(())
}
```

## Query Type

The `RaySelectQuery` type provides a programmatic interface:

```rust
use rayforce::{RayTable, RaySelectQuery};

// Query builder pattern (conceptual)
let query = table
    .select()
    .columns(["name", "salary"])
    .filter("(> salary 70000)")
    .group_by("dept")
    .execute()?;
```

## Performance Tips

1. **Filter early**: Apply WHERE clauses before aggregation
2. **Select only needed columns**: Avoid selecting unused columns
3. **Use appropriate indexes**: Create keyed tables for frequently filtered columns
4. **Batch operations**: Process large datasets in chunks when possible

## Next Steps

- **[Update Query](update.md)** - Modify existing data
- **[Insert Query](insert.md)** - Add new data
- **[Joins](joins.md)** - Combine tables

