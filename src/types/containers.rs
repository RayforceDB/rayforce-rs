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

//! Container types for Rayforce 2.0.

use crate::error::{RayforceError, Result};
use crate::ffi::{self, RayObj};
use crate::types::{RayType, RaySymbol};
use crate::*;
use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// RayList — heterogeneous boxed list (`RAY_LIST`).
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RayList {
    ptr: RayObj,
}

impl RayList {
    pub fn new() -> Self {
        Self { ptr: ffi::new_list() }
    }

    pub fn from_iter<T, I>(items: I) -> Self
    where
        T: Into<RayObj>,
        I: IntoIterator<Item = T>,
    {
        let mut list = Self::new();
        for item in items {
            list.push(item);
        }
        list
    }

    pub fn len(&self) -> usize {
        self.ptr.len() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push<T: Into<RayObj>>(&mut self, item: T) {
        ffi::push_to_list(&mut self.ptr, item.into());
    }

    pub fn get(&self, idx: usize) -> Option<RayObj> {
        if idx >= self.len() {
            None
        } else {
            ffi::list_get(&self.ptr, idx as i64)
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = RayObj> + '_ {
        (0..self.len()).filter_map(move |i| self.get(i))
    }
}

impl Default for RayList {
    fn default() -> Self {
        Self::new()
    }
}

impl RayType for RayList {
    const TYPE_CODE: i8 = RAY_LIST as i8;
    const RAY_NAME: &'static str = "RayList";

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

impl fmt::Debug for RayList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayList[{}]", self.len())
    }
}

impl fmt::Display for RayList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ptr)
    }
}

impl<T: Into<RayObj>> FromIterator<T> for RayList {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        RayList::from_iter(iter)
    }
}

pub type List = RayList;

// ---------------------------------------------------------------------------
// RayVector<T> — homogeneous typed vectors.
// ---------------------------------------------------------------------------

pub struct RayVector<T> {
    ptr: RayObj,
    _marker: PhantomData<T>,
}

impl<T> Clone for RayVector<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> RayVector<T> {
    pub fn len(&self) -> usize {
        self.ptr.len() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ray_obj(&self) -> &RayObj {
        &self.ptr
    }

    /// Type tag of the vector itself (not the element atom).
    pub fn element_type_code(&self) -> i8 {
        self.ptr.type_code()
    }
}

impl<T> fmt::Debug for RayVector<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayVector<{}>[{}]", std::any::type_name::<T>(), self.len())
    }
}

impl<T> fmt::Display for RayVector<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ptr)
    }
}

// ---- RayVector<i64> ----

impl RayVector<i64> {
    pub fn new(capacity: usize) -> Self {
        unsafe {
            Self {
                ptr: RayObj::from_raw(ray_vec_new(RAY_I64 as i8, capacity as i64)),
                _marker: PhantomData,
            }
        }
    }

    pub fn from_slice(data: &[i64]) -> Self {
        Self {
            ptr: RayObj::from(data),
            _marker: PhantomData,
        }
    }

    pub fn from_iter<I: IntoIterator<Item = i64>>(iter: I) -> Self {
        let data: Vec<i64> = iter.into_iter().collect();
        Self::from_slice(&data)
    }

    pub fn as_slice(&self) -> &[i64] {
        unsafe {
            let len = self.ptr.len() as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *const i64;
            std::slice::from_raw_parts(raw, len)
        }
    }

    pub fn get(&self, idx: usize) -> Option<i64> {
        if idx >= self.len() {
            None
        } else {
            Some(self.as_slice()[idx])
        }
    }

    /// Overwrite element `idx` in place.
    ///
    /// Walks `ray_vec_set` and adopts the COW-returned vector.
    pub fn set(&mut self, idx: usize, value: i64) {
        if idx >= self.len() {
            return;
        }
        unsafe {
            let new_ptr = ray_vec_set(
                self.ptr.as_ptr(),
                idx as i64,
                &value as *const i64 as *const std::ffi::c_void,
            );
            replace_ray_obj(&mut self.ptr, new_ptr);
        }
    }
}

impl RayType for RayVector<i64> {
    const TYPE_CODE: i8 = RAY_I64 as i8;
    const RAY_NAME: &'static str = "RayVector<i64>";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr, _marker: PhantomData })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl FromIterator<i64> for RayVector<i64> {
    fn from_iter<I: IntoIterator<Item = i64>>(iter: I) -> Self {
        RayVector::<i64>::from_slice(&iter.into_iter().collect::<Vec<_>>())
    }
}

