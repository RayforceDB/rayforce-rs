# Upsert Query

The UPSERT (UPDATE or INSERT) query updates existing rows or inserts new ones based on key matching.

## Basic Syntax

```rust
ray.eval("(upsert table key_count data)")?;
```

Where:
- `table` - Target table
- `key_count` - Number of key columns (from left)
- `data` - Data to upsert

## How Upsert Works

1. For each row in data:
   - If key exists in table → UPDATE
   - If key doesn't exist → INSERT

## Simple Upsert

### Update or Insert

```rust
let ray = Rayforce::new()?;

// Create keyed table (id is the key)
ray.eval(r#"
    (set employees (table [id name salary]
        (list
            [1 2]
            (list "Alice" "Bob")
            [75000 65000])))
"#)?;

// Upsert: updates id=2, inserts id=3
let result = ray.eval(r#"
    (upsert employees 1 (list
        [2 3]
        (list "Bob-Updated" "Charlie")
        [68000 85000]))
"#)?;
```

Result:
```
id | name         | salary
---+--------------+--------
1  | Alice        | 75000
2  | Bob-Updated  | 68000
3  | Charlie      | 85000
```

## Single Row Upsert

### Upsert One Record

```rust
// Update existing
ray.eval(r#"
    (set employees (upsert employees 1 (list 1 "Alice-New" 80000)))
"#)?;

// Insert new
ray.eval(r#"
    (set employees (upsert employees 1 (list 4 "David" 70000)))
"#)?;
```

## Multi-Key Upsert

### Composite Keys

```rust
// Table with composite key (sym, date)
ray.eval(r#"
    (set prices (table [sym date price]
        (list
            ['AAPL 'AAPL 'MSFT]
            [2024.01.01 2024.01.02 2024.01.01]
            [150.0 151.0 300.0])))
"#)?;

// Upsert with 2-column key
let result = ray.eval(r#"
    (upsert prices 2 (list
        ['AAPL 'MSFT]
        [2024.01.02 2024.01.02]
        [152.0 305.0]))
"#)?;
```

## In-Place Upsert

### Mutating Original Table

```rust
// Modifies table directly
ray.eval(r#"
    (upsert 'employees 1 (list 2 "Bob-Modified" 70000))
"#)?;
```

## Complete Example

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create inventory with product_id as key
    ray.eval(r#"
        (set inventory (table [product_id name qty price]
            (list
                [101 102 103]
                (list "Widget" "Gadget" "Gizmo")
                [100 50 75]
                [9.99 24.99 14.99])))
    "#)?;
    
    println!("Initial inventory:");
    println!("{}\n", ray.eval("inventory")?);
    
    // Upsert: restock existing, add new products
    ray.eval(r#"
        (set inventory (upsert inventory 1 (list
            [102 103 104 105]
            (list "Gadget" "Gizmo-v2" "Doohickey" "Thingamajig")
            [100 80 200 150]
            [24.99 16.99 4.99 7.99])))
    "#)?;
    
    println!("After upsert:");
    println!("{}\n", ray.eval("inventory")?);
    
    Ok(())
}
```

Output:
```
Initial inventory:
product_id | name   | qty | price
-----------+--------+-----+-------
101        | Widget | 100 | 9.99
102        | Gadget | 50  | 24.99
103        | Gizmo  | 75  | 14.99

After upsert:
product_id | name        | qty | price
-----------+-------------+-----+-------
101        | Widget      | 100 | 9.99
102        | Gadget      | 100 | 24.99  (updated)
103        | Gizmo-v2    | 80  | 16.99  (updated)
104        | Doohickey   | 200 | 4.99   (inserted)
105        | Thingamajig | 150 | 7.99   (inserted)
```

## Query Type

The `RayUpsertQuery` type provides a programmatic interface:

```rust
use rayforce::{RayTable, RayUpsertQuery};

// Query builder pattern (conceptual)
let result = table
    .upsert()
    .key_columns(1)
    .values([1, "Alice", 75000])
    .execute()?;
```

## Use Cases

### Time-Series Data

```rust
// Update latest price, insert if new day
ray.eval(r#"
    (upsert 'daily_prices 2 (list
        ['AAPL]
        [today_date]
        [current_price]))
"#)?;
```

### Syncing Data

```rust
// Sync from external source
ray.eval(r#"
    (upsert 'local_data 1 external_data)
"#)?;
```

### Master Data

```rust
// Update master records
ray.eval(r#"
    (upsert 'customers 1 new_customer_data)
"#)?;
```

## Error Handling

Upsert fails if:
- Key count exceeds column count
- Type mismatch
- Invalid data structure

```rust
match ray.eval("(upsert table 5 data)") {
    Ok(_) => println!("Upsert successful"),
    Err(e) => eprintln!("Upsert failed: {}", e),
}
```

## Comparison with Insert/Update

| Operation | Existing Key | Missing Key |
|-----------|--------------|-------------|
| INSERT    | Error/Skip   | Insert      |
| UPDATE    | Update       | No-op       |
| UPSERT    | Update       | Insert      |

## Performance Considerations

1. **Key indexing**: Keyed tables are faster
2. **Batch upserts**: More efficient than row-by-row
3. **Key selectivity**: High-cardinality keys perform better

## Next Steps

- **[Insert Query](insert.md)** - Insert only
- **[Update Query](update.md)** - Update only
- **[Joins](joins.md)** - Combine tables

