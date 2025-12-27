/*
*   Copyright (c) 2025 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

//! Scalar types for Rayforce.

use crate::error::{RayforceError, Result};
use crate::ffi::RayObj;
use crate::types::RayType;
use crate::*;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::fmt;
use uuid::Uuid;

/// Boolean type.
#[derive(Clone)]
pub struct RayBool {
    ptr: RayObj,
}

impl RayBool {
    /// Create a new boolean.
    pub fn new(value: bool) -> Self {
        Self {
            ptr: RayObj::from(value),
        }
    }

    /// Get the boolean value.
    pub fn value(&self) -> bool {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.b8.as_ref() != 0 }
    }
}

impl RayType for RayBool {
    const TYPE_CODE: i8 = -(TYPE_B8 as i8);
    const RAY_NAME: &'static str = "RayBool";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayBool({})", self.value())
    }
}

impl fmt::Display for RayBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<bool> for RayBool {
    fn from(val: bool) -> Self {
        RayBool::new(val)
    }
}

impl From<RayBool> for bool {
    fn from(b: RayBool) -> Self {
        b.value()
    }
}

/// Type alias for backward compatibility.
pub type B8 = RayBool;

/// Unsigned byte type.
#[derive(Clone)]
pub struct RayU8 {
    ptr: RayObj,
}

impl RayU8 {
    /// Create a new unsigned byte.
    pub fn new(value: u8) -> Self {
        Self {
            ptr: RayObj::from(value),
        }
    }

    /// Get the byte value.
    pub fn value(&self) -> u8 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.u8_.as_ref() }
    }
}

impl RayType for RayU8 {
    const TYPE_CODE: i8 = -(TYPE_U8 as i8);
    const RAY_NAME: &'static str = "RayU8";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayU8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayU8({})", self.value())
    }
}

impl fmt::Display for RayU8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Type alias for backward compatibility.
pub type U8 = RayU8;

/// Character type.
#[derive(Clone)]
pub struct RayChar {
    ptr: RayObj,
}

impl RayChar {
    /// Create a new character.
    pub fn new(value: char) -> Self {
        unsafe {
            let ptr = c8(value as i8);
            Self {
                ptr: RayObj::from_raw(ptr),
            }
        }
    }

    /// Get the character value.
    pub fn value(&self) -> char {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.c8.as_ref() as u8 as char }
    }
}

impl RayType for RayChar {
    const TYPE_CODE: i8 = -(TYPE_C8 as i8);
    const RAY_NAME: &'static str = "RayChar";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayChar({:?})", self.value())
    }
}

impl fmt::Display for RayChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Type alias for backward compatibility.
pub type C8 = RayChar;

/// 16-bit integer type.
#[derive(Clone)]
pub struct RayI16 {
    ptr: RayObj,
}

impl RayI16 {
    /// Create a new RayI16.
    pub fn new(value: i16) -> Self {
        Self {
            ptr: RayObj::from(value),
        }
    }

    /// Get the integer value.
    pub fn value(&self) -> i16 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.i16_.as_ref() }
    }
}

impl RayType for RayI16 {
    const TYPE_CODE: i8 = -(TYPE_I16 as i8);
    const RAY_NAME: &'static str = "RayI16";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayI16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayI16({})", self.value())
    }
}

impl fmt::Display for RayI16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Type alias for backward compatibility.
pub type I16 = RayI16;

/// 32-bit integer type.
#[derive(Clone)]
pub struct RayI32 {
    ptr: RayObj,
}

impl RayI32 {
    /// Create a new RayI32.
    pub fn new(value: i32) -> Self {
        Self {
            ptr: RayObj::from(value),
        }
    }

    /// Get the integer value.
    pub fn value(&self) -> i32 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.i32_.as_ref() }
    }
}

impl RayType for RayI32 {
    const TYPE_CODE: i8 = -(TYPE_I32 as i8);
    const RAY_NAME: &'static str = "RayI32";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayI32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayI32({})", self.value())
    }
}

impl fmt::Display for RayI32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

/// Type alias for backward compatibility.
pub type I32 = RayI32;

/// 64-bit integer type.
#[derive(Clone)]
pub struct RayI64 {
    ptr: RayObj,
}

impl RayI64 {
    /// Create a new RayI64.
    pub fn new(value: i64) -> Self {
        Self {
            ptr: RayObj::from(value),
        }
    }

    /// Get the integer value.
    pub fn value(&self) -> i64 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.i64_.as_ref() }
    }
}

impl RayType for RayI64 {
    const TYPE_CODE: i8 = -(TYPE_I64 as i8);
    const RAY_NAME: &'static str = "RayI64";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayI64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayI64({})", self.value())
    }
}

impl fmt::Display for RayI64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<i64> for RayI64 {
    fn from(val: i64) -> Self {
        RayI64::new(val)
    }
}

impl From<RayI64> for i64 {
    fn from(i: RayI64) -> Self {
        i.value()
    }
}

/// Type alias for backward compatibility.
pub type I64 = RayI64;

/// 64-bit floating point type.
#[derive(Clone)]
pub struct RayF64 {
    ptr: RayObj,
}

impl RayF64 {
    /// Create a new RayF64.
    pub fn new(value: f64) -> Self {
        Self {
            ptr: RayObj::from(value),
        }
    }

    /// Get the float value.
    pub fn value(&self) -> f64 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.f64_.as_ref() }
    }
}

impl RayType for RayF64 {
    const TYPE_CODE: i8 = -(TYPE_F64 as i8);
    const RAY_NAME: &'static str = "RayF64";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayF64({})", self.value())
    }
}

impl fmt::Display for RayF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl From<f64> for RayF64 {
    fn from(val: f64) -> Self {
        RayF64::new(val)
    }
}

impl From<RayF64> for f64 {
    fn from(f: RayF64) -> Self {
        f.value()
    }
}

/// Type alias for backward compatibility.
pub type F64 = RayF64;

/// Symbol type.
#[derive(Clone)]
pub struct RaySymbol {
    ptr: RayObj,
}

impl RaySymbol {
    /// Create a new symbol.
    pub fn new(value: &str) -> Self {
        Self {
            ptr: crate::ffi::new_symbol(value),
        }
    }

    /// Get the symbol value as a string.
    pub fn value(&self) -> String {
        crate::ffi::symbol_to_string(&self.ptr).unwrap_or_default()
    }
}

impl RayType for RaySymbol {
    const TYPE_CODE: i8 = -(TYPE_SYMBOL as i8);
    const RAY_NAME: &'static str = "RaySymbol";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RaySymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RaySymbol(`{})", self.value())
    }
}

impl fmt::Display for RaySymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}", self.value())
    }
}

impl From<&str> for RaySymbol {
    fn from(val: &str) -> Self {
        RaySymbol::new(val)
    }
}

/// Type alias for backward compatibility.
pub type Symbol = RaySymbol;

/// Quoted symbol (for use in expressions).
#[derive(Clone)]
pub struct RayQuotedSymbol {
    ptr: RayObj,
}

impl RayQuotedSymbol {
    /// Create a new quoted symbol.
    pub fn new(value: &str) -> Self {
        let sym = RaySymbol::new(value);
        unsafe {
            // Clone the symbol to "quote" it
            let quoted = clone_obj(sym.ptr.as_ptr());
            Self {
                ptr: RayObj::from_raw(quoted),
            }
        }
    }

    /// Get the symbol value as a string.
    pub fn value(&self) -> String {
        crate::ffi::symbol_to_string(&self.ptr).unwrap_or_default()
    }
}

impl RayType for RayQuotedSymbol {
    const TYPE_CODE: i8 = -(TYPE_SYMBOL as i8);
    const RAY_NAME: &'static str = "RayQuotedSymbol";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayQuotedSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayQuotedSymbol(`{})", self.value())
    }
}

/// Type alias for backward compatibility.
pub type QuotedSymbol = RayQuotedSymbol;

/// Date type (days since 2000-01-01).
#[derive(Clone)]
pub struct RayDate {
    ptr: RayObj,
}

impl RayDate {
    /// Create a new date from days since epoch (2000-01-01).
    pub fn from_days(days: i32) -> Self {
        Self {
            ptr: crate::ffi::new_date(days),
        }
    }

    /// Create a new date from a NaiveDate.
    pub fn from_naive_date(date: NaiveDate) -> Self {
        let epoch = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let days = (date - epoch).num_days() as i32;
        Self::from_days(days)
    }

    /// Get the days since epoch.
    pub fn days(&self) -> i32 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.i32_.as_ref() }
    }

    /// Get the date as a NaiveDate.
    pub fn to_naive_date(&self) -> NaiveDate {
        let epoch = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        epoch + chrono::Duration::days(self.days() as i64)
    }
}

impl RayType for RayDate {
    const TYPE_CODE: i8 = -(TYPE_DATE as i8);
    const RAY_NAME: &'static str = "RayDate";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayDate({})", self.to_naive_date())
    }
}

impl fmt::Display for RayDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_naive_date())
    }
}

impl From<NaiveDate> for RayDate {
    fn from(date: NaiveDate) -> Self {
        RayDate::from_naive_date(date)
    }
}

/// Type alias for backward compatibility.
pub type Date = RayDate;

/// Time type (milliseconds since midnight).
#[derive(Clone)]
pub struct RayTime {
    ptr: RayObj,
}

impl RayTime {
    /// Create a new time from milliseconds since midnight.
    pub fn from_ms(ms: i32) -> Self {
        Self {
            ptr: crate::ffi::new_time(ms),
        }
    }

    /// Create a new time from a NaiveTime.
    pub fn from_naive_time(time: NaiveTime) -> Self {
        let ms = time.num_seconds_from_midnight() as i32 * 1000
            + time.nanosecond() as i32 / 1_000_000;
        Self::from_ms(ms)
    }

    /// Get the milliseconds since midnight.
    pub fn ms(&self) -> i32 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.i32_.as_ref() }
    }

    /// Get the time as a NaiveTime.
    pub fn to_naive_time(&self) -> NaiveTime {
        let ms = self.ms();
        let secs = (ms / 1000) as u32;
        let nanos = ((ms % 1000) * 1_000_000) as u32;
        NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap())
    }
}

impl RayType for RayTime {
    const TYPE_CODE: i8 = -(TYPE_TIME as i8);
    const RAY_NAME: &'static str = "RayTime";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayTime({})", self.to_naive_time())
    }
}

impl fmt::Display for RayTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_naive_time())
    }
}

impl From<NaiveTime> for RayTime {
    fn from(time: NaiveTime) -> Self {
        RayTime::from_naive_time(time)
    }
}

/// Type alias for backward compatibility.
pub type Time = RayTime;

/// Timestamp type (nanoseconds since epoch).
#[derive(Clone)]
pub struct RayTimestamp {
    ptr: RayObj,
}

impl RayTimestamp {
    /// Create a new timestamp from nanoseconds since epoch.
    pub fn from_nanos(ns: i64) -> Self {
        Self {
            ptr: crate::ffi::new_timestamp(ns),
        }
    }

    /// Create a new timestamp from a NaiveDateTime.
    pub fn from_naive_datetime(dt: NaiveDateTime) -> Self {
        let ns = dt.and_utc().timestamp_nanos_opt().unwrap_or(0);
        Self::from_nanos(ns)
    }

    /// Get the nanoseconds since epoch.
    pub fn nanos(&self) -> i64 {
        unsafe { *(*self.ptr.as_ptr()).__bindgen_anon_1.i64_.as_ref() }
    }

    /// Get the timestamp as a NaiveDateTime.
    pub fn to_naive_datetime(&self) -> NaiveDateTime {
        let ns = self.nanos();
        let secs = ns / 1_000_000_000;
        let nsec = (ns % 1_000_000_000) as u32;
        chrono::DateTime::from_timestamp(secs, nsec)
            .map(|dt| dt.naive_utc())
            .unwrap_or_default()
    }
}

impl RayType for RayTimestamp {
    const TYPE_CODE: i8 = -(TYPE_TIMESTAMP as i8);
    const RAY_NAME: &'static str = "RayTimestamp";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayTimestamp({})", self.to_naive_datetime())
    }
}

impl fmt::Display for RayTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_naive_datetime())
    }
}

impl From<NaiveDateTime> for RayTimestamp {
    fn from(dt: NaiveDateTime) -> Self {
        RayTimestamp::from_naive_datetime(dt)
    }
}

/// Type alias for backward compatibility.
pub type Timestamp = RayTimestamp;

/// GUID type.
#[derive(Clone)]
pub struct RayGuid {
    ptr: RayObj,
}

impl RayGuid {
    /// Create a new GUID from a UUID.
    pub fn new(uuid: Uuid) -> Self {
        let bytes = uuid.as_bytes();
        unsafe {
            let obj = vector(TYPE_I64 as i8, 2);
            (*obj).type_ = -(TYPE_GUID as i8);
            let dst = (obj as *mut u8).add(std::mem::size_of::<obj_t>() - 8).add(8);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, 16);
            Self {
                ptr: RayObj::from_raw(obj),
            }
        }
    }

    /// Create a new random GUID.
    pub fn random() -> Self {
        Self::new(Uuid::new_v4())
    }

    /// Parse a GUID from a string.
    pub fn parse(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s)
            .map_err(|e| RayforceError::InvalidGuid(e.to_string()))?;
        Ok(Self::new(uuid))
    }

    /// Get the GUID as a UUID.
    pub fn to_uuid(&self) -> Uuid {
        unsafe {
            let raw = (self.ptr.as_ptr() as *const u8)
                .add(std::mem::size_of::<obj_t>() - 8)
                .add(8);
            let bytes: [u8; 16] = std::slice::from_raw_parts(raw, 16)
                .try_into()
                .unwrap_or([0; 16]);
            Uuid::from_bytes(bytes)
        }
    }
}

impl RayType for RayGuid {
    const TYPE_CODE: i8 = -(TYPE_GUID as i8);
    const RAY_NAME: &'static str = "RayGuid";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayGuid({})", self.to_uuid())
    }
}

impl fmt::Display for RayGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uuid())
    }
}

impl From<Uuid> for RayGuid {
    fn from(uuid: Uuid) -> Self {
        RayGuid::new(uuid)
    }
}

/// Type alias for backward compatibility.
pub type GUID = RayGuid;
