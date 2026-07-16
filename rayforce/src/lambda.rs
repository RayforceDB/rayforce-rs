//! User-defined lambda functions (`RAY_LAMBDA`).
//!
//! [`Fn`] wraps a compiled lambda object. Build one from Rayfall source with
//! [`Fn::new`], or grab one already bound in the global env — e.g. defined in a
//! file loaded via `(load "…")` — with [`Fn::from_global`].
//!
//! It supports two evaluation modes:
//!
//! * [`Fn::call`] — direct application. Builds the literal `((fn …) args)`
//!   shape and evaluates it immediately. Leaks no global bindings.
//! * [`Fn::apply`] — deferred application inside a query, producing an [`Expr`].
//!   The engine's DAG compiler rejects a raw `RAY_LAMBDA` in head position, so
//!   the lambda is bound into the global env under a unique generated name and
//!   referenced by that name (the named-reference branch the compiler
//!   β-reduces).
//!
//! # Direct call
//!
//! ```no_run
//! use rayforce::{Fn, Runtime, Value};
//! let _rt = Runtime::new().unwrap();
//! let square = Fn::new("(fn [x] (* x x))").unwrap();
//! // On a scalar…
//! assert_eq!(square.call(&[Value::i64(5)]).unwrap().as_i64().unwrap(), 25);
//! // …or element-wise over a vector.
//! let v = square.call(&[Value::vec(&[2i64, 3, 4])]).unwrap();
//! assert_eq!(v.as_slice::<i64>().unwrap(), &[4, 9, 16]);
//! ```
//!
//! # Applying inside a query
//!
//! ```no_run
//! use rayforce::{col, Fn, Runtime, Table, Value};
//! let _rt = Runtime::new().unwrap();
//! let t = Table::new(
//!     &["id", "value"],
//!     &[Value::sym_vec(&["a", "b", "c"]), Value::vec(&[2i64, 3, 4])],
//! )
//! .unwrap();
//!
//! let square = Fn::new("(fn [x] (* x x))").unwrap();
//! let out = t
//!     .select()
//!     .col("id")
//!     .agg("squared", square.apply([col("value")]).unwrap())
//!     .execute()
//!     .unwrap();
//! assert_eq!(out.column("squared").unwrap().as_slice::<i64>().unwrap(), &[4, 9, 16]);
//! ```
//!
//! # Lambdas loaded from a file
//!
//! ```no_run
//! use rayforce::{eval, Fn, Runtime, Value};
//! let _rt = Runtime::new().unwrap();
//! eval("(load \"prelude.rfl\")").unwrap(); // defines e.g. a `sq` lambda
//! let sq = Fn::from_global("sq").unwrap();
//! assert_eq!(sq.call(&[Value::i64(9)]).unwrap().as_i64().unwrap(), 81);
//! ```
//!
//! Mirrors `rayforce-py`'s `Fn` type.

use std::cell::RefCell;

use rayforce_sys as sys;

use crate::error::{RayError, Result};
use crate::expr::{Expr, IntoExpr};
use crate::ops::Operation;
use crate::runtime::{eval, eval_value, get_global, set_global};
use crate::value::Value;

thread_local! {
    /// Per-thread counter for the generated `__rustfn_*` env names bound by
    /// [`Fn::apply`]. Thread-local because the core VM (and thus its env) is
    /// pinned to a single thread.
    static BIND_COUNTER: RefCell<u64> = const { RefCell::new(1) };
}

/// A compiled RayforceDB lambda function.
///
/// Cheap to [`Clone`] (the underlying [`Value`] is reference-counted). Not
/// `Send`/`Sync` — like every [`Value`], it is bound to the runtime thread.
pub struct Fn {
    value: Value,
    /// The Rayfall source, when the lambda was built from one — used for a
    /// legible [`Display`]/[`Debug`] (the core formatter only prints `lambda`).
    source: Option<String>,
    /// The generated global name this lambda is bound under, populated lazily
    /// on the first [`Fn::apply`] so a call-only lambda never leaks a binding.
    bound_name: RefCell<Option<String>>,
}

