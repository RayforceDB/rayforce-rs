# :material-clock-outline: Temporal

Rayforce has three temporal types measured against a **2000-01-01** epoch, and
a timespan, a signed duration:

| Type | Unit | Atom constructor | Reader | Slice reader |
|------|------|------------------|--------|--------------|
| `Date` | days since 2000-01-01 | `Value::date_days(i32)` | `as_date_days` | `date_days_slice` |
| `Time` | milliseconds since midnight | `Value::time_millis(i32)` | `as_time_millis` | `time_millis_slice` |
| `Timestamp` | nanoseconds since 2000-01-01 UTC | `Value::timestamp_nanos(i64)` | `as_timestamp_nanos` | `timestamp_nanos_slice` |
| `Timespan` | nanoseconds, signed | `Value::timespan_nanos(i64)` | `as_timespan_nanos` | `timespan_nanos_slice` |

!!! note "Assume a live runtime"
    ```rust
    use rayforce::{Runtime, Value};
    // every snippet below runs inside:
    Runtime::scope(|rt| { /* … */ })?;
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
assert_eq!(Value::timespan_nanos(-1_500).as_timespan_nanos()?, -1_500);
```

Note the widths: `Date` and `Time` carry an `i32` offset, while `Timestamp`
and `Timespan` carry an `i64` (nanoseconds need the extra range).

A timespan is what subtracting two timestamps yields, and what a q timespan
decodes to. Its null is `i64::MIN`, as for a timestamp.

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

let span = chrono::Duration::milliseconds(-250);
assert_eq!(span.to_value().extract::<chrono::Duration>()?, span);
```

The mapping is `NaiveDate` ↔ `Date`, `NaiveTime` ↔ `Time`,
`DateTime<Utc>` ↔ `Timestamp`, and `chrono::Duration` ↔ `Timespan`. A
`chrono::Duration` past the ±~292 years an i64 of nanoseconds holds
saturates to that limit.

## Slice readers

For vectors there are dedicated zero-copy slice readers returning the raw offset
buffers. They reject non-temporal vectors:

```rust
let v = Value::vec(&[1i64, 2, 3]);
assert!(v.date_days_slice().is_err());
assert!(v.timestamp_nanos_slice().is_err());
```

- `date_days_slice()` and `time_millis_slice()` return `&[i32]`.
- `timestamp_nanos_slice()` and `timespan_nanos_slice()` return `&[i64]`.

!!! tip "Everything is relative to 2000-01-01"
    Unlike Unix time (epoch 1970), Rayforce dates and timestamps count from
    2000-01-01. The `chrono` conversions adjust automatically; the raw
    `*_days` / `*_nanos` constructors do not — pass an already-relative offset.
