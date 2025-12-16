//! Container types for Rayforce.

use crate::error::{RayforceError, Result};
use crate::ffi::{self, RayObj};
use crate::types::{RayType, RaySymbol};
use crate::*;
use std::fmt;
use std::marker::PhantomData;

/// A generic list that can hold any Rayforce objects.
#[derive(Clone)]
pub struct RayList {
    ptr: RayObj,
}

impl RayList {
    /// Create a new empty list.
    pub fn new() -> Self {
        Self {
            ptr: ffi::new_list(),
        }
    }

    /// Create a list from an iterator of items that can be converted to RayObj.
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

    /// Get the length of the list.
    pub fn len(&self) -> usize {
        self.ptr.len() as usize
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Push an item to the list.
    pub fn push<T: Into<RayObj>>(&mut self, item: T) {
        ffi::push_to_list(&mut self.ptr, item.into());
    }

    /// Get an item at an index.
    pub fn get(&self, idx: usize) -> Option<RayObj> {
        if idx >= self.len() {
            None
        } else {
            ffi::get_at_index(&self.ptr, idx as i64)
        }
    }

    /// Set an item at an index.
    pub fn set<T: Into<RayObj>>(&mut self, idx: usize, item: T) {
        if idx < self.len() {
            ffi::insert_at_index(&mut self.ptr, idx as i64, item.into());
        }
    }

    /// Iterate over items as RayObj.
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
    const TYPE_CODE: i8 = TYPE_LIST as i8;
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

/// Type alias for backward compatibility.
pub type List = RayList;

/// A homogeneous vector of elements.
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
    /// Get the length of the vector.
    pub fn len(&self) -> usize {
        self.ptr.len() as usize
    }

    /// Check if the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the underlying RayObj.
    pub fn as_ray_obj(&self) -> &RayObj {
        &self.ptr
    }

    /// Get the type code of elements.
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

// RayVector of i64
impl RayVector<i64> {
    /// Create a new i64 vector.
    pub fn new(len: usize) -> Self {
        unsafe {
            Self {
                ptr: RayObj::from_raw(vector(TYPE_I64 as i8, len as i64)),
                _marker: PhantomData,
            }
        }
    }

    /// Create from a slice.
    pub fn from_slice(data: &[i64]) -> Self {
        Self {
            ptr: RayObj::from(data),
            _marker: PhantomData,
        }
    }

    /// Create from an iterator.
    pub fn from_iter<I: IntoIterator<Item = i64>>(iter: I) -> Self {
        let data: Vec<i64> = iter.into_iter().collect();
        Self::from_slice(&data)
    }

    /// Get the data as a slice.
    pub fn as_slice(&self) -> &[i64] {
        unsafe {
            let len = ffi::get_obj_len(&self.ptr) as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *const i64;
            std::slice::from_raw_parts(raw, len)
        }
    }

    /// Get the data as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [i64] {
        unsafe {
            let len = ffi::get_obj_len(&self.ptr) as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *mut i64;
            std::slice::from_raw_parts_mut(raw, len)
        }
    }

    /// Get an element.
    pub fn get(&self, idx: usize) -> Option<i64> {
        if idx >= self.len() {
            None
        } else {
            Some(self.as_slice()[idx])
        }
    }

    /// Set an element.
    pub fn set(&mut self, idx: usize, value: i64) {
        if idx < self.len() {
            self.as_mut_slice()[idx] = value;
        }
    }
}

impl RayType for RayVector<i64> {
    const TYPE_CODE: i8 = TYPE_I64 as i8;
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

// RayVector of f64
impl RayVector<f64> {
    /// Create a new f64 vector.
    pub fn new(len: usize) -> Self {
        unsafe {
            Self {
                ptr: RayObj::from_raw(vector(TYPE_F64 as i8, len as i64)),
                _marker: PhantomData,
            }
        }
    }

    /// Create from a slice.
    pub fn from_slice(data: &[f64]) -> Self {
        Self {
            ptr: RayObj::from(data),
            _marker: PhantomData,
        }
    }

    /// Create from an iterator.
    pub fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        let data: Vec<f64> = iter.into_iter().collect();
        Self::from_slice(&data)
    }

    /// Get the data as a slice.
    pub fn as_slice(&self) -> &[f64] {
        unsafe {
            let len = ffi::get_obj_len(&self.ptr) as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *const f64;
            std::slice::from_raw_parts(raw, len)
        }
    }

    /// Get the data as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        unsafe {
            let len = ffi::get_obj_len(&self.ptr) as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *mut f64;
            std::slice::from_raw_parts_mut(raw, len)
        }
    }

    /// Get an element.
    pub fn get(&self, idx: usize) -> Option<f64> {
        if idx >= self.len() {
            None
        } else {
            Some(self.as_slice()[idx])
        }
    }

    /// Set an element.
    pub fn set(&mut self, idx: usize, value: f64) {
        if idx < self.len() {
            self.as_mut_slice()[idx] = value;
        }
    }
}

