# :material-clock-outline: Temporal

Rayforce has three temporal types, all measured against a **2000-01-01** epoch:

| Type | Unit | Atom constructor | Reader | Slice reader |
|------|------|------------------|--------|--------------|
| `Date` | days since 2000-01-01 | `Value::date_days(i32)` | `as_date_days` | `date_days_slice` |
| `Time` | milliseconds since midnight | `Value::time_millis(i32)` | `as_time_millis` | `time_millis_slice` |
| `Timestamp` | nanoseconds since 2000-01-01 UTC | `Value::timestamp_nanos(i64)` | `as_timestamp_nanos` | `timestamp_nanos_slice` |

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    let _rt = Runtime::new()?;
    ```

## Raw constructors and readers

The raw integer forms are the lowest-level interface. They store and read the
underlying offset directly.

```rust
assert_eq!(Value::date_days(9000).as_date_days()?, 9000);
assert_eq!(Value::time_millis(3_600_000).as_time_millis()?, 3_600_000);
assert_eq!(
    Value::timestamp_nanos(1_000_000_000).as_timestamp_nanos()?,
    1_000_000_000,
);
```

Note the widths: `Date` and `Time` carry an `i32` offset, while `Timestamp`
carries an `i64` (nanoseconds need the extra range).

## Chrono conversions

With the default `chrono` feature enabled the temporal types convert to and from
`chrono` calendar types, with the epoch shift handled for you.

```rust
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use rayforce::{ToValue, FromValue};

let d = NaiveDate::from_ymd_opt(2021, 6, 15).unwrap();
assert_eq!(d.to_value().extract::<NaiveDate>()?, d);

let t = NaiveTime::from_hms_milli_opt(13, 30, 45, 250).unwrap();
assert_eq!(t.to_value().extract::<NaiveTime>()?, t);

let ts = Utc.with_ymd_and_hms(2021, 6, 15, 13, 30, 45).unwrap();
assert_eq!(ts.to_value().extract::<chrono::DateTime<Utc>>()?, ts);
```

The mapping is `NaiveDate` ↔ `Date`, `NaiveTime` ↔ `Time`, and
`DateTime<Utc>` ↔ `Timestamp`.

## Slice readers

For vectors there are dedicated zero-copy slice readers returning the raw offset
buffers. They reject non-temporal vectors:

```rust
let v = Value::vec(&[1i64, 2, 3]);
assert!(v.date_days_slice().is_err());
assert!(v.timestamp_nanos_slice().is_err());
```

- `date_days_slice()` and `time_millis_slice()` return `&[i32]`.
- `timestamp_nanos_slice()` returns `&[i64]`.

!!! tip "Everything is relative to 2000-01-01"
    Unlike Unix time (epoch 1970), Rayforce dates and timestamps count from
    2000-01-01. The `chrono` conversions adjust automatically; the raw
    `*_days` / `*_nanos` constructors do not — pass an already-relative offset.
