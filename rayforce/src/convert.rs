//! Conversions between native Rust types and [`Value`] — the Rust analogue of
//! the Python `python_to_ray` / `ray_to_python` helpers.
//!
//! [`ToValue`] builds a `Value` from a Rust value; [`FromValue`] extracts one.
//! Bare `&str`/`String` map to **symbols** (matching the Python bindings); wrap
//! in [`Str`] for a string atom. [`Guid`] carries 16 raw bytes. With the
//! `chrono` feature, date/time/timestamp types convert directly.

use crate::error::{RayError, Result};
use crate::value::Value;

/// Build a [`Value`] from a Rust value.
pub trait ToValue {
    fn to_value(&self) -> Value;
}

/// Extract a Rust value from a [`Value`].
pub trait FromValue: Sized {
    fn from_value(v: &Value) -> Result<Self>;
}

/// Wrapper that maps a string to a **string atom** (`-RAY_STR`) rather than the
/// default symbol mapping of bare `&str`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str<S: AsRef<str>>(pub S);

/// Wrapper carrying 16 raw GUID bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guid(pub [u8; 16]);

// ---- ToValue ----

impl ToValue for Value {
    fn to_value(&self) -> Value {
        self.clone()
    }
}
impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::bool(*self)
    }
}
impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::u8(*self)
    }
}
impl ToValue for i16 {
    fn to_value(&self) -> Value {
        Value::i16(*self)
    }
}
impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::i32(*self)
    }
}
impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::i64(*self)
    }
}
impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::f32(*self)
    }
}
impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::f64(*self)
    }
}
impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::sym(self)
    }
}
impl ToValue for &str {
    fn to_value(&self) -> Value {
        Value::sym(self)
    }
}
impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::sym(self)
    }
}
impl<S: AsRef<str>> ToValue for Str<S> {
    fn to_value(&self) -> Value {
        Value::string(self.0.as_ref())
    }
}
impl ToValue for Guid {
    fn to_value(&self) -> Value {
        Value::guid(&self.0)
    }
}
impl<T: ToValue> ToValue for &T {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}
impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(v) => v.to_value(),
            None => Value::null(),
        }
    }
}

// ---- FromValue ----

impl FromValue for Value {
    fn from_value(v: &Value) -> Result<Self> {
        Ok(v.clone())
    }
}
impl FromValue for bool {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_bool()
    }
}
impl FromValue for u8 {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_u8()
    }
}
impl FromValue for i16 {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_i16()
    }
}
impl FromValue for i32 {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_i32()
    }
}
impl FromValue for i64 {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_i64()
    }
}
impl FromValue for f32 {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_f32()
    }
}
impl FromValue for f64 {
    fn from_value(v: &Value) -> Result<Self> {
        v.as_f64()
    }
}
impl FromValue for String {
    /// Accepts a symbol or a string atom.
    fn from_value(v: &Value) -> Result<Self> {
        match v.as_sym() {
            Ok(s) => Ok(s),
            Err(_) => v.as_string(),
        }
    }
}
impl FromValue for Guid {
    fn from_value(v: &Value) -> Result<Self> {
        Ok(Guid(v.as_guid()?))
    }
}
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(v: &Value) -> Result<Self> {
        if v.is_null() || v.is_atom_null() {
            Ok(None)
        } else {
            Ok(Some(T::from_value(v)?))
        }
    }
}

impl Value {
    /// Convenience: build a `Value` from any [`ToValue`].
    pub fn new<T: ToValue>(v: T) -> Value {
        v.to_value()
    }

    /// Convenience: extract any [`FromValue`] from this value.
    pub fn extract<T: FromValue>(&self) -> Result<T> {
        T::from_value(self)
    }
}

// ---- chrono integration ----

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::*;
    use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike, Utc};

    fn date_epoch() -> NaiveDate {
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()
    }

    impl ToValue for NaiveDate {
        fn to_value(&self) -> Value {
            let days = self.signed_duration_since(date_epoch()).num_days() as i32;
            Value::date_days(days)
        }
    }
    impl ToValue for NaiveTime {
        fn to_value(&self) -> Value {
            let ms = self.num_seconds_from_midnight() as i64 * 1000
                + (self.nanosecond() as i64 / 1_000_000);
            Value::time_millis(ms as i32)
        }
    }
    impl ToValue for DateTime<Utc> {
        fn to_value(&self) -> Value {
            let epoch = date_epoch().and_hms_opt(0, 0, 0).unwrap().and_utc();
            let delta = self.signed_duration_since(epoch);
            // The engine timestamp is i64 nanoseconds (±~292 years from the
            // epoch). Dates outside that range can't be represented; saturate
            // toward the limit rather than silently collapsing to the epoch.
            // Avoid i64::MIN, which is the null sentinel.
            let ns = delta
                .num_nanoseconds()
                .unwrap_or(if delta.num_seconds() < 0 {
                    i64::MIN + 1
                } else {
                    i64::MAX
                });
            Value::timestamp_nanos(ns)
        }
    }

    impl FromValue for NaiveDate {
        fn from_value(v: &Value) -> Result<Self> {
            let days = v.as_date_days()?;
            date_epoch()
                .checked_add_signed(chrono::Duration::days(days as i64))
                .ok_or_else(|| RayError::binding("date out of range"))
        }
    }
    impl FromValue for NaiveTime {
        fn from_value(v: &Value) -> Result<Self> {
            let ms = v.as_time_millis()? as u32;
            NaiveTime::from_num_seconds_from_midnight_opt(ms / 1000, (ms % 1000) * 1_000_000)
                .ok_or_else(|| RayError::binding("time out of range"))
        }
    }
    impl FromValue for DateTime<Utc> {
        fn from_value(v: &Value) -> Result<Self> {
            let ns = v.as_timestamp_nanos()?;
            let epoch = date_epoch().and_hms_opt(0, 0, 0).unwrap().and_utc();
            epoch
                .checked_add_signed(chrono::Duration::nanoseconds(ns))
                .ok_or_else(|| RayError::binding("timestamp out of range"))
        }
    }

    // Silence unused-import warnings when only some traits are exercised.
    #[allow(unused_imports)]
    use {Datelike as _, Timelike as _};
}
