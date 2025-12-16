# Container Types

Container types hold multiple values. RayforceDB provides both homogeneous (vectors) and heterogeneous (lists) containers.

## RayVector<T>

Homogeneous arrays where all elements have the same type. Vectors are the most efficient container for columnar operations.

### Creating Vectors

```rust
use rayforce::{RayVector, RaySymbol};

// From iterator
let prices: RayVector<i64> = RayVector::from_iter([100, 200, 300, 400]);
let ratios: RayVector<f64> = RayVector::from_iter([1.5, 2.0, 3.5]);

// From symbols
let symbols: RayVector<RaySymbol> = RayVector::from_iter([
    RaySymbol::new("AAPL"),
    RaySymbol::new("GOOGL"),
    RaySymbol::new("MSFT"),
]);
```

### Vector Operations

```rust
use rayforce::RayVector;

let v: RayVector<i64> = RayVector::from_iter([10, 20, 30]);

// Length
println!("Length: {}", v.len());  // → 3

// Check if empty
if v.is_empty() {
    println!("Vector is empty");
}

// Display
println!("{}", v);  // → [10 20 30]
```

### Supported Element Types

| Element Type | Description |
|--------------|-------------|
| `i64` | 64-bit integers |
| `f64` | 64-bit floats |
| `RaySymbol` | Interned symbols |

## RayList

Heterogeneous lists that can hold any RayforceDB type. Lists are more flexible but less efficient than vectors.

### Creating Lists

```rust
use rayforce::{RayList, RayObj};

// Empty list
let mut list = RayList::new();

// Add elements
list.push(RayObj::from(42_i64));
list.push(RayObj::from("hello"));
list.push(RayObj::from(3.14_f64));
```

### List Operations

```rust
use rayforce::{RayList, RayObj};

let mut list = RayList::new();
list.push(RayObj::from(1_i64));
list.push(RayObj::from(2_i64));
list.push(RayObj::from(3_i64));

// Length
println!("Length: {}", list.len());  // → 3

// Access by index
if let Some(item) = list.get(0) {
    println!("First: {}", item);
}

// Set by index
list.set(1, RayObj::from(100_i64));

// Iterate
for item in list.iter() {
    println!("{}", item);
}
```

### List from Iterator

```rust
use rayforce::{RayList, RayObj};

let list = RayList::from_iter([
    RayObj::from(1_i64),
    RayObj::from("two"),
    RayObj::from(3.0_f64),
]);
```

## RayString

Character strings, implemented as a vector of characters.

### Creating Strings

```rust
use rayforce::RayString;

// From &str
let s = RayString::from("Hello, World!");

// From String
let owned = String::from("Rust");
let s = RayString::from(owned.as_str());
```

### String Operations

```rust
use rayforce::RayString;

let s = RayString::from("Hello");

// Length (character count)
println!("Length: {}", s.len());  // → 5

// Convert to Rust String
let rust_string: String = s.to_string();

// Display
println!("{}", s);  // → "Hello"
```

## RayDict

Key-value dictionaries mapping symbols to values.

### Creating Dictionaries

```rust
use rayforce::{RayDict, RaySymbol, RayObj};

// From pairs
let dict = RayDict::from_pairs([
    (RaySymbol::new("name"), RayObj::from("Alice")),
    (RaySymbol::new("age"), RayObj::from(30_i64)),
    (RaySymbol::new("active"), RayObj::from(true)),
]);
```

### Dictionary Operations

```rust
use rayforce::{RayDict, RaySymbol, RayObj};

let dict = RayDict::from_pairs([
    (RaySymbol::new("a"), RayObj::from(1_i64)),
    (RaySymbol::new("b"), RayObj::from(2_i64)),
]);

// Length
println!("Size: {}", dict.len());  // → 2

// Check if empty
if dict.is_empty() {
    println!("Dict is empty");
}

// Get keys
let keys = dict.keys();

// Get values
let values = dict.values();

// Clone
let dict2 = dict.clone();
```

### Accessing Values

```rust
use rayforce::{RayDict, RaySymbol, RayObj};

let dict = RayDict::from_pairs([
    (RaySymbol::new("price"), RayObj::from(100_i64)),
]);

// Get by key (returns Option)
if let Some(value) = dict.get(&RaySymbol::new("price")) {
    println!("Price: {}", value);
}
```

## Type Reference Table

| Type | Description | Homogeneous | Mutable |
|------|-------------|-------------|---------|
| `RayVector<T>` | Typed array | Yes | No |
| `RayList` | Mixed list | No | Yes |
| `RayString` | Character string | Yes | No |
| `RayDict` | Key-value map | No | No |

## Performance Considerations

### Vectors vs Lists

**Use Vectors when:**
- All elements have the same type
- Performing columnar operations
- Memory efficiency is important
- Operating on large datasets

**Use Lists when:**
- Elements have different types
- Building heterogeneous records
- Flexibility is more important than performance

### Memory Layout

```
Vector<i64>:  [i64, i64, i64, i64]  ← Contiguous memory
List:         [ptr, ptr, ptr, ptr]  ← Pointers to objects
```

Vectors store values contiguously, enabling SIMD operations and better cache performance.

## Common Patterns

### Building Data Structures

```rust
use rayforce::{RayList, RayVector, RayObj, RaySymbol};

// Table-like structure as list of vectors
let columns = RayList::from_iter([
    RayObj::from(RayVector::<i64>::from_iter([1, 2, 3]).ptr()),
    RayObj::from(RayVector::<f64>::from_iter([1.1, 2.2, 3.3]).ptr()),
]);

// Record as dict
let record = RayDict::from_pairs([
    (RaySymbol::new("id"), RayObj::from(1_i64)),
    (RaySymbol::new("name"), RayObj::from("Alice")),
    (RaySymbol::new("score"), RayObj::from(95.5_f64)),
]);
```

### Nested Structures

```rust
use rayforce::{RayList, RayObj};

// List of lists
let matrix = RayList::from_iter([
    RayObj::from(RayList::from_iter([
        RayObj::from(1_i64),
        RayObj::from(2_i64),
    ]).ptr()),
    RayObj::from(RayList::from_iter([
        RayObj::from(3_i64),
        RayObj::from(4_i64),
    ]).ptr()),
]);
```

## Next Steps

- **[Table](table.md)** - Working with tables
- **[Scalars](scalars.md)** - Scalar type reference
- **[Queries](../queries/select.md)** - Query operations

