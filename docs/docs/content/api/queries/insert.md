# Insert Query

The INSERT query adds new rows to tables.

## Basic Syntax

```rust
ray.eval("(insert table data)")?;
```

## Single Row Insert

### Insert One Record

```rust
let ray = Rayforce::new()?;

// Create sample table
ray.eval(r#"
    (set employees (table [name dept salary]
        (list
            (list "Alice" "Bob")
            ['IT 'HR]
            [75000 65000])))
"#)?;

// Insert single row
let updated = ray.eval(r#"
    (insert employees (list "Charlie" 'IT 85000))
"#)?;
```

### With Reassignment

```rust
ray.eval(r#"
    (set employees (insert employees (list "Charlie" 'IT 85000)))
"#)?;
```

## Multiple Row Insert

### Insert Several Rows

```rust
let updated = ray.eval(r#"
    (insert employees (list
        (list "Charlie" "David" "Eve")
        ['IT 'Sales 'HR]
        [85000 70000 68000]))
"#)?;
```

## In-Place Insert

### Mutating Original Table

Use quoted table name for in-place insertion:

```rust
// Modifies original table directly
ray.eval(r#"
    (insert 'employees (list "Frank" 'IT 72000))
"#)?;
```

## Insert from Query

### Insert Select Results

```rust
// Insert results from another table
ray.eval(r#"
    (insert target_table
        (select {
            name: name
            dept: dept
            salary: salary
            from: source_table
            where: (> salary 70000)}))
"#)?;
```

## Type Safety

### Matching Column Types

The inserted data must match column types:

```rust
// Table has [name:string, dept:symbol, salary:i64]

// Correct types
ray.eval(r#"(insert employees (list "Alice" 'IT 75000))"#)?;

// Wrong: salary should be integer, not string
// ray.eval(r#"(insert employees (list "Alice" 'IT "seventy-five"))"#)?;
// This will fail!
```

## Complete Example

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create orders table
    ray.eval(r#"
        (set orders (table [order_id customer product qty price]
            (list
                [1001 1002]
                ['CUST_A 'CUST_B]
                ['Widget 'Gadget]
                [10 5]
                [99.90 124.95])))
    "#)?;
    
    println!("Initial orders:");
    println!("{}\n", ray.eval("orders")?);
    
    // Insert single order
    ray.eval(r#"
        (set orders (insert orders (list 1003 'CUST_A 'Gizmo 3 44.97)))
    "#)?;
    
    // Insert multiple orders
    ray.eval(r#"
        (set orders (insert orders (list
            [1004 1005 1006]
            ['CUST_C 'CUST_B 'CUST_A]
            ['Widget 'Widget 'Gadget]
            [20 15 8]
            [199.80 149.85 199.92])))
    "#)?;
    
    println!("After inserts:");
    println!("{}\n", ray.eval("orders")?);
    
    // Summary
    let summary = ray.eval(r#"
        (select {
            customer: customer
            order_count: (count order_id)
            total_qty: (sum qty)
            total_value: (sum price)
            from: orders
            by: customer})
    "#)?;
    
    println!("Order summary by customer:");
    println!("{}", summary);
    
    Ok(())
}
```

## Batch Operations

### Efficient Bulk Insert

For large datasets, insert in batches:

```rust
// Insert large dataset efficiently
let batch_size = 1000;
for chunk in data.chunks(batch_size) {
    ray.eval(&format!("(insert 'table {})", chunk_to_list(chunk)))?;
}
```

## Query Type

The `RayInsertQuery` type provides a programmatic interface:

```rust
use rayforce::{RayTable, RayInsertQuery};

// Query builder pattern (conceptual)
let result = table
    .insert()
    .values(["Alice", "IT", 75000])
    .execute()?;
```

## Error Cases

Insert will fail if:
- Column count mismatch
- Type mismatch
- Table doesn't exist

```rust
// Handle errors
match ray.eval("(insert employees (list \"Alice\"))") {
    Ok(_) => println!("Insert successful"),
    Err(e) => eprintln!("Insert failed: {}", e),
}
```

## Best Practices

1. **Validate data before insert**: Check types and constraints
2. **Use batch inserts**: More efficient than row-by-row
3. **Consider keyed tables**: For automatic duplicate handling
4. **Use transactions**: For consistency in multi-row inserts

## Performance Tips

### Append vs Insert

For append-heavy workloads:

```rust
// Append is optimized for adding to end
ray.eval("(insert 'table new_data)")?;
```

### Pre-allocate

For known sizes:

```rust
// Create table with expected capacity
ray.eval(r#"
    (set large_table (table [a b c]
        (list
            (take 0 (til 1000000))
            (take 0 (til 1000000))
            (take 0 (til 1000000)))))
"#)?;
```

## Next Steps

- **[Upsert Query](upsert.md)** - Update or insert
- **[Update Query](update.md)** - Modify existing data
- **[Select Query](select.md)** - Query data

