//! Low-level FFI wrapper types and functions.

use crate::error::{RayforceError, Result};
use crate::*;
use std::ffi::CStr;
use std::fmt;

/// Helper to get the length of a vector/list from an obj_t
#[inline]
unsafe fn obj_len(obj: *mut obj_t) -> i64 {
    // The len field is in the anonymous struct inside the union
    (*obj).__bindgen_anon_1.__bindgen_anon_1.as_ref().len
}

/// Helper to get a pointer to the raw data of a vector
#[inline]
unsafe fn obj_raw_ptr(obj: *mut obj_t) -> *mut i8 {
    (*obj).__bindgen_anon_1.__bindgen_anon_1.as_ref().raw.as_ptr() as *mut i8
}

/// A safe wrapper around the Rayforce object pointer.
///
/// This type manages the lifecycle of Rayforce objects, ensuring proper
/// reference counting and memory management.
pub struct RayObj {
    ptr: *mut obj_t,
}

impl RayObj {
    /// Create a new RayObj from a raw pointer.
    ///
    /// # Safety
    /// The pointer must be a valid Rayforce object pointer.
    pub unsafe fn from_raw(ptr: *mut obj_t) -> Self {
        Self { ptr }
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut obj_t {
        self.ptr
    }

    /// Get the type code of the object.
    pub fn type_code(&self) -> i8 {
        unsafe { (*self.ptr).type_ }
    }

    /// Check if this is a null/nil object.
    pub fn is_nil(&self) -> bool {
        unsafe { is_null(self.ptr) == 1 }
    }

    /// Check if this is an error object.
    pub fn is_error(&self) -> bool {
        unsafe { (*self.ptr).type_ == TYPE_ERR as i8 }
    }

    /// Check if this is an atom (scalar).
    pub fn is_atom(&self) -> bool {
        unsafe { (*self.ptr).type_ < 0 }
    }

    /// Check if this is a vector.
    pub fn is_vector(&self) -> bool {
        let t = unsafe { (*self.ptr).type_ };
        t >= 0 && t <= TYPE_ENUM as i8
    }

    /// Get the length of the object (for vectors/lists).
    pub fn len(&self) -> i64 {
        unsafe { obj_len(self.ptr) }
    }

    /// Check if the object is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the reference count.
    pub fn ref_count(&self) -> u32 {
        unsafe { rc_obj(self.ptr) }
    }

    /// Get the attributes byte.
    pub fn attrs(&self) -> u8 {
        unsafe { (*self.ptr).attrs }
    }

    /// Set the attributes byte.
    pub fn set_attrs(&mut self, attrs: u8) {
        unsafe { (*self.ptr).attrs = attrs }
    }
}

impl Clone for RayObj {
    fn clone(&self) -> Self {
        unsafe { RayObj::from_raw(clone_obj(self.ptr)) }
    }
}

impl Drop for RayObj {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { drop_obj(self.ptr) }
        }
    }
}

impl fmt::Display for RayObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let obj = obj_fmt(self.ptr, 0);
            if obj.is_null() {
                write!(f, "null")
            } else {
                let len = obj_len(obj) as usize;
                let raw = obj_raw_ptr(obj) as *const u8;
                let bytes = std::slice::from_raw_parts(raw, len);
                let s = String::from_utf8_lossy(bytes);
                let result = write!(f, "{s}");
                drop_obj(obj);
                result
            }
        }
    }
}

impl fmt::Debug for RayObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let obj = obj_fmt(self.ptr, 1);
            if obj.is_null() {
                write!(f, "RayObj(null)")
            } else {
                let len = obj_len(obj) as usize;
                let raw = obj_raw_ptr(obj) as *const u8;
                let bytes = std::slice::from_raw_parts(raw, len);
                let s = String::from_utf8_lossy(bytes);
                let result = write!(f, "{s}");
                drop_obj(obj);
                result
            }
        }
    }
}

// Implement conversions FROM Rust types TO RayObj

impl From<bool> for RayObj {
    fn from(val: bool) -> Self {
        unsafe { RayObj::from_raw(b8(val as i8)) }
    }
}

impl From<u8> for RayObj {
    fn from(val: u8) -> Self {
        unsafe {
            let obj = atom(-(TYPE_U8 as i8));
            // Write to the union's memory location directly
            let ptr = &mut (*obj).__bindgen_anon_1 as *mut _ as *mut u8;
            *ptr = val;
            RayObj::from_raw(obj)
        }
    }
}

impl From<i16> for RayObj {
    fn from(val: i16) -> Self {
        unsafe {
            let obj = atom(-(TYPE_I16 as i8));
            let ptr = &mut (*obj).__bindgen_anon_1 as *mut _ as *mut i16;
            *ptr = val;
            RayObj::from_raw(obj)
        }
    }
}

