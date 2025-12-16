# API Overview

This section provides comprehensive documentation for the rayforce-rs API.

## Module Structure

```
rayforce
├── ffi           # Low-level FFI bindings
├── types         # Type system
│   ├── scalars   # Scalar types (RayI64, RayF64, etc.)
│   ├── containers# Container types (RayVector, RayList, etc.)
│   └── table     # Table and query types
├── ipc           # IPC/networking
└── error         # Error types
```

## Core Types

### Runtime

| Type | Description |
|------|-------------|
| `Rayforce` | Main runtime handle for RayforceDB |
| `RayObj` | Generic object wrapper for any RayforceDB value |

### Scalar Types

| Type | Description | Rust Equivalent |
|------|-------------|-----------------|
| `RayI16` | 16-bit signed integer | `i16` |
| `RayI32` | 32-bit signed integer | `i32` |
| `RayI64` | 64-bit signed integer | `i64` |
| `RayF64` | 64-bit floating point | `f64` |
| `RayU8` | Unsigned byte | `u8` |
| `RayB8` | Boolean | `bool` |
| `RayC8` | Character | `char` |
| `RaySymbol` | Interned string symbol | - |
| `RayDate` | Date value | `chrono::NaiveDate` |
| `RayTime` | Time value | `chrono::NaiveTime` |
| `RayTimestamp` | Timestamp | `chrono::NaiveDateTime` |
| `RayGuid` | UUID/GUID | `uuid::Uuid` |

### Container Types

| Type | Description |
|------|-------------|
| `RayVector<T>` | Homogeneous array of values |
| `RayList` | Heterogeneous list |
| `RayString` | Character string |
| `RayDict` | Key-value dictionary |
| `RayTable` | Columnar table |

### Query Types

| Type | Description |
|------|-------------|
| `RaySelectQuery` | SELECT query builder |
| `RayUpdateQuery` | UPDATE query builder |
| `RayInsertQuery` | INSERT query builder |
| `RayUpsertQuery` | UPSERT query builder |
| `RayColumn` | Table column reference |
| `RayExpression` | Query expression |

## Key Traits

### RayType

All RayforceDB types implement the `RayType` trait:

```rust
pub trait RayType {
    const TYPE_CODE: i8;
    const RAY_NAME: &'static str;
    
    fn ptr(&self) -> *mut obj_t;
    fn from_ptr(ptr: *mut obj_t) -> Self;
}
```

### From/Into Conversions

RayforceDB types support conversions from Rust primitives:

```rust
// From Rust primitives
let obj = RayObj::from(42_i64);
let obj = RayObj::from(3.14_f64);
let obj = RayObj::from("hello");

// Into Rust primitives
let value: i64 = obj.into();
```

## Error Handling

All fallible operations return `Result<T, RayError>`:

```rust
use rayforce::{Rayforce, RayError};

fn main() -> Result<(), RayError> {
    let ray = Rayforce::new()?;
    let result = ray.eval("(+ 1 2)")?;
    Ok(())
}
```

### Error Types

| Type | Description |
|------|-------------|
| `RayforceError` | Runtime initialization errors |
| `RayObjError` | Object creation/manipulation errors |
| `ConversionError` | Type conversion failures |
| `TypeError` | Type mismatch errors |
| `IndexError` | Out-of-bounds access |
| `KeyError` | Missing dictionary key |
| `RuntimeError` | General runtime errors |

## Quick Reference

### Creating Values

```rust
use rayforce::*;

// Scalars
let i = RayI64::from_value(42);
let f = RayF64::from_value(3.14);
let s = RaySymbol::new("name");

// Vectors
let v: RayVector<i64> = RayVector::from_iter([1, 2, 3]);

// Lists
let mut l = RayList::new();
l.push(RayObj::from(42_i64));

// Dictionaries
let d = RayDict::from_pairs([
    (RaySymbol::new("key"), RayObj::from("value")),
]);
```

### Evaluating Expressions

```rust
let ray = Rayforce::new()?;

// Evaluate string expressions
let result = ray.eval("(+ 1 2 3)")?;

// Evaluate with objects
let obj = RayObj::from(42_i64);
let result = ray.eval_obj("(+ x 10)", &obj)?;
```

### Querying Tables

```rust
// Create table
let table = ray.eval("(table [a b] (list [1 2] [3 4]))")?;

// Select query
let result = ray.eval(r#"
    (select {
        a: a
        from: table
        where: (> b 2)})
"#)?;
```

## Next Steps

- **[Scalars](types/scalars.md)** - Detailed scalar type documentation
- **[Containers](types/containers.md)** - Container types guide
- **[Tables](types/table.md)** - Working with tables
- **[Queries](queries/select.md)** - Query operations
- **[FFI](ffi.md)** - Low-level FFI details
- **[IPC](ipc.md)** - Inter-process communication

