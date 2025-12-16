# Update Query

The UPDATE query modifies existing data in tables.

## Basic Syntax

```rust
ray.eval(r#"
    (update {
        col1: new_expr1
        col2: new_expr2
        from: table
        where: condition
        by: group_columns})
"#)?;
```

## Simple Updates

### Update All Rows

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

// Give everyone a 10% raise
let updated = ray.eval(r#"
    (update {
        salary: (* salary 1.1)
        from: employees})
"#)?;
```

### Update Single Column

```rust
let updated = ray.eval(r#"
    (update {
        salary: 80000
        from: employees
        where: (= name "Alice")})
"#)?;
```

## Conditional Updates

### WHERE Clause

```rust
// Update based on condition
let updated = ray.eval(r#"
    (update {
        salary: (* salary 1.15)
        from: employees
        where: (= dept 'IT)})
"#)?;

// Multiple conditions
let updated = ray.eval(r#"
    (update {
        salary: (* salary 1.2)
        from: employees
        where: (& (= dept 'IT) (> salary 70000))})
"#)?;
```

## Computed Updates

### Based on Current Values

```rust
// Percentage increase
let updated = ray.eval(r#"
    (update {
        salary: (* salary 1.05)
        from: employees})
"#)?;

// Add fixed amount
let updated = ray.eval(r#"
    (update {
        salary: (+ salary 5000)
        from: employees
        where: (> salary 60000)})
"#)?;
```

### Based on Other Columns

```rust
// Create bonus column
ray.eval(r#"
    (set employees (table [name dept salary performance]
        (list
            (list "Alice" "Bob" "Charlie")
            ['IT 'HR 'IT]
            [75000 65000 85000]
            [0.9 0.8 0.95])))
"#)?;

// Update salary based on performance
let updated = ray.eval(r#"
    (update {
        salary: (* salary (+ 1 (* performance 0.1)))
        from: employees})
"#)?;
```

## Group-Based Updates

### Update by Group

```rust
// Different updates per department
let updated = ray.eval(r#"
    (update {
        salary: (+ salary 1000)
        from: employees
        by: dept
        where: (> salary 55000)})
"#)?;
```

## Multiple Column Updates

### Update Several Columns

```rust
let updated = ray.eval(r#"
    (update {
        salary: (* salary 1.1)
        dept: 'Engineering
        from: employees
        where: (= dept 'IT)})
"#)?;
```

## In-Place Updates

### Mutating the Original Table

Use quoted table name for in-place mutation:

```rust
// In-place update (modifies original)
ray.eval(r#"
    (update {
        salary: (* salary 1.1)
        from: 'employees
        where: (= dept 'IT)})
"#)?;
```

## Return Value vs Mutation

### Returning New Table

```rust
// Returns new table, original unchanged
let new_table = ray.eval(r#"
    (update {
        salary: (* salary 1.1)
        from: employees})
"#)?;
// employees still has original values
```

### Assigning Result

```rust
// Update and reassign
ray.eval(r#"
    (set employees (update {
        salary: (* salary 1.1)
        from: employees}))
"#)?;
```

## Complete Example

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create inventory table
    ray.eval(r#"
        (set inventory (table [product qty price category]
            (list
                (list "Widget" "Gadget" "Gizmo" "Doohickey")
                [100 50 75 200]
                [9.99 24.99 14.99 4.99]
                ['electronics 'electronics 'toys 'misc])))
    "#)?;
    
    println!("Before update:");
    println!("{}", ray.eval("inventory")?);
    
    // Apply price increases by category
    // Electronics: 10% increase
    // Toys: 5% increase
    // Misc: no change
    
    ray.eval(r#"
        (set inventory (update {
            price: (* price 1.1)
            from: inventory
            where: (= category 'electronics)}))
    "#)?;
    
    ray.eval(r#"
        (set inventory (update {
            price: (* price 1.05)
            from: inventory
            where: (= category 'toys)}))
    "#)?;
    
    // Increase quantity for low stock items
    ray.eval(r#"
        (set inventory (update {
            qty: (+ qty 50)
            from: inventory
            where: (< qty 100)}))
    "#)?;
    
    println!("\nAfter updates:");
    println!("{}", ray.eval("inventory")?);
    
    Ok(())
}
```

## Query Type

The `RayUpdateQuery` type provides a programmatic interface:

```rust
use rayforce::{RayTable, RayUpdateQuery};

// Query builder pattern (conceptual)
let result = table
    .update()
    .set("salary", "(* salary 1.1)")
    .filter("(= dept 'IT)")
    .execute()?;
```

## Error Handling

Updates can fail if:
- Column doesn't exist
- Type mismatch in expression
- Invalid expression syntax

```rust
match ray.eval("(update { invalid_col: 1 from: employees })") {
    Ok(result) => println!("Updated: {}", result),
    Err(e) => eprintln!("Update failed: {}", e),
}
```

## Best Practices

1. **Backup before bulk updates**: Save table state before large updates
2. **Test with SELECT first**: Verify WHERE clause selects correct rows
3. **Use transactions**: For complex multi-step updates
4. **Validate types**: Ensure new values match column types

## Next Steps

- **[Insert Query](insert.md)** - Add new data
- **[Upsert Query](upsert.md)** - Update or insert
- **[Select Query](select.md)** - Query data