impl From<i32> for RayObj {
    fn from(val: i32) -> Self {
        unsafe {
            let obj = atom(-(TYPE_I32 as i8));
            let ptr = &mut (*obj).__bindgen_anon_1 as *mut _ as *mut i32;
            *ptr = val;
            RayObj::from_raw(obj)
        }
    }
}

impl From<i64> for RayObj {
    fn from(val: i64) -> Self {
        unsafe {
            let obj = atom(-(TYPE_I64 as i8));
            let ptr = &mut (*obj).__bindgen_anon_1 as *mut _ as *mut i64;
            *ptr = val;
            RayObj::from_raw(obj)
        }
    }
}

impl From<f64> for RayObj {
    fn from(val: f64) -> Self {
        unsafe {
            let obj = atom(-(TYPE_F64 as i8));
            let ptr = &mut (*obj).__bindgen_anon_1 as *mut _ as *mut f64;
            *ptr = val;
            RayObj::from_raw(obj)
        }
    }
}

impl From<&str> for RayObj {
    fn from(val: &str) -> Self {
        unsafe {
            let ptr = string_from_str(val.as_ptr() as *const i8, val.len() as i64);
            RayObj::from_raw(ptr)
        }
    }
}

impl From<String> for RayObj {
    fn from(val: String) -> Self {
        RayObj::from(val.as_str())
    }
}

impl From<&[i64]> for RayObj {
    fn from(val: &[i64]) -> Self {
        unsafe {
            let obj = vector(TYPE_I64 as i8, val.len() as i64);
            let dst = obj_raw_ptr(obj) as *mut i64;
            std::ptr::copy_nonoverlapping(val.as_ptr(), dst, val.len());
            RayObj::from_raw(obj)
        }
    }
}

impl From<&[f64]> for RayObj {
    fn from(val: &[f64]) -> Self {
        unsafe {
            let obj = vector(TYPE_F64 as i8, val.len() as i64);
            let dst = obj_raw_ptr(obj) as *mut f64;
            std::ptr::copy_nonoverlapping(val.as_ptr(), dst, val.len());
            RayObj::from_raw(obj)
        }
    }
}

impl From<Vec<i64>> for RayObj {
    fn from(val: Vec<i64>) -> Self {
        RayObj::from(val.as_slice())
    }
}

impl From<Vec<f64>> for RayObj {
    fn from(val: Vec<f64>) -> Self {
        RayObj::from(val.as_slice())
    }
}

// Implement conversions FROM RayObj TO Rust types

impl TryFrom<RayObj> for i64 {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(TYPE_I64 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "I64".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(*(*obj.ptr).__bindgen_anon_1.i64_.as_ref()) }
    }
}

impl TryFrom<&RayObj> for i64 {
    type Error = RayforceError;

    fn try_from(obj: &RayObj) -> Result<Self> {
        if obj.type_code() != -(TYPE_I64 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "I64".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(*(*obj.ptr).__bindgen_anon_1.i64_.as_ref()) }
    }
}

impl TryFrom<RayObj> for i32 {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(TYPE_I32 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "I32".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(*(*obj.ptr).__bindgen_anon_1.i32_.as_ref()) }
    }
}

impl TryFrom<RayObj> for f64 {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(TYPE_F64 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "F64".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(*(*obj.ptr).__bindgen_anon_1.f64_.as_ref()) }
    }
}

impl TryFrom<RayObj> for bool {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(TYPE_B8 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "B8".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(*(*obj.ptr).__bindgen_anon_1.b8.as_ref() != 0) }
    }
}

