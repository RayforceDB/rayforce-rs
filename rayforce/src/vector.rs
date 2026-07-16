//! Typed homogeneous vectors.
//!
//! The performance-critical path: [`Value::as_slice`] exposes a fixed-width
//! vector's storage as a zero-copy `&[T]`, and [`Value::vec`] builds one from a
//! `&[T]` in a single `memcpy` (`ray_vec_from_raw`) — both avoiding the
//! element-by-element FFI loop the Python bindings pay.
//!
//! Mutators (`set`/`push`) follow the core's copy-on-write move semantics: they
//! consume the current buffer and install the (possibly relocated) result.

use crate::error::{check, RayError, Result};
use crate::raw::{self, Raw};
use crate::value::Value;
use core::ffi::c_void;
use rayforce_sys as sys;

mod sealed {
    pub trait Sealed {}
}

/// A fixed-width vector element type, mapping a Rust scalar to a core vector
/// type tag for zero-copy slicing and bulk construction.
pub trait VecElem: Copy + sealed::Sealed {
    /// Canonical (positive) core vector type id.
    const RAY_TYPE: i8;
}

macro_rules! vec_elem {
    ($t:ty, $tag:expr) => {
        impl sealed::Sealed for $t {}
        impl VecElem for $t {
            const RAY_TYPE: i8 = $tag as i8;
        }
    };
}
vec_elem!(u8, sys::RAY_U8);
vec_elem!(i16, sys::RAY_I16);
vec_elem!(i32, sys::RAY_I32);
vec_elem!(i64, sys::RAY_I64);
vec_elem!(f32, sys::RAY_F32);
vec_elem!(f64, sys::RAY_F64);

impl Value {
    // ---- construction ----

    /// Build a vector from a slice of fixed-width elements (single `memcpy`).
    pub fn vec<T: VecElem>(data: &[T]) -> Value {
        unsafe {
            let p = check(sys::ray_vec_from_raw(
                T::RAY_TYPE,
                data.as_ptr() as *const c_void,
                data.len() as i64,
            ));
            match p {
                Ok(p) => Value::from_owned(p),
                Err(e) => panic!("rayforce: failed to build vector: {e}"),
            }
        }
    }

    /// Build a boolean vector (`RAY_BOOL`) from a slice of `bool`.
    pub fn bool_vec(data: &[bool]) -> Value {
        unsafe {
            // bool is a 1-byte 0/1 value — reinterpret as the byte buffer.
            let p = check(sys::ray_vec_from_raw(
                sys::RAY_BOOL as i8,
                data.as_ptr() as *const c_void,
                data.len() as i64,
            ));
            match p {
                Ok(p) => Value::from_owned(p),
                Err(e) => panic!("rayforce: failed to build bool vector: {e}"),
            }
        }
    }

    /// Build a symbol vector, interning each string.
    pub fn sym_vec<S: AsRef<str>>(items: &[S]) -> Value {
        unsafe {
            let mut v = match check(sys::ray_sym_vec_new(
                sys::RAY_SYM_W64 as u8,
                items.len() as i64,
            )) {
                Ok(v) => v,
                Err(e) => panic!("rayforce: failed to build symbol vector: {e}"),
            };
            for it in items {
                let s = it.as_ref();
                let id = sys::ray_sym_intern(s.as_ptr() as *const _, s.len());
                v = append_or_panic(v, &id as *const i64 as *const c_void, "symbol vector");
            }
            Value::from_owned(v)
        }
    }

    /// Build a string vector (`RAY_STR`).
    pub fn str_vec<S: AsRef<str>>(items: &[S]) -> Value {
        unsafe {
            let mut v = match check(sys::ray_vec_new(sys::RAY_STR as i8, items.len() as i64)) {
                Ok(v) => v,
                Err(e) => panic!("rayforce: failed to build string vector: {e}"),
            };
            for it in items {
                let s = it.as_ref();
                let next = sys::ray_str_vec_append(v, s.as_ptr() as *const _, s.len());
                v = match check(next) {
                    Ok(p) => p,
                    Err(e) => panic!("rayforce: failed to append to string vector: {e}"),
                };
            }
            Value::from_owned(v)
        }
    }

    /// Allocate an empty vector of the given canonical type with `capacity`.
    pub fn empty_vec(abs_type: i8, capacity: i64) -> Value {
        unsafe {
            match check(sys::ray_vec_new(abs_type, capacity)) {
                Ok(p) => Value::from_owned(p),
                Err(e) => panic!("rayforce: failed to allocate vector: {e}"),
            }
        }
    }

    // ---- inspection / reads ----

    /// Element count, for vectors / lists / dicts.
    pub fn len(&self) -> usize {
        unsafe { raw::len(self.as_ptr()).max(0) as usize }
    }

