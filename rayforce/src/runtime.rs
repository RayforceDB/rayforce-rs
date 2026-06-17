//! Runtime lifecycle and evaluation entry points.
//!
//! The core permits exactly one live runtime per process and pins it to the
//! creating thread (thread-local VM). [`Runtime`] is an RAII guard enforcing the
//! single-live-instance rule; hold one for as long as you use the API. Drop it
//! to tear the runtime down.

use crate::error::{check, materialize, RayError, Result};
use crate::value::Value;
use rayforce_sys as sys;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

static LIVE: AtomicBool = AtomicBool::new(false);

/// An owned, live RayforceDB runtime. Only one may exist per process at a time.
///
/// All evaluation and object construction must happen on the thread that created
/// the runtime; `Runtime` is `!Send`/`!Sync` to enforce this.
pub struct Runtime {
    rt: *mut sys::ray_runtime_s,
    _not_send: PhantomData<*mut ()>,
}

impl Runtime {
    /// Create the process runtime. Returns an error if one is already live.
    pub fn new() -> Result<Runtime> {
        if LIVE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(RayError::binding(
                "a rayforce runtime is already live in this process",
            ));
        }
        let rt = unsafe { sys::ray_runtime_create(0, std::ptr::null_mut()) };
        if rt.is_null() {
            LIVE.store(false, Ordering::SeqCst);
            return Err(RayError::binding("ray_runtime_create failed"));
        }
        Ok(Runtime {
            rt,
            _not_send: PhantomData,
        })
    }

    /// Evaluate a Rayfall source string against the global environment.
    pub fn eval(&self, source: &str) -> Result<Value> {
        eval(source)
    }

    /// Bind `value` to the global name `name` (the core retains it). The value
    /// then resolves in evaluated expressions and queries.
    pub fn set_global(&self, name: &str, value: &Value) -> Result<()> {
        set_global(name, value)
    }

    /// Look up a global binding by name.
    pub fn get_global(&self, name: &str) -> Result<Value> {
        get_global(name)
    }
}

/// Bind `value` to a global name. Requires a live [`Runtime`].
pub fn set_global(name: &str, value: &Value) -> Result<()> {
    debug_assert!(is_live(), "set_global called without a live Runtime");
    unsafe {
        let id = sys::ray_sym_intern(name.as_ptr() as *const _, name.len());
        let e = sys::ray_env_set(id, value.as_ptr());
        if e != sys::ray_err_t_RAY_OK {
            return Err(RayError::binding(format!(
                "set_global({name}) failed (err {e})"
            )));
        }
    }
    Ok(())
}

/// Look up a global binding. Requires a live [`Runtime`].
pub fn get_global(name: &str) -> Result<Value> {
    debug_assert!(is_live(), "get_global called without a live Runtime");
    unsafe {
        let id = sys::ray_sym_intern(name.as_ptr() as *const _, name.len());
        let v = sys::ray_env_get(id);
        if v.is_null() {
            return Err(RayError::binding(format!("global not found: {name}")));
        }
        if crate::raw::is_err(v) {
            return Err(RayError::from_obj(v));
        }
        Ok(Value::from_borrowed(v))
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { sys::ray_runtime_destroy(self.rt) };
        LIVE.store(false, Ordering::SeqCst);
    }
}

/// Is a runtime currently live in this process?
pub fn is_live() -> bool {
    LIVE.load(Ordering::SeqCst)
}

/// Evaluate a Rayfall source string. Requires a live [`Runtime`].
///
/// A void / null result becomes [`Value::null`]; a core error becomes `Err`.
pub fn eval(source: &str) -> Result<Value> {
    debug_assert!(is_live(), "eval called without a live Runtime");
    let c = CString::new(source).map_err(|_| RayError::binding("source contains a NUL byte"))?;
    unsafe {
        let r = sys::ray_eval_str(c.as_ptr());
        if r.is_null() {
            return Ok(Value::null());
        }
        Ok(Value::from_owned(materialize(check(r)?)?))
    }
}

/// Evaluate an already-compiled AST [`Value`] (e.g. a query). Requires a live
/// [`Runtime`].
pub fn eval_value(obj: &Value) -> Result<Value> {
    debug_assert!(is_live(), "eval_value called without a live Runtime");
    unsafe {
        let r = sys::ray_eval(obj.as_ptr());
        if r.is_null() {
            return Ok(Value::null());
        }
        Ok(Value::from_owned(materialize(check(r)?)?))
    }
}