impl RayType for RayVector<f64> {
    const TYPE_CODE: i8 = TYPE_F64 as i8;
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

// RayVector of RaySymbol
impl RayVector<RaySymbol> {
    /// Create a new symbol vector.
    pub fn new(len: usize) -> Self {
        unsafe {
            Self {
                ptr: RayObj::from_raw(vector(TYPE_SYMBOL as i8, len as i64)),
                _marker: PhantomData,
            }
        }
    }

    /// Create from an iterator of strings.
    pub fn from_iter<S, I>(iter: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        let items: Vec<_> = iter.into_iter().collect();
        unsafe {
            let obj = vector(TYPE_SYMBOL as i8, items.len() as i64);
            let dst = ffi::get_obj_raw_ptr(&RayObj::from_raw(clone_obj(obj))) as *mut i64;
            for (i, s) in items.iter().enumerate() {
                // Intern the symbol and get its ID
                let sym = ffi::new_symbol(s.as_ref());
                let id = *(*sym.as_ptr()).__bindgen_anon_1.i64_.as_ref();
                *dst.add(i) = id;
            }
            Self {
                ptr: RayObj::from_raw(obj),
                _marker: PhantomData,
            }
        }
    }

    /// Get a symbol at an index.
    pub fn get(&self, idx: usize) -> Option<String> {
        if idx >= self.len() {
            return None;
        }
        unsafe {
            let raw = ffi::get_obj_raw_ptr(&self.ptr) as *const i64;
            let id = *raw.add(idx);
            let cstr = str_from_symbol(id);
            if cstr.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(cstr).to_string_lossy().into_owned())
            }
        }
    }
}

impl RayType for RayVector<RaySymbol> {
    const TYPE_CODE: i8 = TYPE_SYMBOL as i8;
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

/// Type alias for backward compatibility.
pub type Vector<T> = RayVector<T>;

/// String type (vector of characters).
#[derive(Clone)]
pub struct RayString {
    ptr: RayObj,
}

impl RayString {
    /// Create a new string.
    pub fn new(s: &str) -> Self {
        Self {
            ptr: RayObj::from(s),
        }
    }

    /// Get the string value.
    pub fn to_string(&self) -> String {
        unsafe {
            let len = ffi::get_obj_len(&self.ptr) as usize;
            let raw = ffi::get_obj_raw_ptr(&self.ptr);
            let bytes = std::slice::from_raw_parts(raw, len);
            String::from_utf8_lossy(bytes).into_owned()
        }
    }

    /// Get the length.
    pub fn len(&self) -> usize {
        self.ptr.len() as usize
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RayType for RayString {
    const TYPE_CODE: i8 = TYPE_C8 as i8;
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

/// Dictionary type (key-value mapping).
#[derive(Clone)]
pub struct RayDict {
    ptr: RayObj,
}

impl RayDict {
    /// Create a new dictionary from keys and values.
    pub fn new(keys: RayObj, values: RayObj) -> Result<Self> {
        let ptr = ffi::new_dict(keys, values)?;
        Ok(Self { ptr })
    }

    /// Create a dictionary from symbol keys and values.
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
        
        unsafe {
            let d = dict(keys.ptr.as_ptr(), values.ptr.as_ptr());
            if d.is_null() {
                return Err(RayforceError::AllocationFailed);
            }
            std::mem::forget(keys);
            std::mem::forget(values);
            Ok(Self {
                ptr: RayObj::from_raw(d),
            })
        }
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<RayObj> {
        let key_sym = ffi::new_symbol(key);
        unsafe {
            let val = at_obj(self.ptr.as_ptr(), key_sym.as_ptr());
            if val.is_null() {
                None
            } else {
                Some(RayObj::from_raw(clone_obj(val)))
            }
        }
    }

    /// Get the keys.
    pub fn keys(&self) -> RayObj {
        unsafe {
            // Dict is structured as [keys, values] - get first element
            let keys = at_idx(self.ptr.as_ptr(), 0);
            if keys.is_null() {
                ffi::new_list()
            } else {
                RayObj::from_raw(clone_obj(keys))
            }
        }
    }

    /// Get the values.
    pub fn values(&self) -> RayObj {
        unsafe {
            // Dict is structured as [keys, values] - get second element
            let values = at_idx(self.ptr.as_ptr(), 1);
            if values.is_null() {
                ffi::new_list()
            } else {
                RayObj::from_raw(clone_obj(values))
            }
        }
    }

    /// Get the number of key-value pairs.
    pub fn len(&self) -> usize {
        unsafe {
            let keys = at_idx(self.ptr.as_ptr(), 0);
            if keys.is_null() {
                0
            } else {
                let keys_obj = RayObj::from_raw(clone_obj(keys));
                ffi::get_obj_len(&keys_obj) as usize
            }
        }
    }

    /// Check if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RayType for RayDict {
    const TYPE_CODE: i8 = TYPE_DICT as i8;
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

/// Type alias for backward compatibility.
pub type Dict = RayDict;
