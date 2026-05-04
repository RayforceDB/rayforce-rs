# Scalar Types

Scalar types represent single atom values. Every wrapper here carries
a **negative** type tag in rayforce 2.0 (e.g. `-RAY_I64`, `-RAY_F64`)
and stores its value inline in the 32-byte object header.

All wrappers implement the [`RayType`](../overview.md#raytype) trait.
The constructors all return owned values — drop them when you're done
or hand them off to a container that takes ownership.

## Integer types

### RayI64 (64-bit integer)

The most common integer type for general-purpose use.

```rust
use rayforce::RayI64;

let x = RayI64::new(42);
let y = RayI64::new(-100);

let value: i64 = x.value();
println!("{}", x);  // → 42
```

### RayI32 (32-bit integer)

```rust
use rayforce::RayI32;
let x = RayI32::new(1000);
let value: i32 = x.value();
```

### RayI16 (16-bit integer)

```rust
use rayforce::RayI16;
let x = RayI16::new(100);
let value: i16 = x.value();
```

## Floating point

### RayF64 (64-bit float)

```rust
use rayforce::RayF64;

let pi = RayF64::new(3.14159);
let value: f64 = pi.value();
println!("{}", pi);  // → 3.14159
```

## Byte / boolean

### RayU8 (unsigned byte)

```rust
use rayforce::RayU8;
let byte = RayU8::new(255);
let value: u8 = byte.value();
```

### RayBool (boolean)

```rust
use rayforce::RayBool;          // also re-exported as `B8`
let flag = RayBool::new(true);
let is_true: bool = flag.value();
println!("{}", flag);            // → true
```

!!! note "What about `RayChar` / `C8`?"
    Removed in 2.0 — there's no single-character atom type. Use
    `RayU8::new(b'a')` (a byte) or `RayString::new("a")` (a one-byte
    string atom) instead.

## Symbol type

### RaySymbol

Interned strings. Construction goes through `ray_sym_intern` (returns
an `i64` ID), then `ray_sym(id)` wraps the ID as a `-RAY_SYM` atom:

```rust
use rayforce::RaySymbol;

let name = RaySymbol::new("price");
let dept = RaySymbol::new("IT");

// Same name → same interned ID
let s1 = RaySymbol::new("test");
let s2 = RaySymbol::new("test");
assert_eq!(s1.id(), s2.id());

println!("{}", name);  // → `price
```

`RaySymbol::id()` exposes the underlying `i64` ID — useful when
calling raw FFI that takes a sym ID (e.g. `ray_env_set`,
`ray_table_add_col`).

## String type

### RayString

A `RAY_STR` atom: small-string-optimised under 7 bytes, pool-backed
otherwise. New in 2.0 (in 1.0 strings were `TYPE_C8` char vectors).

```rust
use rayforce::RayString;

let s = RayString::new("hello");
println!("{}", s);            // → hello
println!("len = {}", s.len());

let long = RayString::new(&"x".repeat(1000));
assert_eq!(long.len(), 1000);
```

## Temporal types

All three temporal atoms store an `i64` value internally. The vector
element widths differ (`RAY_DATE` and `RAY_TIME` are 4-byte columns,
`RAY_TIMESTAMP` is an 8-byte column), but atoms always use the i64
union arm.

### RayDate (days since 2000-01-01)

```rust
use rayforce::RayDate;
use chrono::NaiveDate;

let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
let ray_date = RayDate::from_naive_date(date);

let back: NaiveDate = ray_date.to_naive_date();
println!("{}", ray_date);     // → 2024-01-15
println!("days since epoch: {}", ray_date.days());
```

### RayTime (milliseconds since midnight)

```rust
use rayforce::RayTime;
use chrono::NaiveTime;

let time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
let ray_time = RayTime::from_naive_time(time);

let back: NaiveTime = ray_time.to_naive_time();
println!("{}", ray_time);     // → 09:30:00
println!("ms since midnight: {}", ray_time.ms());
```

### RayTimestamp (nanoseconds since epoch)

```rust
use rayforce::RayTimestamp;
use chrono::NaiveDateTime;

let dt = NaiveDateTime::parse_from_str(
    "2024-01-15 09:30:00", "%Y-%m-%d %H:%M:%S"
).unwrap();

let ts = RayTimestamp::from_naive_datetime(dt);
let back: NaiveDateTime = ts.to_naive_datetime();
println!("ns: {}", ts.nanos());
```

## GUID type

### RayGuid

16-byte GUID. The atom stores a pointer to a U8 vector of length 16 in
its `obj` union field.

```rust
use rayforce::RayGuid;
use uuid::Uuid;

let g = RayGuid::random();                          // new V4 UUID
let g = RayGuid::parse("550e8400-e29b-41d4-a716-446655440000")?;
let g = RayGuid::new(Uuid::new_v4());

let uuid: Uuid = g.to_uuid();
println!("{}", g);
```

## Type reference table

Atom type tags are negative; the magnitude matches the corresponding
`RAY_*` vector tag. Numeric values come straight from `rayforce.h`.

| Wrapper | Atom tag | Vector tag | Element size | Rust value |
|---------|----------|------------|--------------|------------|
| `RayBool` | -1 | `RAY_BOOL` (1) | 1 byte | `bool` |
| `RayU8` | -2 | `RAY_U8` (2) | 1 byte | `u8` |
| `RayI16` | -3 | `RAY_I16` (3) | 2 bytes | `i16` |
| `RayI32` | -4 | `RAY_I32` (4) | 4 bytes | `i32` |
| `RayI64` | -5 | `RAY_I64` (5) | 8 bytes | `i64` |
| `RayF64` | -7 | `RAY_F64` (7) | 8 bytes | `f64` |
| `RayDate` | -8 | `RAY_DATE` (8) | 4 bytes | `chrono::NaiveDate` |
| `RayTime` | -9 | `RAY_TIME` (9) | 4 bytes | `chrono::NaiveTime` |
| `RayTimestamp` | -10 | `RAY_TIMESTAMP` (10) | 8 bytes | `chrono::NaiveDateTime` |
| `RayGuid` | -11 | `RAY_GUID` (11) | 16 bytes | `uuid::Uuid` |
| `RaySymbol` | -12 | `RAY_SYM` (12) | adaptive width | — |
| `RayString` | -13 | `RAY_STR` (13) | variable (SSO + pool) | `String` |

## Common patterns

### Going through `RayObj`

`RayObj::from(T)` works for any of the primitives that have a
corresponding `From` impl:

```rust
use rayforce::RayObj;

let obj = RayObj::from(42i64);
let obj = RayObj::from(3.14f64);
let obj = RayObj::from(true);
let obj = RayObj::from("hello");          // → -RAY_STR atom

// Check type via the magnitude of the type tag
if obj.type_code() == -(rayforce::RAY_I64 as i8) {
    let value: i64 = obj.try_into().unwrap();
    println!("{}", value);
}
```

### Display and Debug

All scalar types implement `Display` and `Debug`:

```rust
use rayforce::{RayI64, RaySymbol};

let x = RayI64::new(42);
let s = RaySymbol::new("test");

println!("{}", x);   // → 42
println!("{}", s);   // → `test
println!("{:?}", x); // → RayI64(42)
println!("{:?}", s); // → RaySymbol(`test)
```

## Next steps

- **[Containers](containers.md)** — `RayVector` / `RayList` / `RayDict` / `RayString`.
- **[Table](table.md)** — `RayTable` reference.
- **[FFI](../ffi.md)** — low-level `RayObj` accessor helpers.
