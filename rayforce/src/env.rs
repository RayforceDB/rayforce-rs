//! Binding native functions into the global environment.
//!
//! A name bound here is callable from Rayfall source — and, more usefully, from
//! a *remote* peer. When a q publisher pushes `` (`upd; payload) `` down an
//! attached [`crate::q::Subscription`], the event loop looks `upd` up in this
//! environment and calls whatever it finds. Binding a Rust function under that
//! name is how pushed data reaches Rust.
//!
//! ```no_run
//! use rayforce::{env, Runtime, Value};
//!
//! struct OnUpd;
//! impl env::VaryFn for OnUpd {
//!     fn call(args: env::Args<'_>) -> Value {
//!         println!("pushed frame with {} argument(s)", args.len());
//!         Value::null()      // an async push expects no reply
//!     }
//! }
//!
//! Runtime::scope(|_rt| {
//!     env::bind_vary::<OnUpd>("upd")?;
//!     Ok(())
//! })?;
//! # Ok::<(), rayforce::RayError>(())
//! ```
//!
//! # Why a trait and not a closure
//!
//! The core takes a bare C function pointer with no user-data argument, so
//! there is nowhere to put a closure's captured environment. Binding a *type*
//! instead sidesteps that: each `H` gets its own monomorphized trampoline, so
//! the function address itself carries the identity that a data pointer
//! normally would. State that a handler needs lives in statics.
//!
//! The trampoline also owns the parts that are easy to get wrong — borrowing
//! the argument array without taking ownership of it, and catching a panic
//! before it can unwind into C — so implementors write ordinary safe Rust.

use std::ffi::{CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

use rayforce_sys as sys;

use crate::error::{RayError, Result};
use crate::raw;
use crate::value::Value;

/// The arguments of a call, borrowed for its duration.
///
/// Nothing is converted up front: [`Args::get`] takes a reference to the one
/// element you ask for, so a handler that reads a single argument out of a
/// wide call does not pay for the rest.
pub struct Args<'a> {
    raw: &'a [*mut sys::ray_t],
}

impl Args<'_> {
    /// How many arguments the caller passed.
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// True if the call had no arguments.
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Argument `i`, or `None` if the call had fewer.
    ///
    /// The engine owns the arguments for the duration of the call; the returned
    /// [`Value`] takes its own reference, so it can outlive this `Args` — and
    /// keeping one is how a handler retains a pushed batch.
    pub fn get(&self, i: usize) -> Option<Value> {
        let ptr = *self.raw.get(i)?;
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { Value::from_borrowed(ptr) })
    }
}

/// A handler bound with [`bind_vary`]: any arity.
///
/// Prefer this over [`UnaryFn`] for a q push handler. The two peer kinds
/// disagree about arity: a dict-form publisher sends `` (`upd; dict) `` — one
/// argument — while a kdb+ tickerplant sends `` (`upd;`trade; tbl) `` — two. A
/// fixed arity does not error, it silently drops every frame from the other
/// kind, so one variadic binding is what serves both.
///
/// Return [`Value::null`] when there is nothing to reply with, which is the
/// normal case for an async push.
pub trait VaryFn {
    fn call(args: Args<'_>) -> Value;
}

/// A handler bound with [`bind_unary`]: exactly one argument.
///
/// For handlers whose arity is not in doubt — a dict-form publisher's `eod`, for
/// instance, which it sends **synchronously**, so leaving it unbound leaves the
/// peer blocked waiting on a reply.
pub trait UnaryFn {
    fn call(arg: Value) -> Value;
}

/// Turn a panic into a null reply.
///
/// A handler runs on the event loop with C frames on either side of it, and
/// unwinding through those is undefined. Swallowing the panic keeps one
/// malformed frame from taking the process down or wedging the stream.
fn guard(name: &str, f: impl FnOnce() -> Value) -> *mut sys::ray_t {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v.into_raw(),
        Err(_) => {
            eprintln!("rayforce: handler `{name}` panicked; replying null");
            Value::null().into_raw()
        }
    }
}