// ---- RayVector<f64> ----

impl RayVector<f64> {
    pub fn new(capacity: usize) -> Self {
        unsafe {
            Self {
                ptr: RayObj::from_raw(ray_vec_new(RAY_F64 as i8, capacity as i64)),
                _marker: PhantomData,
            }
        }
    }

    pub fn from_slice(data: &[f64]) -> Self {
        Self {
            ptr: RayObj::from(data),
            _marker: PhantomData,
        }
    }

    pub fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        let data: Vec<f64> = iter.into_iter().collect();
        Self::from_slice(&data)
    }

    pub fn as_slice(&self) -> &[f64] {
        unsafe {
            let len = self.ptr.len() as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *const f64;
            std::slice::from_raw_parts(raw, len)
        }
    }

    pub fn get(&self, idx: usize) -> Option<f64> {
        if idx >= self.len() {
            None
        } else {
            Some(self.as_slice()[idx])
        }
    }

    pub fn set(&mut self, idx: usize, value: f64) {
        if idx >= self.len() {
            return;
        }
        unsafe {
            let new_ptr = ray_vec_set(
                self.ptr.as_ptr(),
                idx as i64,
                &value as *const f64 as *const std::ffi::c_void,
            );
            replace_ray_obj(&mut self.ptr, new_ptr);
        }
    }
}

impl RayType for RayVector<f64> {
    const TYPE_CODE: i8 = RAY_F64 as i8;
    const RAY_NAME: &'static str = "RayVector<f64>";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr, _marker: PhantomData })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl FromIterator<f64> for RayVector<f64> {
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        RayVector::<f64>::from_slice(&iter.into_iter().collect::<Vec<_>>())
    }
}

// ---- RayVector<RaySymbol> ----
//
// A `RAY_SYM` column stores an int64 symbol ID per element (W64 width).
// Symbol IDs come from `ray_sym_intern`. We always pick W64 so the
// vector can grow without rewriting existing entries.

impl RayVector<RaySymbol> {
    pub fn new(capacity: usize) -> Self {
        unsafe {
            Self {
                ptr: RayObj::from_raw(ray_sym_vec_new(RAY_SYM_W64 as u8, capacity as i64)),
                _marker: PhantomData,
            }
        }
    }

    pub fn from_iter<S, I>(iter: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        let items: Vec<_> = iter.into_iter().collect();
        unsafe {
            let mut v = ray_sym_vec_new(RAY_SYM_W64 as u8, items.len() as i64);
            for s in &items {
                let s = s.as_ref();
                let id = ray_sym_intern(s.as_ptr() as *const i8, s.len());
                let new_v = ray_vec_append(v, &id as *const i64 as *const std::ffi::c_void);
                if new_v != v && !v.is_null() {
                    ray_release(v);
                }
                v = new_v;
            }
            Self {
                ptr: RayObj::from_raw(v),
                _marker: PhantomData,
            }
        }
    }

