# Join Operations

Joins combine data from multiple tables based on matching columns.

## Join Types

| Type | Description |
|------|-------------|
| `left-join` | All rows from left, matching from right |
| `inner-join` | Only matching rows from both |
| `asof-join` | Time-series join (as-of lookup) |
| `window-join` | Window-based time-series join |

## Left Join

Returns all rows from the left table with matching data from the right table.

```rust
let ray = Rayforce::new()?;

// Create tables
ray.eval(r#"
    (set orders (table [order_id customer_id product qty]
        (list
            [1001 1002 1003 1004]
            [1 2 1 3]
            ['Widget 'Gadget 'Gizmo 'Widget]
            [10 5 3 20])))
"#)?;

ray.eval(r#"
    (set customers (table [customer_id name region]
        (list
            [1 2]
            (list "Alice" "Bob")
            ['East 'West])))
"#)?;

// Left join
let result = ray.eval(r#"
    (left-join [customer_id] orders customers)
"#)?;
```

Result includes all orders, with customer info for matching IDs:
```
order_id | customer_id | product | qty | name  | region
---------+-------------+---------+-----+-------+--------
1001     | 1           | Widget  | 10  | Alice | East
1002     | 2           | Gadget  | 5   | Bob   | West
1003     | 1           | Gizmo   | 3   | Alice | East
1004     | 3           | Widget  | 20  | nil   | nil
```

## Inner Join

Returns only rows that match in both tables.

```rust
let result = ray.eval(r#"
    (inner-join [customer_id] orders customers)
"#)?;
```

Result excludes order 1004 (no matching customer):
```
order_id | customer_id | product | qty | name  | region
---------+-------------+---------+-----+-------+--------
1001     | 1           | Widget  | 10  | Alice | East
1002     | 2           | Gadget  | 5   | Bob   | West
1003     | 1           | Gizmo   | 3   | Alice | East
```

## Multi-Column Join

Join on multiple columns:

```rust
ray.eval(r#"
    (set trades (table [sym date price qty]
        (list
            ['AAPL 'AAPL 'MSFT]
            [2024.01.01 2024.01.02 2024.01.01]
            [150.0 151.0 300.0]
            [100 200 50])))
"#)?;

ray.eval(r#"
    (set quotes (table [sym date bid ask]
        (list
            ['AAPL 'AAPL 'MSFT]
            [2024.01.01 2024.01.02 2024.01.01]
            [149.5 150.5 299.0]
            [150.5 151.5 301.0])))
"#)?;

// Join on both sym and date
let result = ray.eval(r#"
    (left-join [sym date] trades quotes)
"#)?;
```

## As-Of Join

Joins time-series data, matching to the most recent record.

```rust
ray.eval(r#"
    (set trades (table [sym time price]
        (list
            ['AAPL 'AAPL 'AAPL]
            [09:30:01 09:31:05 09:32:10]
            [150.0 150.5 151.0])))
"#)?;

ray.eval(r#"
    (set quotes (table [sym time bid ask]
        (list
            ['AAPL 'AAPL 'AAPL 'AAPL]
            [09:30:00 09:30:30 09:31:00 09:32:00]
            [149.8 149.9 150.2 150.8]
            [150.2 150.3 150.6 151.2])))
"#)?;

// As-of join: match each trade to most recent quote
let result = ray.eval(r#"
    (asof-join [sym time] trades quotes)
"#)?;
```

Trade at 09:30:01 matches quote from 09:30:00.

## Window Join

Joins within a time window.

```rust
let result = ray.eval(r#"
    (window-join [sym time] trades quotes -00:01:00 00:00:00)
"#)?;
```

Parameters:
- `[sym time]` - Join columns
- `trades` - Left table
- `quotes` - Right table  
- `-00:01:00` - Window start (1 minute before)
- `00:00:00` - Window end (exact time)

## Complete Example

```rust
use rayforce::Rayforce;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ray = Rayforce::new()?;
    
    // Create trades table
    ray.eval(r#"
        (set trades (table [trade_id sym time price qty]
            (list
                [1 2 3 4 5]
                ['AAPL 'MSFT 'AAPL 'GOOGL 'MSFT]
                [09:30:01 09:30:05 09:31:00 09:31:30 09:32:00]
                [150.0 300.0 150.5 2800.0 301.0]
                [100 50 200 25 75])))
    "#)?;
    
    // Create reference data
    ray.eval(r#"
        (set securities (table [sym name sector exchange]
            (list
                ['AAPL 'MSFT 'GOOGL]
                (list "Apple Inc" "Microsoft" "Alphabet")
                ['Tech 'Tech 'Tech]
                ['NASDAQ 'NASDAQ 'NASDAQ])))
    "#)?;
    
    // Join to get full trade details
    let enriched = ray.eval(r#"
        (left-join [sym] trades securities)
    "#)?;
    
    println!("Enriched trades:");
    println!("{}\n", enriched);
    
    // Aggregate by security name
    let by_name = ray.eval(r#"
        (select {
            name: name
            trade_count: (count trade_id)
            total_qty: (sum qty)
            total_value: (sum (* price qty))
            from: (left-join [sym] trades securities)
            by: name})
    "#)?;
    
    println!("Summary by security:");
    println!("{}", by_name);
    
    Ok(())
}
```

## Performance Tips

### Index Optimization

Create keyed tables for join columns:

```rust
// Key the lookup table
ray.eval("(set customers (key customers 1))")?;

// Join will be faster
ray.eval("(left-join [customer_id] orders customers)")?;
```

### Column Order

Put join columns first for efficiency:

```rust
// Good: join column first
ray.eval(r#"
    (table [customer_id name region] data)
"#)?;
```

### Join Direction

For `left-join`, put the larger table on the left:

```rust
// Efficient: large table left, small table right
ray.eval("(left-join [key] large_table small_lookup)")?;
```

## Error Handling

Joins fail if:
- Join columns don't exist
- Type mismatch in join columns

```rust
match ray.eval("(left-join [missing_col] t1 t2)") {
    Ok(result) => println!("{}", result),
    Err(e) => eprintln!("Join failed: {}", e),
}
```

## Join Comparison

| Join | Matching | Non-Matching |
|------|----------|--------------|
| left-join | Include | Left only (nil right) |
| inner-join | Include | Exclude |
| asof-join | Include (latest) | Left only |
| window-join | Include (in window) | Left only |

## Next Steps

- **[Select Query](select.md)** - Query data
- **[Update Query](update.md)** - Modify data
- **[Table](../types/table.md)** - Table operations

