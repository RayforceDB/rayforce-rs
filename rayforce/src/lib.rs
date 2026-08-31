//! Convenient, high-performance Rust bindings for RayforceDB v2.
//!
//! Binds the core `ray_*` C API directly (via [`rayforce_sys`]); see `PLAN.md`
//! for the roadmap. The crate is single-threaded by construction: the core runs
//! on one thread with a thread-local VM and allows a single live [`Runtime`] per
//! process. [`Runtime::scope`] brackets it: the closure gets a `&Runtime` for as
//! long as it runs, and everything built inside is torn down with it.
//!
//! ```no_run
//! rayforce::Runtime::scope(|rt| {
//!     let two = rt.eval("(+ 1 1)")?;
//!     assert_eq!(two.format(), "2");
//!     Ok(())
//! })
//! # .unwrap();
//! ```

mod convert;
mod dict;
mod error;
mod expr;
mod ipc;
mod lambda;
mod list;
mod ops;
pub mod q;
mod query;
mod raw;
mod runtime;
mod scalars;
mod table;
mod value;
mod vector;

pub use convert::{FromValue, Guid, Str, ToValue};
pub use error::{ErrorCode, RayError, Result};
pub use expr::{
    avg, col, count, distinct, first, last, lit, max, median, min, sum, Expr, IntoExpr,
};
pub use ipc::TcpClient;
pub use lambda::Fn;
pub use ops::Operation;
pub use q::QConnection;
pub use query::{Select, Update};
pub use runtime::{eval, eval_value, get_global, on_runtime_thread, set_global, Runtime};
pub use table::Table;
pub use value::Value;
pub use vector::{VecElem, VecIter};

/// Re-export of the raw FFI crate for advanced / unsafe use.
pub use rayforce_sys as sys;
