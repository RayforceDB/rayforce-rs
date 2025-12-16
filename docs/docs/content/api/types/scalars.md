# Scalar Types

Scalar types represent single values in RayforceDB. All scalar types are prefixed with `Ray` for namespace clarity.

## Integer Types

### RayI64 (64-bit Integer)

The most common integer type for general-purpose use.

```rust
use rayforce::RayI64;

// Create from value
let x = RayI64::from_value(42);
let y = RayI64::from_value(-100);

// Access value
let value: i64 = x.to_python();

// Display
println!("{}", x);  // → 42
```

### RayI32 (32-bit Integer)

For smaller integer values with reduced memory footprint.

```rust
use rayforce::RayI32;

let x = RayI32::from_value(1000);
let value: i32 = x.to_python();
```

### RayI16 (16-bit Integer)

Compact integer type for memory-efficient storage.

```rust
use rayforce::RayI16;

let x = RayI16::from_value(100);
let value: i16 = x.to_python();
```

## Floating Point

### RayF64 (64-bit Float)

Double-precision floating point numbers.

```rust
use rayforce::RayF64;

let pi = RayF64::from_value(3.14159);
let e = RayF64::from_value(2.71828);

let value: f64 = pi.to_python();
println!("{}", pi);  // → 3.14159
```

## Byte Types

### RayU8 (Unsigned Byte)

8-bit unsigned integer, useful for raw byte data.

```rust
use rayforce::RayU8;

let byte = RayU8::from_value(255);
let value: u8 = byte.to_python();
```

### RayB8 (Boolean)

Boolean true/false values.

```rust
use rayforce::RayB8;

let flag = RayB8::from_value(true);
let is_true: bool = flag.to_python();

println!("{}", flag);  // → 1b (RayforceDB boolean format)
```

### RayC8 (Character)

Single character values.

```rust
use rayforce::RayC8;

let ch = RayC8::from_value('A');
let value: char = ch.to_python();
```

## Symbol Type

### RaySymbol

Interned strings for efficient storage and comparison. Symbols are commonly used for column names and categorical data.

```rust
use rayforce::RaySymbol;

// Create symbols
let name = RaySymbol::new("price");
let dept = RaySymbol::new("IT");

// Symbols with same content are identical
let s1 = RaySymbol::new("test");
let s2 = RaySymbol::new("test");
// s1 and s2 reference the same interned string

println!("{}", name);  // → `price
```

### QuotedSymbol

For symbols that need to be quoted in expressions.

```rust
use rayforce::QuotedSymbol;

let quoted = QuotedSymbol::new("myvar");
// Used in expression contexts where quoting is needed
```

## Temporal Types

### RayDate

Date values without time component.

```rust
use rayforce::RayDate;
use chrono::NaiveDate;

// Create from chrono date
let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
let ray_date = RayDate::from_value(date);

// Convert back
let value: NaiveDate = ray_date.to_python();
println!("{}", ray_date);  // → 2024.01.15
```

### RayTime

Time-of-day values.

```rust
use rayforce::RayTime;
use chrono::NaiveTime;

let time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
let ray_time = RayTime::from_value(time);

let value: NaiveTime = ray_time.to_python();
println!("{}", ray_time);  // → 09:30:00.000
```

### RayTimestamp

Combined date and time.

```rust
use rayforce::RayTimestamp;
use chrono::NaiveDateTime;

let dt = NaiveDateTime::parse_from_str(
    "2024-01-15 09:30:00",
    "%Y-%m-%d %H:%M:%S"
).unwrap();

let ts = RayTimestamp::from_value(dt);
let value: NaiveDateTime = ts.to_python();
```

## GUID Type

### RayGuid

Universally unique identifiers.

```rust
use rayforce::RayGuid;
use uuid::Uuid;

// Create from UUID
let uuid = Uuid::new_v4();
let guid = RayGuid::from_value(uuid);

// Convert back
let value: Uuid = guid.to_python();
```

## Type Reference Table

| Type | Code | Size | Rust Type | Format |
|------|------|------|-----------|--------|
| `RayI16` | -5 | 2 bytes | `i16` | `100h` |
| `RayI32` | -6 | 4 bytes | `i32` | `100i` |
| `RayI64` | -7 | 8 bytes | `i64` | `100` |
| `RayF64` | -9 | 8 bytes | `f64` | `3.14` |
| `RayU8` | -4 | 1 byte | `u8` | `0x42` |
| `RayB8` | -1 | 1 byte | `bool` | `1b` / `0b` |
| `RayC8` | -10 | 1 byte | `char` | `"a"` |
| `RaySymbol` | -11 | ptr | - | `` `sym`` |
| `RayDate` | -14 | 4 bytes | `NaiveDate` | `2024.01.15` |
| `RayTime` | -19 | 8 bytes | `NaiveTime` | `09:30:00` |
| `RayTimestamp` | -12 | 8 bytes | `NaiveDateTime` | timestamp |
| `RayGuid` | -2 | 16 bytes | `Uuid` | GUID |

## Common Patterns

### Type Conversion with RayObj

```rust
use rayforce::RayObj;

// From primitives
let obj = RayObj::from(42_i64);
let obj = RayObj::from(3.14_f64);
let obj = RayObj::from(true);

// Check type
if obj.type_of() == -7 {  // TYPE_I64
    let value: i64 = obj.into();
}

// Check for nil
if obj.is_nil() {
    println!("Object is nil");
}
```

### Display and Debug

All scalar types implement `Display` and `Debug`:

```rust
use rayforce::{RayI64, RaySymbol};

let x = RayI64::from_value(42);
let s = RaySymbol::new("test");

// Display (user-friendly)
println!("{}", x);   // → 42
println!("{}", s);   // → `test

// Debug (with type info)
println!("{:?}", x); // → RayI64(42)
println!("{:?}", s); // → RaySymbol("test")
```

## Next Steps

- **[Containers](containers.md)** - Vector, list, dict types
- **[Table](table.md)** - Working with tables
- **[Queries](../queries/select.md)** - Query operations