    /// True if the collection has zero elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Zero-copy view of a fixed-width vector's storage as `&[T]`.
    ///
    /// Errors if the vector's element type doesn't match `T`. The slice borrows
    /// `self`, so it cannot outlive the vector.
    pub fn as_slice<T: VecElem>(&self) -> Result<&[T]> {
        self.raw_slice::<T>(T::RAY_TYPE)
    }

    /// Zero-copy view of a boolean vector's storage (each byte is 0 or 1).
    pub fn bool_slice(&self) -> Result<&[u8]> {
        self.raw_slice::<u8>(sys::RAY_BOOL as i8)
    }

    /// Zero-copy view of a date vector as raw days since 2000-01-01.
    pub fn date_days_slice(&self) -> Result<&[i32]> {
        self.raw_slice::<i32>(sys::RAY_DATE as i8)
    }

    /// Zero-copy view of a time vector as raw milliseconds since midnight.
    pub fn time_millis_slice(&self) -> Result<&[i32]> {
        self.raw_slice::<i32>(sys::RAY_TIME as i8)
    }

    /// Zero-copy view of a timestamp vector as raw nanoseconds since
    /// 2000-01-01 UTC.
    pub fn timestamp_nanos_slice(&self) -> Result<&[i64]> {
        self.raw_slice::<i64>(sys::RAY_TIMESTAMP as i8)
    }

    /// Zero-copy slice of a vector whose element type tag equals `expected` and
    /// whose element width equals `size_of::<T>()`.
    fn raw_slice<T: Copy>(&self, expected: i8) -> Result<&[T]> {
        if !self.is_vec() {
            return Err(RayError::binding("slice: value is not a vector"));
        }
        if self.abs_type() != expected {
            return Err(RayError::binding(format!(
                "slice: vector element type {} != expected {}",
                self.abs_type(),
                expected
            )));
        }
        unsafe {
            let n = self.len();
            let ptr = raw::data(self.as_ptr()) as *const T;
            Ok(std::slice::from_raw_parts(ptr, n))
        }
    }

    /// True if element `idx` is null (consults the vector's null bitmap).
    pub fn is_null_at(&self, idx: usize) -> bool {
        unsafe { sys::ray_vec_is_null(self.as_ptr(), idx as i64) }
    }

    /// Box element `idx` as a [`Value`]. Returns the null singleton for null
    /// elements. Bounds-checked.
    pub fn get(&self, idx: usize) -> Result<Value> {
        let n = self.len();
        if idx >= n {
            return Err(RayError::binding(format!(
                "index {idx} out of range for length {n}"
            )));
        }
        // Heterogeneous list: elements are boxed values (borrowed from the list).
        if self.type_code() == sys::RAY_LIST as i8 {
            unsafe {
                let e = sys::ray_list_get(self.as_ptr(), idx as i64);
                return Ok(Value::from_borrowed(e));
            }
        }
        if self.is_null_at(idx) {
            return Ok(Value::null());
        }
        let t = self.abs_type() as u32;
        unsafe {
            let base = raw::data(self.as_ptr());
            let v = match t {
                sys::RAY_BOOL => Value::bool(*(base.add(idx)) != 0),
                sys::RAY_U8 => Value::u8(*base.add(idx)),
                sys::RAY_I16 => Value::i16(*(base.add(idx * 2) as *const i16)),
                sys::RAY_I32 => Value::i32(*(base.add(idx * 4) as *const i32)),
                sys::RAY_I64 => Value::i64(*(base.add(idx * 8) as *const i64)),
                sys::RAY_F32 => Value::f32(*(base.add(idx * 4) as *const f32)),
                sys::RAY_F64 => Value::f64(*(base.add(idx * 8) as *const f64)),
                sys::RAY_DATE => Value::date_days(*(base.add(idx * 4) as *const i32)),
                sys::RAY_TIME => Value::time_millis(*(base.add(idx * 4) as *const i32)),
                sys::RAY_TIMESTAMP => Value::timestamp_nanos(*(base.add(idx * 8) as *const i64)),
                sys::RAY_GUID => Value::guid(&*(base.add(idx * 16) as *const [u8; 16])),
                sys::RAY_SYM => {
                    // A cell is a *position* in the vector's symbol domain, not a
                    // runtime intern id. For the runtime domain the LUT is NULL
                    // and positions pass through; for FILE domains (loaded
                    // splayed columns) the LUT maps position -> runtime id.
                    let pos = sys::ray_vec_get_sym_id(self.as_ptr(), idx as i64);
                    let dom = raw::sym_domain(self.as_ptr());
                    let lut = sys::ray_sym_domain_runtime_lut(dom);
                    let id = if lut.is_null() {
                        pos
                    } else {
                        *lut.offset(pos as isize)
                    };
                    Value::from_owned(crate::error::check(sys::ray_sym(id))?)
                }
                sys::RAY_STR => {
                    let mut len: usize = 0;
                    let p = sys::ray_str_vec_get(self.as_ptr(), idx as i64, &mut len).cast::<u8>();
                    if p.is_null() {
                        Value::string("")
                    } else {
                        let bytes = std::slice::from_raw_parts(p, len);
                        Value::string(&String::from_utf8_lossy(bytes))
                    }
                }
                other => {
                    return Err(RayError::binding(format!(
                        "get: unsupported vector element type {other}"
                    )))
                }
            };
            Ok(v)
        }
    }

