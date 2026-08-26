//! Dictionaries (`RAY_DICT`) — a `[keys, values]` pair of parallel vectors/lists.

use crate::error::{check, RayError, Result};
use crate::raw;
use crate::runtime::assert_live;
use crate::value::Value;
use rayforce_sys as sys;

impl Value {
    /// Build a dict from a keys vector/list and a values vector/list.
    ///
    /// Consumes both arguments (the core takes ownership of each).
    pub fn dict(keys: Value, values: Value) -> Value {
        assert_live("Value::dict");
        unsafe {
            let d = sys::ray_dict_new(keys.into_raw(), values.into_raw());
            match check(d) {
                Ok(p) => Value::from_owned(p),
                Err(e) => panic!("rayforce: failed to build dict: {e}"),
            }
        }
    }

    /// True if this value is a dictionary.
    pub fn is_dict(&self) -> bool {
        self.type_code() == sys::RAY_DICT as i8
    }

    /// The keys collection (borrowed view).
    pub fn dict_keys(&self) -> Result<Value> {
        if !self.is_dict() {
            return Err(RayError::binding("dict_keys: value is not a dict"));
        }
        unsafe { Ok(Value::from_borrowed(sys::ray_dict_keys(self.as_ptr()))) }
    }

    /// The values collection (borrowed view).
    pub fn dict_values(&self) -> Result<Value> {
        if !self.is_dict() {
            return Err(RayError::binding("dict_values: value is not a dict"));
        }
        unsafe { Ok(Value::from_borrowed(sys::ray_dict_vals(self.as_ptr()))) }
    }

    /// Number of key/value pairs.
    pub fn dict_len(&self) -> Result<usize> {
        if !self.is_dict() {
            return Err(RayError::binding("dict_len: value is not a dict"));
        }
        Ok(unsafe { sys::ray_dict_len(self.as_ptr()).max(0) as usize })
    }

    /// Look up a key, returning the associated value if present.
    pub fn dict_get(&self, key: &Value) -> Result<Option<Value>> {
        if !self.is_dict() {
            return Err(RayError::binding("dict_get: value is not a dict"));
        }
        unsafe {
            let r = sys::ray_dict_get(self.as_ptr(), key.as_ptr());
            if r.is_null() {
                return Ok(None);
            }
            if raw::is_err(r) {
                return Err(RayError::from_obj(r));
            }
            Ok(Some(Value::from_owned(r)))
        }
    }
}
