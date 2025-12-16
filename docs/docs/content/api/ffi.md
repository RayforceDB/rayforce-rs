# FFI (Foreign Function Interface)

The FFI module provides low-level bindings to the RayforceDB C library.

!!! warning "Advanced Usage"
    Most users should use the high-level API. FFI is for advanced use cases requiring direct C interop.

## Overview

The FFI layer provides:

- Direct access to C functions
- Raw pointer management
- Type conversion utilities
- Runtime initialization

## Core Types

### RayObj

The fundamental object wrapper for all RayforceDB values:

```rust
use rayforce::ffi::RayObj;

// Create from Rust primitive
let obj = RayObj::from(42_i64);

// Access raw pointer
let ptr = obj.as_ptr();

// Get type code
let type_code = obj.type_of();

// Check for nil
if obj.is_nil() {
    println!("Object is nil");
}

// Get length (for containers)
let len = obj.len();
```

### obj_t

The raw C struct for RayforceDB objects:

```rust
use rayforce::ffi::obj_t;

// obj_t is the underlying C type
// RayObj wraps obj_t* safely
```

## Rayforce Runtime

### Initialization

```rust
use rayforce::Rayforce;

// Create runtime
let ray = Rayforce::new()?;

// Access version
println!("Version: {}", ray.version());

// Runtime is cleaned up when dropped
```

### Evaluation Functions

```rust
use rayforce::Rayforce;

let ray = Rayforce::new()?;

// Evaluate string expression
let result = ray.eval("(+ 1 2)")?;

// Evaluate with object context
let obj = RayObj::from(10_i64);
let result = ray.eval_obj("(+ x 5)", &obj)?;
```

## Type Conversions

### From Rust to RayObj

```rust
use rayforce::ffi::RayObj;

// Primitives
let i = RayObj::from(42_i64);
let f = RayObj::from(3.14_f64);
let b = RayObj::from(true);
let c = RayObj::from('A');

// Strings
let s = RayObj::from("hello");

// From chrono types
use chrono::{NaiveDate, NaiveTime};
let date = RayObj::from(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
let time = RayObj::from(NaiveTime::from_hms_opt(9, 30, 0).unwrap());

// From UUID
use uuid::Uuid;
let guid = RayObj::from(Uuid::new_v4());
```

### From RayObj to Rust

```rust
use rayforce::ffi::RayObj;

let obj = RayObj::from(42_i64);

// Into primitive
let value: i64 = obj.into();

// Via AsMut
let ptr: &mut i64 = obj.as_mut();
```

## Low-Level Functions

### Object Operations

```rust
use rayforce::ffi::*;

// Get object length
let len = get_obj_length(&obj);

// Clone object
let cloned = obj.clone();

// Check type
let type_code = obj.type_of();
```

### Symbol Operations

```rust
use rayforce::ffi::*;

// Create symbol
let sym = new_symbol("price");

// Quote object
let quoted = quote_obj(&obj);
```

### Container Operations

```rust
use rayforce::ffi::*;

// Get element at index
let elem = get_at_index(&container, 0);

// Set element
binary_set(&mut container, 1, &new_value);
```

## Type Codes

| Code | Type | Description |
|------|------|-------------|
| -1 | B8 | Boolean |
| -2 | GUID | UUID |
| -4 | U8 | Unsigned byte |
| -5 | I16 | 16-bit integer |
| -6 | I32 | 32-bit integer |
| -7 | I64 | 64-bit integer |
| -9 | F64 | 64-bit float |
| -10 | C8 | Character |
| -11 | Symbol | Symbol |
| -12 | Timestamp | Timestamp |
| -14 | Date | Date |
| -19 | Time | Time |
| 0 | List | General list |
| 1+ | Vector | Typed vectors |
| 98 | Dict | Dictionary |
| 99 | Table | Table |

## Memory Management

### Ownership

`RayObj` manages memory through RAII:

```rust
{
    let obj = RayObj::from(42_i64);
    // obj owns the memory
} // Memory freed here when obj drops
```

### Reference Counting

For shared objects:

```rust
use rayforce::ffi::rc_obj;

let obj = RayObj::from(42_i64);
let refcount = rc_obj(&obj);  // Increment refcount
```

### Raw Pointer Access

```rust
// Get raw pointer (careful!)
let ptr = obj.as_ptr();

// Create from raw pointer (takes ownership)
let obj = unsafe { RayObj::from_raw(ptr) };
```

## Error Handling

### Error Messages

```rust
use rayforce::ffi::get_error_message;

let error_msg = get_error_message();
if let Some(msg) = error_msg {
    eprintln!("Error: {}", msg);
}
```

### Result Types

All fallible operations return `Result`:

```rust
let result = ray.eval("(invalid")?;  // Returns Err on parse error
```

## Advanced Usage

### Loading Functions

```rust
use rayforce::ffi::loadfn_from_file;

// Load custom function from shared library
loadfn_from_file("mylib.so", "myfunc")?;
```

### Environment Access

```rust
use rayforce::ffi::*;

// Get function by name
let func = env_get_internal_function_by_name("sum");

// Get name by function pointer
let name = env_get_internal_name_by_function(func_ptr);
```

### Object Attributes

```rust
use rayforce::ffi::set_obj_attrs;

// Set object attributes
set_obj_attrs(&mut obj, attrs);
```

## Safety Considerations

The FFI module contains `unsafe` code. Follow these guidelines:

1. **Never use raw pointers after dropping RayObj**
2. **Don't mix ownership** - either Rust or C owns memory
3. **Validate type codes** before casting
4. **Use high-level API when possible**

```rust
// Safe: using high-level API
let vec: RayVector<i64> = RayVector::from_iter([1, 2, 3]);

// Unsafe: direct pointer manipulation
unsafe {
    let ptr = vec.ptr();
    // Must ensure ptr remains valid
}
```

## Bindgen Types

The FFI types are generated by `bindgen` from C headers:

```rust
// Generated types
pub type obj_t = /* ... */;
pub type guid_t = /* ... */;

// Generated functions
extern "C" {
    pub fn i64_(x: i64) -> *mut obj_t;
    pub fn f64_(x: f64) -> *mut obj_t;
    // ...
}
```

## Next Steps

- **[IPC](ipc.md)** - Network communication
- **[Types](types/scalars.md)** - High-level type system
- **[API Overview](overview.md)** - Full API reference

