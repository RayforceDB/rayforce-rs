//! Runtime lifecycle and evaluation entry points.
//!
//! The core permits exactly one live runtime per process and pins it to the
//! creating thread (thread-local VM). [`Runtime::scope`] is the only way to get
//! one: it creates the runtime, hands your closure a `&Runtime`, and tears it
//! down when the closure returns.
//!
//! That shape is what keeps the crate sound. `ray_runtime_destroy` unmaps the
//! engine heap without consulting any object's reference count, so a [`Value`]
//! that outlived its runtime would point into unmapped address space. Inside a
//! scope it cannot: the caller never owns the guard, so cannot drop it early,
//! and the bounds on [`Runtime::scope`] stop values leaving the closure.

use crate::error::{check, materialize, RayError, Result};
use crate::value::Value;
use rayforce_sys as sys;
use std::ffi::CString;
use std::marker::PhantomData;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

/// Is a runtime live in this process? The core permits exactly one.
static LIVE: AtomicBool = AtomicBool::new(false);

/// A live RayforceDB runtime. Obtained only from [`Runtime::scope`], and only as
/// a shared reference — you cannot own one, so you cannot drop one.
///
/// All evaluation and object construction must happen on the thread that created
/// the runtime; `Runtime` is `!Send`/`!Sync` to enforce this.
///
/// # Safety
///
/// `!Send`/`!Sync`, and must stay so: the core's VM is thread-local, so a guard
/// that reached another thread would evaluate against a VM that is not there.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<rayforce::Runtime>();
/// ```
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<rayforce::Runtime>();
/// ```
/// Control — `compile_fail` passes on *any* build failure, a rename included:
/// ```
/// fn assert_exists<T>() {}
/// assert_exists::<rayforce::Runtime>();
/// ```
pub struct Runtime {
    rt: *mut sys::ray_runtime_s,
    _not_send: PhantomData<*mut ()>,
}

impl Runtime {
    /// Create the process runtime, run `f` against it, and tear it down.
    ///
    /// ```
    /// # use rayforce::Runtime;
    /// let sum = Runtime::scope(|rt| Ok(rt.eval("(+ 1 1)")?.as_i64()?))?;
    /// assert_eq!(sum, 2);
    /// # Ok::<(), rayforce::RayError>(())
    /// ```
    ///
    /// The runtime is torn down when `f` returns, on the error path and on
    /// unwind alike. Scopes may run back to back, but never nested — the core
    /// permits one live runtime at a time.
    ///
    /// # Why the `Send` bounds
    ///
    /// Nothing engine-backed may leave the closure: the heap it points into is
    /// unmapped on the way out. Every such type — [`Value`], [`crate::Table`],
    /// [`crate::Fn`], [`crate::TcpClient`], [`crate::QConnection`] — is already
    /// `!Send` and `!Sync` because the core's VM is thread-local, so requiring
    /// `Send` of the return type and of the closure rejects exactly them.
    ///
    /// `R: Send` closes the return path, transitively and through references
    /// (`&T: Send` needs `T: Sync`):
    ///
    /// ```compile_fail
    /// # use rayforce::Runtime;
    /// let v = Runtime::scope(|rt| rt.eval("(+ 1 1)")).unwrap();
    /// ```
    /// ```compile_fail
    /// # use rayforce::{Runtime, Value};
    /// let v = Runtime::scope(|rt| Ok(vec![rt.eval("1")?])).unwrap();
    /// ```
    ///
    /// `F: Send` closes the capture path, since a closure is `Send` only if
    /// every capture is:
    ///
    /// ```compile_fail
    /// # use rayforce::{Runtime, Value};
    /// let mut out = None;
    /// Runtime::scope(|rt| { out = Some(rt.eval("1")?); Ok(()) }).unwrap();
    /// ```
    ///
    /// Control — the same shape, extracting plain data instead, must compile:
    ///
    /// ```
    /// # use rayforce::Runtime;
    /// let mut out = None;
    /// Runtime::scope(|rt| { out = Some(rt.eval("(+ 1 1)")?.format()); Ok(()) })?;
    /// assert_eq!(out.unwrap(), "2");
    /// # Ok::<(), rayforce::RayError>(())
    /// ```
    ///
    /// The cost is that an unrelated `!Send` capture — an `Rc`, a `RefCell`
    /// borrow — is refused too, with a diagnostic about threads when no thread
    /// is involved. Move such values into the closure, or construct them inside.
    ///
    /// # The one escape these bounds miss
    ///
    /// A closure that stashes a value into a `thread_local!` captures nothing,
    /// so it satisfies `F: Send`, and the assignment compiles. Nothing catches
    /// it afterwards either: the value's `Drop` runs at thread exit, against a
    /// heap that was unmapped when the scope ended. A plain `static` cannot do
    /// this — `Value` is `!Sync` — and the bounds cover every other route, so
    /// this is the single remaining way to build a dangling handle from safe
    /// code. It takes deliberate effort; don't.
    pub fn scope<F, R>(f: F) -> Result<R>
    where
        F: Send + FnOnce(&Runtime) -> Result<R>,
        R: Send,
    {
        let rt = Runtime::new()?;
        f(&rt)
    }

    /// Create the process runtime. Private: [`Runtime::scope`] is the entry
    /// point, and it being the only one is what bounds a `Runtime`'s life.
    fn new() -> Result<Runtime> {
        if LIVE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(RayError::binding(
                "a rayforce runtime is already live in this process — \
                 Runtime::scope cannot be nested",
            ));
        }
        let rt = unsafe { sys::ray_runtime_create(0, ptr::null_mut()) };
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

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            // The poll belongs to the runtime, and closing a selector releases
            // engine objects held for it — so it has to go down first, while
            // the heap is still there. `ray_runtime_destroy` does not do this
            // itself, so a process that only ever used a `TcpClient` leaked it.
            let poll = sys::ray_runtime_get_poll();
            if !poll.is_null() {
                sys::ray_runtime_set_poll(ptr::null_mut());
                sys::ray_poll_destroy(poll.cast());
            }
            sys::ray_runtime_destroy(self.rt);
        }
        LIVE.store(false, Ordering::SeqCst);
    }
}

/// Is a runtime currently live in this process?
///
/// True only inside a [`Runtime::scope`] body.
pub fn is_live() -> bool {
    LIVE.load(Ordering::SeqCst)
}

/// Panic unless a runtime is live.
///
/// The engine has no guard of its own: `ray_eval` and the atom constructors
/// dereference a thread-local VM that is null with no live runtime. This is an
/// unconditional assertion, not a `debug_assert` — the release build has exactly
/// the same hole.
#[inline]
pub(crate) fn assert_live(what: &str) {
    assert!(is_live(), "rayforce: {what} requires a live Runtime");
}

/// Bind `value` to a global name. Requires a live [`Runtime`].
pub fn set_global(name: &str, value: &Value) -> Result<()> {
    assert_live("set_global");
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
    assert_live("get_global");
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

/// Evaluate a Rayfall source string. Requires a live [`Runtime`].
///
/// A void / null result becomes [`Value::null`]; a core error becomes `Err`.
pub fn eval(source: &str) -> Result<Value> {
    assert_live("eval");
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
    assert_live("eval_value");
    unsafe {
        let r = sys::ray_eval(obj.as_ptr());
        if r.is_null() {
            return Ok(Value::null());
        }
        Ok(Value::from_owned(materialize(check(r)?)?))
    }
}
