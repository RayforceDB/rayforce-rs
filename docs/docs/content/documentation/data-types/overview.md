# :octicons-database-16: Data Types

Rayforce exposes RayforceDB's value model to Rust through a single, uniform
handle: every piece of data — an integer atom, a float vector, a dictionary, a
whole table — is a [`Value`](values.md). A `Value` is a thin, reference-counted
handle onto memory owned by the engine, so cloning one is cheap and never copies
the payload.

!!! note "Assume a live runtime"
    A single `Runtime` must be alive for the whole process before you touch any
    `Value`. Every example below assumes you have already written:

    ```rust
    use rayforce::{Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## The value model

Values fall into a few shapes:

- **Atoms** — a single scalar: `Value::i64(42)`, `Value::f64(3.5)`,
  `Value::sym("AAPL")`. Atoms carry a *negative* type code.
- **Vectors** — a homogeneous, contiguous buffer of one element type:
  `Value::vec(&[1i64, 2, 3])`. Vectors carry a *positive* type code (the
  positive of the matching atom code).
- **List** — a heterogeneous, ordered collection of `Value`s (type code `0`).
- **Dict** — a keys-vector → values mapping (type code `99`).
- **Table** — a columnar dict of named vectors (type code `98`). See
  [Table](../table/overview.md).
- **Function** — a user-defined lambda (type code `100`). See
  [Functions](functions.md).

The atom code and its vector code are mirror images: an `I64` atom is `-5`, an
`I64` vector is `5`. Inspect them with `.type_code()` (signed) or `.abs_type()`
(absolute), and classify with `.is_atom()`, `.is_vec()`, `.is_list()`,
`.is_dict()`, `.is_table()`.

## Type summary

| Type | Abs code | Atom constructor | Vector constructor | Reader |
|------|---------:|------------------|--------------------|--------|
| [B8 (bool)](boolean.md) | 1 | `Value::bool(b)` | `Value::bool_vec(&[..])` | `as_bool` / `bool_slice` |
| [U8](integers.md) | 2 | `Value::u8(x)` | `Value::vec(&[..u8])` | `as_u8` / `as_slice::<u8>` |
| [I16](integers.md) | 3 | `Value::i16(x)` | `Value::vec(&[..i16])` | `as_i16` / `as_slice::<i16>` |
| [I32](integers.md) | 4 | `Value::i32(x)` | `Value::vec(&[..i32])` | `as_i32` / `as_slice::<i32>` |
| [I64](integers.md) | 5 | `Value::i64(x)` | `Value::vec(&[..i64])` | `as_i64` / `as_slice::<i64>` |
| [F32](float.md) | 6 | `Value::f32(x)` | `Value::vec(&[..f32])` | `as_f32` / `as_slice::<f32>` |
| [F64](float.md) | 7 | `Value::f64(x)` | `Value::vec(&[..f64])` | `as_f64` / `as_slice::<f64>` |
| [Date](temporal.md) | 8 | `Value::date_days(i32)` | — | `as_date_days` / `date_days_slice` |
| [Time](temporal.md) | 9 | `Value::time_millis(i32)` | — | `as_time_millis` / `time_millis_slice` |
| [Timestamp](temporal.md) | 10 | `Value::timestamp_nanos(i64)` | — | `as_timestamp_nanos` / `timestamp_nanos_slice` |
| [GUID](guid.md) | 11 | `Value::guid(&[u8;16])` | — | `as_guid` |
| [Symbol](symbol.md) | 12 | `Value::sym(&str)` | `Value::sym_vec(&[..])` | `as_sym` |
| [String](string.md) | 13 | `Value::string(&str)` | `Value::str_vec(&[..])` | `as_string` |
| [List](list.md) | 0 | `Value::list(&[..])` | — | `get` |
| [Dict](dict.md) | 99 | `Value::dict(keys, values)` | — | `dict_get` |
| [Table](../table/overview.md) | 98 | `Table::new(..)` | — | `column(..)` |
| [Function](functions.md) | 100 | `Fn::new("(fn …)")` | — | `call` / `apply` |

!!! tip "Integer literals default to `i32`"
    A bare literal like `42` is `i32` in Rust. Annotate the element type when you
    build non-`i32` vectors: `Value::vec(&[1i64, 2, 3])`, not `Value::vec(&[1, 2, 3])`
    if you want `I64`.

## Null handling

Each type has a designated null. For the signed integers and floats the null is
the type's sentinel: `i16::MIN`, `i32::MIN`, `i64::MIN`, and `NaN` for floats.

```rust
assert!(Value::i64(i64::MIN).is_atom_null());
assert!(Value::f64(f64::NAN).is_atom_null());
```

In Rust these map cleanly onto `Option<T>`: a `None` becomes a typed null and a
null `Value` extracts back to `None`.

```rust
let none: Option<i64> = None;
assert!(none.to_value().is_null());
assert_eq!(Value::i64(i64::MIN).extract::<Option<i64>>()?, None);
```

Vectors track nulls in a separate bitmap rather than by inspecting payload bytes.
A buffer that *happens* to hold a sentinel value is **not** null until it is
explicitly marked — see [Vectors](vector.md#null-bitmap) for the details.

Continue with [Values & Conversions](values.md) for how `Value` interoperates
with native Rust types.
