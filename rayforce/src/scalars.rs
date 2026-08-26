//! Typed scalar (atom) constructors and readers on [`Value`].
//!
//! Constructors wrap the core `ray_*` atom builders; readers verify the type
//! tag and decode the payload. Temporal encodings match the engine: `date` =
//! days since 2000-01-01, `time` = milliseconds since midnight, `timestamp` =
//! nanoseconds since 2000-01-01 UTC.

use crate::error::{check, RayError, Result};
use crate::raw::{self, Raw};
use crate::runtime::assert_live;
use crate::value::Value;
use rayforce_sys as sys;

#[inline]
unsafe fn own(ptr: Raw) -> Value {
    // Atom constructors only fail on hard OOM (returning RAY_OOM_OBJ), which is
    // unrecoverable. Panic with the detail rather than silently substituting a
    // null — masking it would corrupt downstream type checks.
    match check(ptr) {
        Ok(p) => Value::from_owned(p),
        Err(e) => panic!("rayforce: failed to construct atom: {e}"),
    }
}

impl Value {
    // ---- constructors ----

    /// A boolean atom (`-RAY_BOOL`).
    pub fn bool(v: bool) -> Value {
        assert_live("Value::bool");
        unsafe { own(sys::ray_bool(v)) }
    }
    /// An unsigned byte atom (`-RAY_U8`).
    pub fn u8(v: u8) -> Value {
        assert_live("Value::u8");
        unsafe { own(sys::ray_u8(v)) }
    }
    /// A 16-bit signed integer atom (`-RAY_I16`).
    pub fn i16(v: i16) -> Value {
        assert_live("Value::i16");
        unsafe { own(sys::ray_i16(v)) }
    }
    /// A 32-bit signed integer atom (`-RAY_I32`).
    pub fn i32(v: i32) -> Value {
        assert_live("Value::i32");
        unsafe { own(sys::ray_i32(v)) }
    }
    /// A 64-bit signed integer atom (`-RAY_I64`).
    pub fn i64(v: i64) -> Value {
        assert_live("Value::i64");
        unsafe { own(sys::ray_i64(v)) }
    }
    /// A 32-bit float atom (`-RAY_F32`).
    pub fn f32(v: f32) -> Value {
        assert_live("Value::f32");
        unsafe { own(sys::ray_f32(v)) }
    }
    /// A 64-bit float atom (`-RAY_F64`).
    pub fn f64(v: f64) -> Value {
        assert_live("Value::f64");
        unsafe { own(sys::ray_f64(v)) }
    }

    /// A symbol atom (`-RAY_SYM`): interns `s` in the global table.
    pub fn sym(s: &str) -> Value {
        assert_live("Value::sym");
        unsafe {
            let id = sys::ray_sym_intern(s.as_ptr() as *const _, s.len());
            own(sys::ray_sym(id))
        }
    }

    /// A string atom (`-RAY_STR`).
    pub fn string(s: &str) -> Value {
        assert_live("Value::string");
        unsafe { own(sys::ray_str(s.as_ptr() as *const _, s.len())) }
    }

    /// A symbol atom used as a **name reference** (column / global-env lookup):
    /// the `ATTR_QUOTED` flag is cleared so the query compiler resolves it by
    /// name rather than treating it as a literal symbol.
    pub fn name_ref(name: &str) -> Value {
        assert_live("Value::name_ref");
        unsafe {
            let id = sys::ray_sym_intern(name.as_ptr() as *const _, name.len());
            let v = own(sys::ray_sym(id));
            raw::set_attrs(v.as_ptr(), 0);
            v
        }
    }

    /// A date atom: raw days since 2000-01-01.
    pub fn date_days(days: i32) -> Value {
        assert_live("Value::date_days");
        unsafe { own(sys::ray_date(days as i64)) }
    }
    /// A time atom: raw milliseconds since midnight.
    pub fn time_millis(ms: i32) -> Value {
        assert_live("Value::time_millis");
        unsafe { own(sys::ray_time(ms as i64)) }
    }
    /// A timestamp atom: raw nanoseconds since 2000-01-01 UTC.
    pub fn timestamp_nanos(ns: i64) -> Value {
        assert_live("Value::timestamp_nanos");
        unsafe { own(sys::ray_timestamp(ns)) }
    }

    /// A GUID atom from 16 raw bytes.
    pub fn guid(bytes: &[u8; 16]) -> Value {
        assert_live("Value::guid");
        unsafe { own(sys::ray_guid(bytes.as_ptr())) }
    }

    /// A typed null atom for the given canonical type id (e.g. [`sys::RAY_I64`]).
    pub fn typed_null(abs_type: i8) -> Value {
        assert_live("Value::typed_null");
        unsafe { own(sys::ray_typed_null(abs_type)) }
    }