    /// Resolve the symbol at `idx` back to its interned name.
    pub fn get(&self, idx: usize) -> Option<String> {
        if idx >= self.len() {
            return None;
        }
        unsafe {
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *const i64;
            let id = *raw.add(idx);
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
}

impl RayType for RayVector<RaySymbol> {
    const TYPE_CODE: i8 = RAY_SYM as i8;
    const RAY_NAME: &'static str = "RayVector<RaySymbol>";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != Self::TYPE_CODE {
            return Err(RayforceError::TypeMismatch {
                expected: Self::RAY_NAME.into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr, _marker: PhantomData })
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

pub type Vector<T> = RayVector<T>;

// ---------------------------------------------------------------------------
// RayString — single string atom (`-RAY_STR`).
// ---------------------------------------------------------------------------
//
// 2.0 changed strings from "C8 vector of chars" to a dedicated atom
// type with SSO (≤ 7 bytes inline, longer values in a pool).

#[derive(Clone)]
pub struct RayString {
    ptr: RayObj,
}

impl RayString {
    pub fn new(s: &str) -> Self {
        Self { ptr: RayObj::from(s) }
    }

    pub fn to_string(&self) -> String {
        unsafe {
            let p = ray_str_ptr(self.ptr.as_ptr());
            if p.is_null() {
                return String::new();
            }
            let n = ray_str_len(self.ptr.as_ptr());
            let bytes = std::slice::from_raw_parts(p as *const u8, n);
            String::from_utf8_lossy(bytes).into_owned()
        }
    }

    pub fn len(&self) -> usize {
        unsafe { ray_str_len(self.ptr.as_ptr()) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RayType for RayString {
    const TYPE_CODE: i8 = -(RAY_STR as i8);
    const RAY_NAME: &'static str = "RayString";

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

impl fmt::Debug for RayString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayString({:?})", self.to_string())
    }
}

impl fmt::Display for RayString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<&str> for RayString {
    fn from(s: &str) -> Self {
        RayString::new(s)
    }
}

impl From<String> for RayString {
    fn from(s: String) -> Self {
        RayString::new(&s)
    }
}

// ---------------------------------------------------------------------------
// RayDict — `RAY_DICT`.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RayDict {
    ptr: RayObj,
}

impl RayDict {
    /// Wrap a freshly built dict from `keys` and `vals`. Both are
    /// consumed by `ray_dict_new`.
    pub fn new(keys: RayObj, values: RayObj) -> Result<Self> {
        let ptr = ffi::new_dict(keys, values)?;
        Ok(Self { ptr })
    }

    /// Build a dict from `(name, value)` pairs where each name is interned
    /// to a symbol.
    pub fn from_pairs<K, V, I>(pairs: I) -> Result<Self>
    where
        K: AsRef<str>,
        V: Into<RayObj>,
        I: IntoIterator<Item = (K, V)>,
    {
        let items: Vec<_> = pairs.into_iter().collect();
        let keys = RayVector::<RaySymbol>::from_iter(items.iter().map(|(k, _)| k.as_ref()));
        let mut values = RayList::new();
        for (_, v) in items {
            values.push(v);
        }
        // Hand off ownership of the keys vector and values list.
        Self::new(keys.ptr.clone(), values.ptr().clone())
    }

    /// Return the underlying value for `key` (interned as a symbol).
    /// `ray_dict_get` returns an owned reference on hit, NULL on miss.
    pub fn get(&self, key: &str) -> Option<RayObj> {
        let key_sym = ffi::new_symbol(key);
        unsafe {
            let val = ray_dict_get(self.ptr.as_ptr(), key_sym.as_ptr());
            if val.is_null() {
                None
            } else {
                Some(RayObj::from_raw(val))
            }
        }
    }

    /// Borrowed reference to the dict's keys vector.
    pub fn keys(&self) -> RayObj {
        unsafe {
            let k = ray_dict_keys(self.ptr.as_ptr());
            if k.is_null() {
                ffi::new_list()
            } else {
                ray_retain(k);
                RayObj::from_raw(k)
            }
        }
    }

    /// Borrowed reference to the dict's values vector.
    pub fn values(&self) -> RayObj {
        unsafe {
            let v = ray_dict_vals(self.ptr.as_ptr());
            if v.is_null() {
                ffi::new_list()
            } else {
                ray_retain(v);
                RayObj::from_raw(v)
            }
        }
    }

    pub fn len(&self) -> usize {
        unsafe { ray_dict_len(self.ptr.as_ptr()) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RayType for RayDict {
    const TYPE_CODE: i8 = RAY_DICT as i8;
    const RAY_NAME: &'static str = "RayDict";

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

impl fmt::Debug for RayDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RayDict[{}]", self.len())
    }
}

impl fmt::Display for RayDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ptr)
    }
}

pub type Dict = RayDict;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Replace the `ray_t*` inside a `RayObj` with `new_ptr`, releasing the
/// previous pointer if the engine returned a freshly allocated COW
/// successor.  Used by mutation paths that go through `ray_vec_*` /
/// `ray_list_*` and adopt the returned pointer.
unsafe fn replace_ray_obj(slot: &mut RayObj, new_ptr: *mut ray_t) {
    let old = slot.as_ptr();
    if !old.is_null() && old != new_ptr {
        ray_release(old);
    }
    *slot = RayObj::from_raw(new_ptr);
}

// Silence unused warnings for items only referenced by tests.
#[allow(dead_code)]
fn _unused_cstr(_: &CStr) {}