extern "C" fn vary_trampoline<H: VaryFn>(args: *mut *mut sys::ray_t, n: i64) -> *mut sys::ray_t {
    guard(std::any::type_name::<H>(), || {
        let raw: &[*mut sys::ray_t] = if args.is_null() || n <= 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(args, n as usize) }
        };
        H::call(Args { raw })
    })
}

extern "C" fn unary_trampoline<H: UnaryFn>(arg: *mut sys::ray_t) -> *mut sys::ray_t {
    guard(std::any::type_name::<H>(), || {
        let v = if arg.is_null() {
            Value::null()
        } else {
            unsafe { Value::from_borrowed(arg) }
        };
        H::call(v)
    })
}

/// Bind an already-built function object under `name`.
///
/// Binds it **twice**, deliberately. `ray_env_bind` walks dotted-segment dicts
/// so `.foo.bar` nests properly; `ray_env_bind_flat` writes the fully-qualified
/// name straight into the global hash. rayforce-q's inbound dispatch resolves
/// against the flat binding, so a name bound only the first way is invisible to
/// a pushed frame. rayforce-q's own `embed/rayforce_q.c` calls both for the
/// same reason.
fn bind_obj(name: &str, c: &CStr, f: *mut sys::ray_t) -> Result<()> {
    if f.is_null() || unsafe { raw::is_err(f) } {
        // `ray_fn_*` reports allocation failure as an error *object*, not NULL.
        if !f.is_null() {
            unsafe { sys::ray_release(f) };
        }
        return Err(RayError::binding(format!(
            "could not build a function object for `{name}`"
        )));
    }
    unsafe {
        let id = sys::ray_sym_intern(c.as_ptr(), name.len());
        let a = sys::ray_env_bind(id, f);
        let b = sys::ray_env_bind_flat(id, f);
        sys::ray_release(f);
        // Both binders can refuse — the global table is capped, and reserved
        // names are rejected outright. Silently returning Ok would leave the
        // caller waiting for frames that dispatch nowhere.
        if a != sys::ray_err_t_RAY_OK || b != sys::ray_err_t_RAY_OK {
            return Err(RayError::binding(format!(
                "could not bind `{name}` (rc {}, flat rc {})",
                a as i64, b as i64
            )));
        }
    }
    Ok(())
}

/// Bind `H` under `name` as a variadic function.
///
/// Requires a live [`crate::Runtime`], on its thread.
pub fn bind_vary<H: VaryFn>(name: &str) -> Result<()> {
    let c = bind_name(name)?;
    let f = unsafe { sys::ray_fn_vary(c.as_ptr(), sys::RAY_FN_NONE, Some(vary_trampoline::<H>)) };
    bind_obj(name, &c, f)
}

/// Bind `H` under `name` as a one-argument function.
///
/// Requires a live [`crate::Runtime`], on its thread.
pub fn bind_unary<H: UnaryFn>(name: &str) -> Result<()> {
    let c = bind_name(name)?;
    let f = unsafe { sys::ray_fn_unary(c.as_ptr(), sys::RAY_FN_NONE, Some(unary_trampoline::<H>)) };
    bind_obj(name, &c, f)
}

/// Validate a binding name and require the runtime's own thread.
///
/// `ray_fn_*` allocates out of the engine heap, which is thread-local, so
/// calling either binder off that thread is a use-after-free rather than an
/// error the core reports.
fn bind_name(name: &str) -> Result<CString> {
    if !crate::runtime::on_runtime_thread() {
        return Err(RayError::binding(
            "binding a handler requires a live rayforce runtime on this thread",
        ));
    }
    if name.is_empty() {
        return Err(RayError::binding("function name is empty"));
    }
    CString::new(name).map_err(|_| RayError::binding("function name contains NUL"))
}