    // ---- inspection ----

    /// True if this atom is a (typed or untyped) null.
    pub fn is_atom_null(&self) -> bool {
        unsafe { raw::atom_is_null(self.as_ptr()) }
    }

    // ---- readers ----

    /// Read a boolean atom.
    pub fn as_bool(&self) -> Result<bool> {
        self.expect_atom(sys::RAY_BOOL)?;
        Ok(unsafe { raw::as_b8(self.as_ptr()) })
    }
    /// Read an unsigned-byte atom.
    pub fn as_u8(&self) -> Result<u8> {
        self.expect_atom(sys::RAY_U8)?;
        Ok(unsafe { raw::as_u8(self.as_ptr()) })
    }
    /// Read an `i16` atom.
    pub fn as_i16(&self) -> Result<i16> {
        self.expect_atom(sys::RAY_I16)?;
        Ok(unsafe { raw::as_i16(self.as_ptr()) })
    }
    /// Read an `i32` atom.
    pub fn as_i32(&self) -> Result<i32> {
        self.expect_atom(sys::RAY_I32)?;
        Ok(unsafe { raw::as_i32(self.as_ptr()) })
    }
    /// Read an `i64` atom.
    pub fn as_i64(&self) -> Result<i64> {
        self.expect_atom(sys::RAY_I64)?;
        Ok(unsafe { raw::as_i64(self.as_ptr()) })
    }
    /// Read an `f32` atom.
    pub fn as_f32(&self) -> Result<f32> {
        self.expect_atom(sys::RAY_F32)?;
        Ok(unsafe { raw::as_f64(self.as_ptr()) as f32 })
    }
    /// Read an `f64` atom.
    pub fn as_f64(&self) -> Result<f64> {
        self.expect_atom(sys::RAY_F64)?;
        Ok(unsafe { raw::as_f64(self.as_ptr()) })
    }

    /// Read a symbol atom as an owned `String`.
    pub fn as_sym(&self) -> Result<String> {
        self.expect_atom(sys::RAY_SYM)?;
        unsafe {
            let id = raw::as_i64(self.as_ptr());
            let s = sys::ray_sym_str(id); // borrowed
            if s.is_null() {
                return Ok(String::new());
            }
            Ok(read_str_atom(s))
        }
    }

    /// Read a string atom as an owned `String`.
    pub fn as_string(&self) -> Result<String> {
        self.expect_atom(sys::RAY_STR)?;
        Ok(unsafe { read_str_atom(self.as_ptr()) })
    }

    /// Read a date atom as raw days since 2000-01-01.
    pub fn as_date_days(&self) -> Result<i32> {
        self.expect_atom(sys::RAY_DATE)?;
        Ok(unsafe { raw::as_i32(self.as_ptr()) })
    }
    /// Read a time atom as raw milliseconds since midnight.
    pub fn as_time_millis(&self) -> Result<i32> {
        self.expect_atom(sys::RAY_TIME)?;
        Ok(unsafe { raw::as_i32(self.as_ptr()) })
    }
    /// Read a timestamp atom as raw nanoseconds since 2000-01-01 UTC.
    pub fn as_timestamp_nanos(&self) -> Result<i64> {
        self.expect_atom(sys::RAY_TIMESTAMP)?;
        Ok(unsafe { raw::as_i64(self.as_ptr()) })
    }

    /// Read a GUID atom as 16 raw bytes.
    pub fn as_guid(&self) -> Result<[u8; 16]> {
        self.expect_atom(sys::RAY_GUID)?;
        unsafe { Ok(raw::guid_bytes(self.as_ptr()).unwrap_or([0u8; 16])) }
    }

    /// Verify this is an atom of the given canonical type id.
    fn expect_atom(&self, abs: u32) -> Result<()> {
        // Compare in i32 to avoid any wrap in `abs as i8` (all atom tags are
        // small, but keep the comparison exact regardless).
        if self.type_code() as i32 == -(abs as i32) {
            Ok(())
        } else {
            Err(RayError::binding(format!(
                "expected atom of type {}, got type tag {}",
                abs,
                self.type_code()
            )))
        }
    }
}

/// Read the bytes of a `-RAY_STR` atom into an owned `String`.
unsafe fn read_str_atom(p: Raw) -> String {
    let ptr = sys::ray_str_ptr(p).cast::<u8>();
    let n = sys::ray_str_len(p);
    if ptr.is_null() || n == 0 {
        String::new()
    } else {
        String::from_utf8_lossy(std::slice::from_raw_parts(ptr, n)).into_owned()
    }
}
