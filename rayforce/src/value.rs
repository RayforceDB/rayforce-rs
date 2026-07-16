//! [`Value`] — the safe, reference-counted handle to a core `ray_t` object.
//!
//! `Value` owns exactly one reference. `Clone` retains, `Drop` releases (both
//! no-ops for the null singleton and error objects, per the core). It is
//! deliberately `!Send`/`!Sync`: the core is single-threaded with a
//! thread-local VM, so values must never cross threads.

use crate::error::{self, Result};
use crate::raw::{self, Raw};
use rayforce_sys as sys;
use std::fmt;
use std::marker::PhantomData;

/// A handle to a RayforceDB object (atom, vector, list, dict, table, …).
pub struct Value {
    ptr: Raw,
    /// Makes `Value` `!Send` + `!Sync`.
    _not_send: PhantomData<*mut ()>,
}

impl Value {
    /// Wrap a pointer whose reference this `Value` now owns (the common case for
    /// values returned by core constructors and `eval`).
    ///
    /// # Safety
    /// `ptr` must be a valid, non-error core pointer carrying a reference that
    /// is transferred to the returned `Value`.
    pub(crate) unsafe fn from_owned(ptr: Raw) -> Value {
        debug_assert!(!ptr.is_null(), "from_owned: null pointer");
        // Defensive: value-null must be RAY_NULL_OBJ, never a bare C NULL.
        // Normalize so a stray NULL can never reach `ray_release` on Drop.
        let ptr = if ptr.is_null() { raw::null_obj() } else { ptr };
        Value {
            ptr,
            _not_send: PhantomData,
        }
    }

    /// Wrap a borrowed pointer, taking a new reference (`ray_retain`). Use for
    /// pointers the core still owns (e.g. `ray_list_get`, `ray_dict_keys`).
    ///
    /// # Safety
    /// `ptr` must be a valid core pointer.
    pub(crate) unsafe fn from_borrowed(ptr: Raw) -> Value {
        sys::ray_retain(ptr);
        Value {
            ptr,
            _not_send: PhantomData,
        }
    }

    /// The untyped null singleton (`RAY_NULL_OBJ`).
    pub fn null() -> Value {
        // Arena-allocated singleton: retain/release are no-ops.
        Value {
            ptr: raw::null_obj(),
            _not_send: PhantomData,
        }
    }

    /// Borrow the underlying raw pointer (does not transfer ownership).
    #[inline]
    pub(crate) fn as_ptr(&self) -> Raw {
        self.ptr
    }

    /// Consume `self`, returning the raw pointer and its reference (no release).
    #[inline]
    pub(crate) fn into_raw(self) -> Raw {
        let p = self.ptr;
        std::mem::forget(self);
        p
    }

    /// Replace the held pointer after a C call that already **consumed** our old
    /// reference (copy-on-write move semantics) and returned `new` as an owned
    /// reference. The old pointer is not released here.
    #[inline]
    pub(crate) unsafe fn replace_ptr_consumed(&mut self, new: Raw) {
        self.ptr = new;
    }

    /// The signed type tag (negative = atom, positive = vector, 0 = list, …).
    #[inline]
    pub fn type_code(&self) -> i8 {
        unsafe { raw::type_code(self.ptr) }
    }

    /// `|type|` — the canonical (unsigned) type id.
    #[inline]
    pub fn abs_type(&self) -> i8 {
        unsafe { raw::abs_type(self.ptr) }
    }

    /// True for atoms (scalars and function objects).
    #[inline]
    pub fn is_atom(&self) -> bool {
        unsafe { raw::is_atom(self.ptr) }
    }

    /// True for homogeneous vectors (bool…str).
    #[inline]
    pub fn is_vec(&self) -> bool {
        unsafe { raw::is_vec(self.ptr) }
    }

    /// True if this is the null singleton.
    #[inline]
    pub fn is_null(&self) -> bool {
        raw::is_null_singleton(self.ptr)
    }

    /// Element / pair count for vectors, lists, and dicts; for other objects the
    /// raw `len` field (not meaningful for atoms).
    #[inline]
    pub fn len_raw(&self) -> i64 {
        unsafe { raw::len(self.ptr) }
    }

    /// Current core reference count (diagnostic).
    #[inline]
    pub fn ref_count(&self) -> u32 {
        unsafe { raw::rc(self.ptr) }
    }

    /// Pretty-print via the core formatter (`ray_fmt`).
    pub fn format(&self) -> String {
        unsafe {
            let s = sys::ray_fmt(self.ptr, 1);
            if s.is_null() {
                return String::new();
            }
            // ray_fmt can return an error object; don't read it as a string
            // (and ray_release is a no-op on errors, so free explicitly).
            if raw::is_err(s) {
                sys::ray_error_free(s);
                return String::new();
            }
            let p = sys::ray_str_ptr(s).cast::<u8>();
            let n = sys::ray_str_len(s);
            let out = if p.is_null() || n == 0 {
                String::new()
            } else {
                String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned()
            };
            sys::ray_release(s);
            out
        }
    }

    /// Serialize to a byte vector (core wire format with IPC header).
    pub fn serialize(&self) -> Result<Vec<u8>> {
        unsafe {
            let ser = error::check(sys::ray_ser(self.ptr))?;
            if ser.is_null() {
                return Err(crate::error::RayError::binding("serialize returned null"));
            }
            let v = Value::from_owned(ser);
            let p = raw::data(v.ptr) as *const u8;
            let n = raw::len(v.ptr) as usize;
            Ok(std::slice::from_raw_parts(p, n).to_vec())
        }
    }

    /// Deserialize a value from bytes previously produced by [`Value::serialize`]
    /// (or another Rayforce wire-format encoder).
    pub fn deserialize(bytes: &[u8]) -> Result<Value> {
        // Wrap the bytes in a U8 vector for ray_de.
        let buf = Value::vec(bytes);
        unsafe {
            let de = error::check(sys::ray_de(buf.ptr))?;
            if de.is_null() {
                return Err(crate::error::RayError::binding("deserialize returned null"));
            }
            Ok(Value::from_owned(error::materialize(de)?))
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        unsafe {
            sys::ray_retain(self.ptr);
            Value {
                ptr: self.ptr,
                _not_send: PhantomData,
            }
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        unsafe { sys::ray_release(self.ptr) }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value(type={}, {})", self.type_code(), self.format())
    }
}