    /// Collect all elements, converting each via [`crate::FromValue`].
    pub fn to_vec<T: crate::FromValue>(&self) -> Result<Vec<T>> {
        let n = self.len();
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(T::from_value(&self.get(i)?)?);
        }
        Ok(out)
    }

    /// Iterate over boxed elements.
    pub fn iter(&self) -> VecIter<'_> {
        VecIter {
            value: self,
            idx: 0,
            len: self.len(),
        }
    }

    // ---- mutation (copy-on-write move semantics) ----

    /// Overwrite element `idx` with a fixed-width value.
    pub fn set<T: VecElem>(&mut self, idx: usize, elem: T) -> Result<()> {
        if self.abs_type() != T::RAY_TYPE {
            return Err(RayError::binding("set: element type mismatch"));
        }
        // Bounds-check here: ray_vec_set's range error returns *before* its
        // copy-on-write step, so it does not consume our reference — letting
        // install_mutation's null swap leak the vector. Reject up front.
        let n = self.len();
        if idx >= n {
            return Err(RayError::binding(format!(
                "set: index {idx} out of range for length {n}"
            )));
        }
        unsafe {
            let r = sys::ray_vec_set(
                self.as_ptr(),
                idx as i64,
                &elem as *const T as *const c_void,
            );
            self.install_mutation(r, "set")
        }
    }

    /// Append a fixed-width value.
    pub fn push<T: VecElem>(&mut self, elem: T) -> Result<()> {
        if self.abs_type() != T::RAY_TYPE {
            return Err(RayError::binding("push: element type mismatch"));
        }
        unsafe {
            let r = sys::ray_vec_append(self.as_ptr(), &elem as *const T as *const c_void);
            self.install_mutation(r, "push")
        }
    }

    /// Mark element `idx` as null (or clear it). Sets the vector's null sentinel
    /// and `HAS_NULLS` attribute so [`Value::is_null_at`] reports it.
    pub fn set_null(&mut self, idx: usize, is_null: bool) -> Result<()> {
        unsafe {
            let e = sys::ray_vec_set_null_checked(self.as_ptr(), idx as i64, is_null);
            if e != sys::ray_err_t_RAY_OK {
                return Err(RayError::binding(format!(
                    "set_null: failed (err {e}) — index {idx} out of range or unsupported type"
                )));
            }
        }
        Ok(())
    }

    /// A zero-copy sub-range view `[offset, offset+len)`.
    pub fn slice(&self, offset: i64, len: i64) -> Result<Value> {
        unsafe {
            let r = check(sys::ray_vec_slice(self.as_ptr(), offset, len))?;
            Ok(Value::from_owned(r))
        }
    }

    /// Concatenate two vectors into a new one.
    pub fn concat(&self, other: &Value) -> Result<Value> {
        unsafe {
            let r = check(sys::ray_vec_concat(self.as_ptr(), other.as_ptr()))?;
            Ok(Value::from_owned(r))
        }
    }

    /// Install the result of a consuming (cow) mutation, or surface an error.
    unsafe fn install_mutation(&mut self, result: Raw, op: &str) -> Result<()> {
        if raw::is_err(result) {
            let e = RayError::from_obj(result);
            // The cow path may already have released our old buffer; replace
            // with the null singleton so Drop cannot double-free.
            self.replace_ptr_consumed(raw::null_obj());
            return Err(RayError::binding(format!("{op}: {e}")));
        }
        self.replace_ptr_consumed(result);
        Ok(())
    }
}

/// Append helper for builders: consumes `v`, returns the new buffer or panics.
unsafe fn append_or_panic(v: Raw, elem: *const c_void, what: &str) -> Raw {
    let r = sys::ray_vec_append(v, elem);
    match check(r) {
        Ok(p) => p,
        Err(e) => panic!("rayforce: failed to append to {what}: {e}"),
    }
}

/// Iterator over a vector's boxed elements.
pub struct VecIter<'a> {
    value: &'a Value,
    idx: usize,
    len: usize,
}

impl Iterator for VecIter<'_> {
    type Item = Result<Value>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }
        let r = self.value.get(self.idx);
        self.idx += 1;
        Some(r)
    }
}