impl Fn {
    /// Compile a lambda from Rayfall source, e.g. `"(fn [x] (* x x))"`.
    ///
    /// Errors if the source is not a `(fn …)` expression or does not evaluate
    /// to a lambda. Requires a live [`crate::Runtime`].
    pub fn new(source: &str) -> Result<Fn> {
        if !source.trim_start().starts_with("(fn") {
            return Err(RayError::binding(
                "Fn::new: source must be a `(fn …)` expression",
            ));
        }
        let mut f = Fn::from_value(eval(source)?)?;
        f.source = Some(source.to_string());
        Ok(f)
    }

    /// Look up a lambda already bound in the global env by `name` — e.g. one
    /// defined in a file loaded via `(load "…")`.
    ///
    /// Errors if the name is unbound or does not resolve to a lambda. Requires
    /// a live [`crate::Runtime`].
    pub fn from_global(name: &str) -> Result<Fn> {
        let f = Fn::from_value(get_global(name)?)?;
        // Reuse the existing binding rather than generating a fresh one in
        // `apply`: it already resolves by this name.
        *f.bound_name.borrow_mut() = Some(name.to_string());
        Ok(f)
    }

    /// Wrap an existing lambda [`Value`] (e.g. one returned by [`crate::eval`]).
    ///
    /// Errors if `value` is not a `RAY_LAMBDA`.
    pub fn from_value(value: Value) -> Result<Fn> {
        if value.type_code() != sys::RAY_LAMBDA as i8 {
            return Err(RayError::binding(format!(
                "Fn::from_value: expected a lambda, got type {}",
                value.type_code()
            )));
        }
        Ok(Fn {
            value,
            source: None,
            bound_name: RefCell::new(None),
        })
    }

    /// Borrow the underlying lambda [`Value`].
    #[inline]
    pub fn as_value(&self) -> &Value {
        &self.value
    }

    /// Call the lambda directly and evaluate immediately.
    ///
    /// Uses the literal `((fn …) args)` shape, so no global binding is created.
    pub fn call(&self, args: &[Value]) -> Result<Value> {
        let mut items = Vec::with_capacity(args.len() + 1);
        items.push(self.value.clone());
        items.extend(args.iter().cloned());
        eval_value(&Value::list(&items))
    }

    /// Bind the lambda into the global env under a unique name (once), returning
    /// that name so a query can reference it. Subsequent calls reuse the name.
    fn bind(&self) -> Result<String> {
        if let Some(name) = self.bound_name.borrow().as_ref() {
            return Ok(name.clone());
        }
        let n = BIND_COUNTER.with(|c| {
            let mut c = c.borrow_mut();
            let n = *c;
            *c += 1;
            n
        });
        let name = format!("__rustfn_{n:x}");
        set_global(&name, &self.value)?;
        *self.bound_name.borrow_mut() = Some(name.clone());
        Ok(name)
    }

    /// Apply the lambda to expression operands, producing an [`Expr`] usable in
    /// query projections / predicates (e.g. `select().agg("sq", sq.apply([col("v")])?)`).
    ///
    /// Binds the lambda into the global env on first use so the query DAG
    /// compiler can resolve and β-reduce it by name. Requires a live
    /// [`crate::Runtime`].
    pub fn apply<I>(&self, args: I) -> Result<Expr>
    where
        I: IntoIterator,
        I::Item: IntoExpr,
    {
        let name = self.bind()?;
        let operands = args.into_iter().map(IntoExpr::into_expr).collect();
        Ok(Expr::Call(name, operands))
    }

    /// The source string this lambda was built from, if any.
    #[inline]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// The lambda's metadata dict (`(meta fn)`), as reported by the engine.
    pub fn meta(&self) -> Result<Value> {
        let ast = Value::list(&[
            Value::name_ref(Operation::Meta.as_str()),
            self.value.clone(),
        ]);
        eval_value(&ast)
    }
}

impl Clone for Fn {
    fn clone(&self) -> Fn {
        // A clone shares the same env binding (if any) — it is the same lambda.
        Fn {
            value: self.value.clone(),
            source: self.source.clone(),
            bound_name: RefCell::new(self.bound_name.borrow().clone()),
        }
    }
}

impl std::fmt::Display for Fn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The core formatter only prints `lambda`; prefer the original source.
        match &self.source {
            Some(src) => f.write_str(src),
            None => f.write_str(&self.value.format()),
        }
    }
}

impl std::fmt::Debug for Fn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fn({self})")
    }
}
