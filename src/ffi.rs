/*
*   Copyright (c) 2025-2026 Anton Kundenko <singaraiona@gmail.com>
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

//! Low-level FFI wrapper types and functions for rayforce 2.0.

use crate::error::{RayforceError, Result};
use crate::*;
use std::ffi::CStr;
use std::fmt;

// ===== Raw accessors over the bindgen-generated ray_t layout =====
//
// `ray_t` is a typedef'd top-level union of an "allocated" struct and a
// "free-block" struct.  Bindgen renders that as a struct with two
// `__BindgenUnionField` arms.  All public ray_t pointers we ever see at
// runtime are in the allocated form, so every accessor below treats the
// allocated arm as the canonical one.

#[inline]
unsafe fn alloc_arm<'a>(p: *const ray_t) -> &'a ray_t__bindgen_ty_1 {
    // SAFETY: caller guarantees `p` is a valid live ray_t pointer. The
    // engine only hands out allocated-form objects through the public
    // API, so reading the allocated arm is sound.
    (*p).__bindgen_anon_1.as_ref()
}

// These accessors are named with a `read_` prefix to avoid shadowing
// the bindgen-generated C constructors `ray_bool` / `ray_i64` / `ray_f64`
// / etc., which live in the same module namespace via `use crate::*;`.

#[inline]
pub(crate) unsafe fn read_type(p: *const ray_t) -> i8 {
    alloc_arm(p).type_
}

#[inline]
pub(crate) unsafe fn read_attrs(p: *const ray_t) -> u8 {
    alloc_arm(p).attrs
}

#[inline]
pub(crate) unsafe fn read_rc(p: *const ray_t) -> u32 {
    alloc_arm(p).rc
}

#[inline]
pub(crate) unsafe fn read_len(p: *const ray_t) -> i64 {
    alloc_arm(p).__bindgen_anon_2.len
}

#[inline]
pub(crate) unsafe fn read_b8(p: *const ray_t) -> u8 {
    alloc_arm(p).__bindgen_anon_2.b8
}

#[inline]
pub(crate) unsafe fn read_u8(p: *const ray_t) -> u8 {
    alloc_arm(p).__bindgen_anon_2.u8_
}

#[inline]
pub(crate) unsafe fn read_i16(p: *const ray_t) -> i16 {
    alloc_arm(p).__bindgen_anon_2.i16_
}

#[inline]
pub(crate) unsafe fn read_i32(p: *const ray_t) -> i32 {
    alloc_arm(p).__bindgen_anon_2.i32_
}

#[inline]
pub(crate) unsafe fn read_i64(p: *const ray_t) -> i64 {
    alloc_arm(p).__bindgen_anon_2.i64_
}

#[inline]
pub(crate) unsafe fn read_f64(p: *const ray_t) -> f64 {
    alloc_arm(p).__bindgen_anon_2.f64_
}

#[inline]
pub(crate) unsafe fn read_obj_field(p: *const ray_t) -> *mut ray_t {
    alloc_arm(p).__bindgen_anon_2.obj
}

#[inline]
pub(crate) unsafe fn read_slen(p: *const ray_t) -> u8 {
    alloc_arm(p).__bindgen_anon_2.__bindgen_anon_1.slen
}

#[inline]
pub(crate) unsafe fn read_sdata(p: *const ray_t) -> *const i8 {
    alloc_arm(p)
        .__bindgen_anon_2
        .__bindgen_anon_1
        .sdata
        .as_ptr() as *const i8
}

/// Slice-aware element data pointer for a vector.
///
/// Mirrors `ray_data` from the C header: if the vector is a slice, walks
/// to the parent's data; otherwise returns the inline `data[]` pointer
/// (which sits immediately after the 32-byte header).
#[inline]
pub(crate) unsafe fn read_data_ptr(p: *const ray_t) -> *mut u8 {
    let attrs = read_attrs(p);
    let elem_type = read_type(p);
    if attrs & RAY_ATTR_SLICE as u8 != 0 {
        let arm = alloc_arm(p);
        let slice = &arm.__bindgen_anon_1.__bindgen_anon_1;
        let parent = slice.slice_parent;
        let offset = slice.slice_offset;
        let elem_size = ray_type_sizes[elem_type as u8 as usize] as i64;
        let parent_data = (parent as *mut u8).add(std::mem::size_of::<ray_t>());
        parent_data.add((offset * elem_size) as usize)
    } else {
        (p as *mut u8).add(std::mem::size_of::<ray_t>())
    }
}

/// Test whether a pointer points to a Rayforce error object.
///
/// Mirrors the `RAY_IS_ERR(p)` macro — non-NULL, not in the low 32-byte
/// reserved range, and tagged `RAY_ERROR` (127).
#[inline]
pub(crate) unsafe fn is_error(p: *const ray_t) -> bool {
    !p.is_null() && (p as usize) > 31 && read_type(p) == RAY_ERROR as i8
}

/// Test whether a pointer is the global null singleton.
#[inline]
pub(crate) unsafe fn is_null_obj(p: *const ray_t) -> bool {
    p == &raw const __ray_null
}

// ===== RayObj — owned ray_t* with rc-managed lifetime =====

/// A safe wrapper around a Rayforce object pointer.
///
/// Owns one strong reference: `Clone` calls `ray_retain`, `Drop` calls
/// `ray_release`. The pointer may be NULL (treated as a void / "no
/// result" sentinel — release is a no-op there).
pub struct RayObj {
    ptr: *mut ray_t,
}

impl RayObj {
    /// Wrap an existing `ray_t*`. The wrapper takes ownership of one
    /// strong reference; do not call `ray_release` on the pointer
    /// afterwards.
    ///
    /// # Safety
    /// `ptr` must either be NULL or a valid Rayforce object pointer
    /// with at least one strong reference that the caller is donating
    /// to this wrapper.
    pub unsafe fn from_raw(ptr: *mut ray_t) -> Self {
        Self { ptr }
    }

    /// Get the raw pointer (unowned borrow).
    pub fn as_ptr(&self) -> *mut ray_t {
        self.ptr
    }

    /// The 2.0 type tag (negative for atoms, 0 for `RAY_LIST`,
    /// positive for vectors / compounds).
    pub fn type_code(&self) -> i8 {
        if self.ptr.is_null() {
            0
        } else {
            unsafe { read_type(self.ptr) }
        }
    }

    /// True for the global null singleton or a NULL pointer.
    pub fn is_nil(&self) -> bool {
        if self.ptr.is_null() {
            return true;
        }
        unsafe { is_null_obj(self.ptr) }
    }

    /// True when the wrapped object is a Rayforce error.
    pub fn is_error(&self) -> bool {
        unsafe { is_error(self.ptr) }
    }

    /// True for atoms (negative type tag, or function/error/null sentinels).
    pub fn is_atom(&self) -> bool {
        let t = self.type_code();
        t < 0 || t >= RAY_LAMBDA as i8
    }

    /// True when the type tag is in the vector range
    /// (`RAY_BOOL ..= RAY_STR`, plus `RAY_LIST`).
    pub fn is_vector(&self) -> bool {
        let t = self.type_code();
        t >= 0 && t <= RAY_STR as i8
    }

    /// Length of vectors / lists / strings (0 for non-string atoms and
    /// NULL). For `RAY_STR` atoms reads the SSO/pool length via
    /// `ray_str_len`, since the i64 union arm holds different data.
    pub fn len(&self) -> i64 {
        if self.ptr.is_null() {
            return 0;
        }
        unsafe {
            let t = read_type(self.ptr);
            if t == -(RAY_STR as i8) {
                ray_str_len(self.ptr) as i64
            } else if t < 0 {
                // Other atoms have no meaningful length.
                0
            } else {
                read_len(self.ptr)
            }
        }
    }

    /// True when `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Strong reference count (0 for the NULL pointer).
    pub fn ref_count(&self) -> u32 {
        if self.ptr.is_null() {
            0
        } else {
            unsafe { read_rc(self.ptr) }
        }
    }

    /// Attribute byte (`RAY_ATTR_*` flags).
    pub fn attrs(&self) -> u8 {
        if self.ptr.is_null() {
            0
        } else {
            unsafe { read_attrs(self.ptr) }
        }
    }
}

impl Clone for RayObj {
    fn clone(&self) -> Self {
        if !self.ptr.is_null() {
            unsafe { ray_retain(self.ptr) };
        }
        Self { ptr: self.ptr }
    }
}

impl Drop for RayObj {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ray_release(self.ptr) };
        }
    }
}

// ===== Display / Debug =====
//
// rayforce 2.0 doesn't expose `obj_fmt` as a public C API, so we can't
// reproduce the C engine's pretty-printer here.  First-cut formatting
// shows the type tag and length; richer rendering is a follow-up.

impl fmt::Display for RayObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_ray_obj(self, f, false)
    }
}

impl fmt::Debug for RayObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_ray_obj(self, f, true)
    }
}

fn format_ray_obj(obj: &RayObj, f: &mut fmt::Formatter<'_>, debug: bool) -> fmt::Result {
    if obj.ptr.is_null() {
        return if debug { write!(f, "RayObj(null)") } else { write!(f, "null") };
    }
    if obj.is_nil() {
        return if debug { write!(f, "RayObj(nil)") } else { write!(f, "nil") };
    }
    let t = obj.type_code();
    if obj.is_error() {
        let code = unsafe {
            let c = ray_err_code(obj.ptr);
            if c.is_null() {
                "?".to_string()
            } else {
                CStr::from_ptr(c).to_string_lossy().into_owned()
            }
        };
        return write!(f, "RayObj(error[{code}])");
    }
    // Render atoms with their value; vectors/compounds show type+len.
    unsafe {
        match t {
            x if x == -(RAY_BOOL as i8) => write!(f, "{}", read_b8(obj.ptr) != 0),
            x if x == -(RAY_U8 as i8) => write!(f, "{}", read_u8(obj.ptr)),
            x if x == -(RAY_I16 as i8) => write!(f, "{}", read_i16(obj.ptr)),
            x if x == -(RAY_I32 as i8) => write!(f, "{}", read_i32(obj.ptr)),
            x if x == -(RAY_I64 as i8) => write!(f, "{}", read_i64(obj.ptr)),
            x if x == -(RAY_F32 as i8) || x == -(RAY_F64 as i8) => {
                write!(f, "{}", read_f64(obj.ptr))
            }
            x if x == -(RAY_SYM as i8) => {
                let id = read_i64(obj.ptr);
                let s = ray_sym_str(id);
                if s.is_null() {
                    write!(f, "`?")
                } else {
                    let label = format_atom_string(s);
                    write!(f, "`{label}")
                }
            }
            x if x == -(RAY_STR as i8) => {
                let p = ray_str_ptr(obj.ptr);
                let n = ray_str_len(obj.ptr);
                let bytes = std::slice::from_raw_parts(p as *const u8, n);
                let s = String::from_utf8_lossy(bytes);
                if debug {
                    write!(f, "{s:?}")
                } else {
                    write!(f, "{s}")
                }
            }
            _ => format_compound(obj, f, debug),
        }
    }
}

/// First-pass renderer for vectors / lists / tables / dicts.
///
/// 2.0 doesn't expose `obj_fmt`, so we walk the slice ourselves for the
/// common numeric vector types and fall back to a "type+len" summary
/// for everything else. Richer formatting is a planned follow-up.
unsafe fn format_compound(obj: &RayObj, f: &mut fmt::Formatter<'_>, _debug: bool) -> fmt::Result {
    let t = obj.type_code();
    let len = obj.len() as usize;
    let data = read_data_ptr(obj.ptr);
    match t {
        x if x == RAY_I64 as i8 => {
            let s = std::slice::from_raw_parts(data as *const i64, len);
            write!(f, "{s:?}")
        }
        x if x == RAY_I32 as i8 => {
            let s = std::slice::from_raw_parts(data as *const i32, len);
            write!(f, "{s:?}")
        }
        x if x == RAY_I16 as i8 => {
            let s = std::slice::from_raw_parts(data as *const i16, len);
            write!(f, "{s:?}")
        }
        x if x == RAY_F64 as i8 => {
            let s = std::slice::from_raw_parts(data as *const f64, len);
            write!(f, "{s:?}")
        }
        x if x == RAY_F32 as i8 => {
            let s = std::slice::from_raw_parts(data as *const f32, len);
            write!(f, "{s:?}")
        }
        x if x == RAY_U8 as i8 => {
            let s = std::slice::from_raw_parts(data, len);
            write!(f, "{s:?}")
        }
        x if x == RAY_BOOL as i8 => {
            let s = std::slice::from_raw_parts(data, len);
            let bs: Vec<bool> = s.iter().map(|&b| b != 0).collect();
            write!(f, "{bs:?}")
        }
        _ => write!(f, "RayObj(type={t}, len={len})"),
    }
}

unsafe fn format_atom_string(p: *mut ray_t) -> String {
    if p.is_null() {
        return String::new();
    }
    let t = read_type(p);
    if t == -(RAY_STR as i8) {
        let cp = ray_str_ptr(p);
        let n = ray_str_len(p);
        let bytes = std::slice::from_raw_parts(cp as *const u8, n);
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        String::new()
    }
}

// ===== Atom From<T> conversions =====

impl From<bool> for RayObj {
    fn from(val: bool) -> Self {
        unsafe { RayObj::from_raw(ray_bool(val)) }
    }
}

impl From<u8> for RayObj {
    fn from(val: u8) -> Self {
        unsafe { RayObj::from_raw(ray_u8(val)) }
    }
}

impl From<i16> for RayObj {
    fn from(val: i16) -> Self {
        unsafe { RayObj::from_raw(ray_i16(val)) }
    }
}

impl From<i32> for RayObj {
    fn from(val: i32) -> Self {
        unsafe { RayObj::from_raw(ray_i32(val)) }
    }
}

impl From<i64> for RayObj {
    fn from(val: i64) -> Self {
        unsafe { RayObj::from_raw(ray_i64(val)) }
    }
}

impl From<f32> for RayObj {
    fn from(val: f32) -> Self {
        unsafe { RayObj::from_raw(ray_f32(val)) }
    }
}

impl From<f64> for RayObj {
    fn from(val: f64) -> Self {
        unsafe { RayObj::from_raw(ray_f64(val)) }
    }
}

impl From<&str> for RayObj {
    fn from(val: &str) -> Self {
        unsafe {
            RayObj::from_raw(ray_str(val.as_ptr() as *const i8, val.len()))
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
            RayObj::from_raw(ray_vec_from_raw(
                RAY_I64 as i8,
                val.as_ptr() as *const std::ffi::c_void,
                val.len() as i64,
            ))
        }
    }
}

impl From<&[f64]> for RayObj {
    fn from(val: &[f64]) -> Self {
        unsafe {
            RayObj::from_raw(ray_vec_from_raw(
                RAY_F64 as i8,
                val.as_ptr() as *const std::ffi::c_void,
                val.len() as i64,
            ))
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

// ===== TryFrom<RayObj> for primitive Rust types =====

impl TryFrom<RayObj> for i64 {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(RAY_I64 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "I64".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(read_i64(obj.ptr)) }
    }
}

impl TryFrom<&RayObj> for i64 {
    type Error = RayforceError;

    fn try_from(obj: &RayObj) -> Result<Self> {
        if obj.type_code() != -(RAY_I64 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "I64".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(read_i64(obj.ptr)) }
    }
}

impl TryFrom<RayObj> for i32 {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(RAY_I32 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "I32".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(read_i32(obj.ptr)) }
    }
}

impl TryFrom<RayObj> for f64 {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(RAY_F64 as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "F64".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(read_f64(obj.ptr)) }
    }
}

impl TryFrom<RayObj> for bool {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(RAY_BOOL as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "Bool".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe { Ok(read_b8(obj.ptr) != 0) }
    }
}

impl TryFrom<RayObj> for String {
    type Error = RayforceError;

    fn try_from(obj: RayObj) -> Result<Self> {
        if obj.type_code() != -(RAY_STR as i8) {
            return Err(RayforceError::TypeMismatch {
                expected: "String".into(),
                actual: format!("type code {}", obj.type_code()),
            });
        }
        unsafe {
            let p = ray_str_ptr(obj.ptr);
            let n = ray_str_len(obj.ptr);
            let bytes = std::slice::from_raw_parts(p as *const u8, n);
            Ok(String::from_utf8_lossy(bytes).into_owned())
        }
    }
}

// ===== Helpers used by the higher-level wrappers =====

/// Extract the textual error code from a Rayforce error pointer.
///
/// Returns "Unknown error" for NULL or non-error pointers.
pub fn get_error_message(obj: *mut ray_t) -> String {
    unsafe {
        if !is_error(obj) {
            return "Unknown error".to_string();
        }
        let c = ray_err_code(obj);
        if c.is_null() {
            "Rayforce error".to_string()
        } else {
            CStr::from_ptr(c).to_string_lossy().into_owned()
        }
    }
}

/// Construct an empty boxed list (`RAY_LIST` with capacity 0).
pub fn new_list() -> RayObj {
    unsafe { RayObj::from_raw(ray_list_new(0)) }
}

/// Construct a typed vector pre-sized with `cap` elements of capacity.
pub fn new_vector(type_code: i8, cap: i64) -> RayObj {
    unsafe { RayObj::from_raw(ray_vec_new(type_code, cap)) }
}

/// Append an item to a list. The wrapper adopts the COW-returned new
/// list pointer, releasing the previous reference.
pub fn push_to_list(list: &mut RayObj, item: RayObj) {
    unsafe {
        let new_list = ray_list_append(list.ptr, item.ptr);
        // ray_list_append retains `item` internally; we still own our ref.
        let old = list.ptr;
        list.ptr = new_list;
        if !old.is_null() && old != new_list {
            ray_release(old);
        }
    }
}

/// 1.0-compat alias for [`list_get`].
pub fn get_at_index(list: &RayObj, idx: i64) -> Option<RayObj> {
    list_get(list, idx)
}

/// Get an item at `idx` in a list. Returns a freshly retained `RayObj`.
pub fn list_get(list: &RayObj, idx: i64) -> Option<RayObj> {
    unsafe {
        let item = ray_list_get(list.ptr, idx);
        if item.is_null() {
            None
        } else {
            ray_retain(item);
            Some(RayObj::from_raw(item))
        }
    }
}

/// Insert an item into a list at `idx`. The wrapper adopts the
/// COW-returned new list pointer.
pub fn list_insert_at(list: &mut RayObj, idx: i64, item: RayObj) {
    unsafe {
        let new_list = ray_list_insert_at(list.ptr, idx, item.ptr);
        let old = list.ptr;
        list.ptr = new_list;
        if !old.is_null() && old != new_list {
            ray_release(old);
        }
    }
}

/// Intern `s` and return a `RAY_SYM` atom wrapping the resulting ID.
pub fn new_symbol(s: &str) -> RayObj {
    unsafe {
        let id = ray_sym_intern(s.as_ptr() as *const i8, s.len());
        RayObj::from_raw(ray_sym(id))
    }
}

/// Look up the textual name of a symbol atom.
pub fn symbol_to_string(obj: &RayObj) -> Option<String> {
    if obj.type_code() != -(RAY_SYM as i8) {
        return None;
    }
    unsafe {
        let id = read_i64(obj.ptr);
        let s_obj = ray_sym_str(id);
        if s_obj.is_null() {
            return None;
        }
        let p = ray_str_ptr(s_obj);
        if p.is_null() {
            return None;
        }
        let n = ray_str_len(s_obj);
        let bytes = std::slice::from_raw_parts(p as *const u8, n);
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// Construct a `RAY_DATE` atom from days since 2000-01-01.
pub fn new_date(days: i32) -> RayObj {
    unsafe { RayObj::from_raw(ray_date(days as i64)) }
}

/// Construct a `RAY_TIME` atom from milliseconds since midnight.
pub fn new_time(ms: i32) -> RayObj {
    unsafe { RayObj::from_raw(ray_time(ms as i64)) }
}

/// Construct a `RAY_TIMESTAMP` atom from nanoseconds since epoch.
pub fn new_timestamp(ns: i64) -> RayObj {
    unsafe { RayObj::from_raw(ray_timestamp(ns)) }
}

/// Build a fresh `RAY_TABLE` from an iterator of `(name, column)` pairs.
///
/// `name` is interned; the column ray_t* is consumed (the table now
/// owns one reference to it).
pub fn new_table_from_pairs<I, S>(pairs: I) -> Result<RayObj>
where
    S: AsRef<str>,
    I: IntoIterator<Item = (S, RayObj)>,
{
    let pairs: Vec<_> = pairs.into_iter().collect();
    unsafe {
        let mut tbl = ray_table_new(pairs.len() as i64);
        if tbl.is_null() {
            return Err(RayforceError::AllocationFailed);
        }
        for (name, col) in pairs {
            let id = ray_sym_intern(name.as_ref().as_ptr() as *const i8, name.as_ref().len());
            // ray_table_add_col retains `col` internally; we still own our ref
            // through the RayObj wrapper, which will release on drop.
            let new_tbl = ray_table_add_col(tbl, id, col.as_ptr());
            if new_tbl.is_null() || is_error(new_tbl) {
                if !tbl.is_null() && tbl != new_tbl {
                    ray_release(tbl);
                }
                if is_error(new_tbl) {
                    let msg = get_error_message(new_tbl);
                    ray_error_free(new_tbl);
                    return Err(RayforceError::CApiError(msg));
                }
                return Err(RayforceError::AllocationFailed);
            }
            if tbl != new_tbl {
                ray_release(tbl);
            }
            tbl = new_tbl;
        }
        Ok(RayObj::from_raw(tbl))
    }
}

/// Build a fresh `RAY_DICT` from `keys` and `vals` ray objects.
///
/// `ray_dict_new` consumes both inputs on success or error (it adopts
/// the strong references), so we forget the wrappers.
pub fn new_dict(keys: RayObj, vals: RayObj) -> Result<RayObj> {
    unsafe {
        let d = ray_dict_new(keys.ptr, vals.ptr);
        std::mem::forget(keys);
        std::mem::forget(vals);
        if d.is_null() {
            return Err(RayforceError::AllocationFailed);
        }
        if is_error(d) {
            let msg = get_error_message(d);
            ray_error_free(d);
            return Err(RayforceError::CApiError(msg));
        }
        Ok(RayObj::from_raw(d))
    }
}

/// Bind a global value under `name`, interning the symbol on the way in.
pub fn set_global(name: &str, value: &RayObj) -> Result<()> {
    unsafe {
        let id = ray_sym_intern(name.as_ptr() as *const i8, name.len());
        let rc = ray_env_set(id, value.ptr);
        if rc != ray_err_t_RAY_OK {
            return Err(RayforceError::from(rc));
        }
        Ok(())
    }
}

/// Return the raw element-data pointer of a vector (slice-aware).
pub fn get_obj_raw_ptr(obj: &RayObj) -> *mut u8 {
    unsafe { read_data_ptr(obj.ptr) }
}

/// Length helper for use from outside the crate.
pub fn get_obj_len(obj: &RayObj) -> i64 {
    obj.len()
}
