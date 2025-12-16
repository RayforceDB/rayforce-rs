# rayforce-rs

Rust bindings for [RayforceDB](https://github.com/RayforceDB/rayforce) - a high-performance time-series database.

## Features

- **Complete Type System**: Full support for all Rayforce types including scalars (I64, F64, Symbol, Date, Time, Timestamp, GUID), containers (List, Vector, Dict), and Table.
- **Query Builder**: Fluent API for building SELECT, UPDATE, INSERT, and UPSERT queries.
- **Expression Builder**: Type-safe expression construction for WHERE clauses and computed columns.
- **IPC Support**: Connect to remote Rayforce servers.
- **Automatic Build**: Automatically clones and builds the Rayforce C library from source.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rayforce-rs = "0.1"
```

### Build Requirements

- **Clang/LLVM**: Required for bindgen
- **Git**: For cloning Rayforce sources
- **Make**: For building the C library
- **C compiler** (gcc or clang)

On Ubuntu/Debian:
```bash
sudo apt install llvm-dev libclang-dev clang git build-essential
```

On macOS:
```bash
xcode-select --install
brew install llvm
```

## Quick Start

```rust
use rayforce::{Rayforce, Table, Column, Vector, Symbol, I64, Result};

fn main() -> Result<()> {
    // Initialize the runtime
    let rf = Rayforce::new()?;
    
    // Create a table
    let table = Table::from_dict([
        ("id", Vector::<i64>::from_iter([1i64, 2, 3]).as_ray_obj().clone()),
        ("name", Vector::<Symbol>::from_iter(["Alice", "Bob", "Charlie"]).ptr().clone()),
        ("score", Vector::<f64>::from_iter([95.5, 87.3, 92.1]).as_ray_obj().clone()),
    ])?;
    
    println!("Table:\n{}", table);
    
    // Query with WHERE clause
    let result = table
        .select()
        .columns(&["id", "name"])
        .where_cond(Column::new("score").gt(90.0f64))
        .execute()?;
    
    println!("High scorers:\n{}", result);
    
    // Evaluate Rayforce expressions
    let sum = rf.eval("sum(1 2 3 4 5)")?;
    println!("Sum: {}", sum);
    
    Ok(())
}
```

## Type System

### Scalar Types

```rust
use rayforce::{I64, F64, B8, Symbol, Date, Time, Timestamp, GUID};
use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
use uuid::Uuid;

let i = I64::new(42);
let f = F64::new(3.14);
let b = B8::new(true);
let sym = Symbol::new("hello");

// Temporal types
let date = Date::from_naive_date(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
let time = Time::from_ms(43200000); // 12:00:00
let ts = Timestamp::from_nanos(1705312800000000000);

// GUID
let guid = GUID::random();
let guid2 = GUID::parse("550e8400-e29b-41d4-a716-446655440000")?;
```

### Container Types

```rust
use rayforce::{List, Vector, Dict, Symbol};

// Vectors (homogeneous)
let ints: Vector<i64> = Vector::from_iter([1i64, 2, 3]);
let floats: Vector<f64> = Vector::from_iter([1.1, 2.2, 3.3]);
let symbols = Vector::<Symbol>::from_iter(["a", "b", "c"]);

// Lists (heterogeneous)
let mut list = List::new();
list.push(I64::new(42).ptr().clone());
list.push(RayString::new("hello").ptr().clone());

// Dictionaries
let dict = Dict::from_pairs([
    ("name", RayString::new("Alice").ptr().clone()),
    ("age", I64::new(30).ptr().clone()),
])?;
```

### Tables

```rust
use rayforce::{Table, Column, Vector, Symbol};

// Create from dictionary
let table = Table::from_dict([
    ("col1", Vector::<i64>::from_iter([1, 2, 3]).as_ray_obj().clone()),
    ("col2", Vector::<Symbol>::from_iter(["a", "b", "c"]).ptr().clone()),
])?;

// Reference a named table
let table_ref = Table::from_name("my_table");

// Access columns
let cols = table.columns()?;
let row_count = table.len()?;
```

## Query Builder

### SELECT

```rust
let result = table
    .select()
    .columns(&["id", "name", "score"])
    .where_cond(Column::new("score").gt(80.0))
    .group_by(&["department"])
    .execute()?;
```

### UPDATE

```rust
let updated = table
    .update()
    .set("score", Column::new("score").sum())
    .where_cond(Column::new("active").eq(true))
    .execute()?;
```

### INSERT

```rust
let inserted = table
    .insert()
    .values([
        ("id", I64::new(4).ptr().clone()),
        ("name", Symbol::new("Dave").ptr().clone()),
    ])
    .execute()?;
```

### UPSERT

```rust
let upserted = table
    .upsert(1)  // match by first 1 column(s)
    .values([
        ("id", I64::new(1).ptr().clone()),
        ("score", F64::new(99.9).ptr().clone()),
    ])
    .execute()?;
```

## Joins

```rust
// Inner join
let result = table1.inner_join(&table2, &["key_column"])?;

// Left join
let result = table1.left_join(&table2, &["key_column"])?;
```

## IPC (Remote Connection)

```rust
use rayforce::ipc::hopen;

let conn = hopen("localhost", 5000)?;
let result = conn.execute("select * from trades")?;
conn.close()?;
```

## Expression Builder

```rust
use rayforce::Column;

let col = Column::new("price");

// Comparisons
let expr1 = col.gt(100.0);      // price > 100
let expr2 = col.le(200.0);      // price <= 200
let expr3 = col.eq(150.0);      // price == 150

// Combine with AND/OR
let combined = col.gt(100.0).and(col.lt(200.0));

// Aggregations
let sum_expr = col.sum();
let avg_expr = col.avg();
let count_expr = col.count();
```

## Environment Variables

- `RAYFORCE_GITHUB`: Override the Rayforce repository URL (default: `https://github.com/RayforceDB/rayforce.git`)

## License

MIT License - see [LICENSE](LICENSE) for details.

## See Also

- [RayforceDB](https://github.com/RayforceDB/rayforce) - The Rayforce database
- [rayforce-py](https://github.com/RayforceDB/rayforce-py) - Official Python bindings