impl TryFrom<RayObj> for String {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != TYPE_C8 as i8 {
            return Err(RayforceError::TypeMismatch {
                expected: "String".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe {
            let len = obj_len(obj.ptr) as usize;
            let raw = obj_raw_ptr(obj.ptr) as *const u8;
            let bytes = std::slice::from_raw_parts(raw, len);
            Ok(String::from_utf8_lossy(bytes).into_owned())
        }
    }
}

/// Get error message from an error object.
pub fn get_error_message(obj: *mut obj_t) -> String {
    unsafe {
        if obj.is_null() || (*obj).type_ != TYPE_ERR as i8 {
            return "Unknown error".to_string();
        }
        
        // Format the error object to get a string representation
        let formatted = obj_fmt(obj, 1);
        if formatted.is_null() {
            return "Error formatting failed".to_string();
        }
        
        let len = obj_len(formatted) as usize;
        let raw = obj_raw_ptr(formatted) as *const u8;
        let bytes = std::slice::from_raw_parts(raw, len);
        let result = String::from_utf8_lossy(bytes).into_owned();
        drop_obj(formatted);
        result
    }
}

/// Create a new list object.
pub fn new_list() -> RayObj {
    unsafe { RayObj::from_raw(vector(TYPE_LIST as i8, 0)) }
}

/// Create a new vector of a given type and length.
pub fn new_vector(type_code: i8, len: i64) -> RayObj {
    unsafe { RayObj::from_raw(vector(type_code, len)) }
}

/// Push an object to a list.
pub fn push_to_list(list: &mut RayObj, item: RayObj) {
    unsafe {
        let cloned = clone_obj(item.ptr);
        push_obj(&mut list.ptr as *mut *mut obj_t, cloned);
    }
}

/// Get item at index from a list/vector.
pub fn get_at_index(obj: &RayObj, idx: i64) -> Option<RayObj> {
    unsafe {
        let item = at_idx(obj.ptr, idx);
        if item.is_null() {
            None
        } else {
            Some(RayObj::from_raw(clone_obj(item)))
        }
    }
}

/// Insert item at index in a list/vector.
pub fn insert_at_index(obj: &mut RayObj, idx: i64, item: RayObj) {
    unsafe {
        let cloned = clone_obj(item.ptr);
        ins_obj(&mut obj.ptr as *mut *mut obj_t, idx, cloned);
    }
}

/// Create a symbol from a string.
pub fn new_symbol(s: &str) -> RayObj {
    unsafe {
        let ptr = symbol(s.as_ptr() as *const i8, s.len() as i64);
        RayObj::from_raw(ptr)
    }
}

/// Get the string representation of a symbol.
pub fn symbol_to_string(obj: &RayObj) -> Option<String> {
    if obj.type_code() != -(TYPE_SYMBOL as i8) {
        return None;
    }
    unsafe {
        let id = *(*obj.ptr).__bindgen_anon_1.i64_.as_ref();
        let cstr = str_from_symbol(id);
        if cstr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(cstr).to_string_lossy().into_owned())
        }
    }
}

/// Create a date from days since epoch (2000-01-01).
pub fn new_date(days: i32) -> RayObj {
    unsafe { RayObj::from_raw(adate(days)) }
}

/// Create a time from milliseconds since midnight.
pub fn new_time(ms: i32) -> RayObj {
    unsafe { RayObj::from_raw(atime(ms)) }
}

/// Create a timestamp from nanoseconds since epoch.
pub fn new_timestamp(ns: i64) -> RayObj {
    unsafe { RayObj::from_raw(timestamp(ns)) }
}

/// Create a table from keys (symbols) and values (list of columns).
pub fn new_table(keys: RayObj, values: RayObj) -> Result<RayObj> {
    unsafe {
        let tbl = table(keys.ptr, values.ptr);
        if tbl.is_null() {
            Err(RayforceError::AllocationFailed)
        } else {
            // Don't drop the keys and values as they're now owned by the table
            std::mem::forget(keys);
            std::mem::forget(values);
            Ok(RayObj::from_raw(tbl))
        }
    }
}

/// Create a dictionary from keys and values.
pub fn new_dict(keys: RayObj, values: RayObj) -> Result<RayObj> {
    unsafe {
        let d = dict(keys.ptr, values.ptr);
        if d.is_null() {
            Err(RayforceError::AllocationFailed)
        } else {
            std::mem::forget(keys);
            std::mem::forget(values);
            Ok(RayObj::from_raw(d))
        }
    }
}

/// Get internal function by name.
pub fn get_internal_function(name: &str) -> Option<RayObj> {
    let c_name = std::ffi::CString::new(name).ok()?;
    unsafe {
        let func = env_get_internal_function(c_name.as_ptr());
        if func.is_null() {
            None
        } else {
            Some(RayObj::from_raw(clone_obj(func)))
        }
    }
}

/// Get internal function name.
pub fn get_internal_name(obj: &RayObj) -> Option<String> {
    unsafe {
        let name = env_get_internal_name(obj.ptr);
        if name.is_null() {
            None
        } else {
            Some(CStr::from_ptr(name).to_string_lossy().into_owned())
        }
    }
}

/// Quote (clone) an object.
pub fn quote(obj: &RayObj) -> RayObj {
    unsafe { RayObj::from_raw(clone_obj(obj.ptr)) }
}

/// Assign a value to a symbol in the environment.
pub fn set_global(name: &str, value: &RayObj) -> Result<RayObj> {
    let sym = new_symbol(name);
    unsafe {
        let result = binary_set(sym.ptr, value.ptr);
        if result.is_null() {
            Err(RayforceError::CApiError("Failed to set global".into()))
        } else {
            Ok(RayObj::from_raw(result))
        }
    }
}

/// Get the length of an object (helper for external use)
pub fn get_obj_len(obj: &RayObj) -> i64 {
    unsafe { obj_len(obj.ptr) }
}

/// Get a pointer to the raw data of a vector (helper for external use)
pub fn get_obj_raw_ptr(obj: &RayObj) -> *mut u8 {
    unsafe { obj_raw_ptr(obj.ptr) as *mut u8 }
}
