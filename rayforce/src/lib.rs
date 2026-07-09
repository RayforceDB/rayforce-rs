//! Convenient, high-performance Rust bindings for RayforceDB v2.
//!
//! Binds the core `ray_*` C API directly (via [`rayforce_sys`]); see `PLAN.md`
//! for the roadmap. The crate is single-threaded by construction: the core runs
//! on one thread with a thread-local VM and allows a single live [`Runtime`] per
//! process. Hold a `Runtime` for as long as you use the API.
//!
//! ```no_run
//! let _rt = rayforce::Runtime::new().unwrap();
//! let two = rayforce::eval("(+ 1 1)").unwrap();
//! assert_eq!(two.format(), "2");
//! ```

mod convert;
mod dict;
mod error;
mod expr;
mod ipc;
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
pub use ops::Operation;
pub use q::QConnection;
pub use query::{Select, Update};
pub use runtime::{eval, eval_value, get_global, is_live, set_global, Runtime};
pub use table::Table;
pub use value::Value;
pub use vector::{VecElem, VecIter};

/// Re-export of the raw FFI crate for advanced / unsafe use.
pub use rayforce_sys as sys;
