//! Heterogeneous lists (`RAY_LIST`) — ordered sequences of boxed [`Value`]s.
//!
//! Element access is via [`Value::get`] / [`Value::iter`] (shared with vectors).
//! `ray_list_append` retains the appended item, so the source `Value` keeps its
//! own reference.

use crate::error::{check, RayError, Result};
use crate::raw::{self, Raw};
use crate::runtime::assert_on_runtime_thread;
use crate::value::Value;
use rayforce_sys as sys;

impl Value {
    /// Build a list from boxed values (each is retained by the list).
    pub fn list(items: &[Value]) -> Value {
        assert_on_runtime_thread("Value::list");
        unsafe {
            let mut l = match check(sys::ray_list_new(items.len() as i64)) {
                Ok(p) => p,
                Err(e) => panic!("rayforce: failed to allocate list: {e}"),
            };
            for item in items {
                let r = sys::ray_list_append(l, item.as_ptr());
                l = match check(r) {
                    Ok(p) => p,
                    Err(e) => panic!("rayforce: failed to append to list: {e}"),
                };
            }
            Value::from_owned(l)
        }
    }

    /// An empty list with the given capacity.
    pub fn empty_list(capacity: i64) -> Value {
        assert_on_runtime_thread("Value::empty_list");
        unsafe {
            match check(sys::ray_list_new(capacity)) {
                Ok(p) => Value::from_owned(p),
                Err(e) => panic!("rayforce: failed to allocate list: {e}"),
            }
        }
    }

    /// True if this value is a heterogeneous list.
    pub fn is_list(&self) -> bool {
        self.type_code() == sys::RAY_LIST as i8
    }

    /// Append a boxed value (the list retains it). Copy-on-write move semantics.
    pub fn list_push(&mut self, item: &Value) -> Result<()> {
        if !self.is_list() {
            return Err(RayError::binding("list_push: value is not a list"));
        }
        unsafe {
            let r: Raw = sys::ray_list_append(self.as_ptr(), item.as_ptr());
            if raw::is_err(r) {
                let e = RayError::from_obj(r);
                self.replace_ptr_consumed(raw::null_obj());
                return Err(RayError::binding(format!("list_push: {e}")));
            }
            self.replace_ptr_consumed(r);
            Ok(())
        }
    }
}
