//! # Rayforce Rust Bindings
//!
//! Rust bindings for RayforceDB - a high-performance time-series database.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rayforce::{Rayforce, RayI64, RayF64, RaySymbol, RayVector, RayList, RayType};
//!
//! // Initialize the runtime
//! let rf = Rayforce::new().unwrap();
//!
//! // Create scalar values
//! let num = RayI64::new(42);
//! let pi = RayF64::new(3.14159);
//!
//! // Create vectors with explicit types
//! let ids = RayVector::<i64>::from_iter([1i64, 2, 3, 4, 5]);
//! let names = RayVector::<RaySymbol>::from_iter(["Alice", "Bob", "Charlie"]);
//!
//! // Create mixed lists
//! let mut list = RayList::new();
//! list.push(1i64);
//! list.push("hello");
//! list.push(3.14f64);
//!
//! // Evaluate expressions
//! let result = rf.eval("42").unwrap();
//! println!("Result: {}", result);
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod error;
pub mod ffi;
pub mod types;
pub mod query;
pub mod ipc;

pub use error::{RayforceError, Result};
pub use ffi::RayObj;
pub use types::*;
// Query types are re-exported from types::table
// pub use query::*;
pub use ipc::{Connection, hopen};

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Once;

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

static INIT: Once = Once::new();
static mut RUNTIME: *mut runtime_t = ptr::null_mut();

/// Builder for creating a Rayforce runtime with custom arguments.
pub struct RayforceBuilder {
    args: Vec<CString>,
}

impl RayforceBuilder {
    /// Create a new builder with default program name.
    pub fn new() -> Self {
        Self {
            args: vec![CString::new("rayforce").unwrap()],
        }
    }

    /// Add a command-line argument.
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(CString::new(arg).unwrap());
        self
    }

    /// Build the Rayforce runtime.
    pub fn build(self) -> Result<Rayforce> {
        unsafe {
            let mut c_args: Vec<*mut c_char> = self
                .args
                .iter()
                .map(|arg| arg.as_ptr() as *mut c_char)
                .collect();
            c_args.push(ptr::null_mut());

            let runtime = runtime_create(c_args.len() as i32 - 1, c_args.as_mut_ptr());
            if !runtime.is_null() {
                RUNTIME = runtime;
                Ok(Rayforce { runtime })
            } else {
                Err(RayforceError::RuntimeCreationFailed)
            }
        }
    }
}

impl Default for RayforceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The main Rayforce runtime handle.
///
/// This struct manages the lifecycle of the Rayforce database runtime.
/// Only one runtime can exist at a time.
pub struct Rayforce {
    runtime: *mut runtime_t,
}

// Safety: The runtime is thread-safe as documented by Rayforce
unsafe impl Send for Rayforce {}
unsafe impl Sync for Rayforce {}

impl Rayforce {
    /// Create a new Rayforce runtime with default settings.
    ///
    /// The `-r 0` flag disables the REPL for embedded use.
    pub fn new() -> Result<Self> {
        RayforceBuilder::new().with_arg("-r").with_arg("0").build()
    }

    /// Create a new Rayforce runtime with a builder.
    pub fn builder() -> RayforceBuilder {
        RayforceBuilder::new()
    }

    /// Get the Rayforce version.
    pub fn version(&self) -> u8 {
        unsafe { version() }
    }

    /// Run the Rayforce event loop (blocking).
    pub fn run(&self) -> i32 {
        unsafe { runtime_run() }
    }

    /// Get the raw runtime pointer.
    pub fn as_ptr(&self) -> *mut runtime_t {
        self.runtime
    }

    /// Evaluate a string expression.
    pub fn eval(&self, code: &str) -> Result<RayObj> {
        let c_str = CString::new(code).map_err(|_| RayforceError::InvalidString)?;
        unsafe {
            let obj = eval_str(c_str.as_ptr());
            if obj.is_null() {
                Err(RayforceError::EvalFailed("Evaluation returned null".into()))
            } else if (*obj).type_ == TYPE_ERR as i8 {
                let error_msg = ffi::get_error_message(obj);
                Err(RayforceError::EvalFailed(error_msg))
            } else {
                Ok(RayObj::from_raw(obj))
            }
        }
    }

    /// Evaluate a RayObj expression.
    pub fn eval_obj(&self, obj: &RayObj) -> Result<RayObj> {
        unsafe {
            let cloned = clone_obj(obj.as_ptr());
            let result = eval_obj(cloned);
            if result.is_null() {
                Err(RayforceError::EvalFailed("Evaluation returned null".into()))
            } else if (*result).type_ == TYPE_ERR as i8 {
                let error_msg = ffi::get_error_message(result);
                Err(RayforceError::EvalFailed(error_msg))
            } else {
                Ok(RayObj::from_raw(result))
            }
        }
    }
}

impl Drop for Rayforce {
    fn drop(&mut self) {
        unsafe {
            runtime_destroy();
            RUNTIME = ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: All runtime tests must be in one test function
    // because only one Rayforce runtime can exist at a time.
    #[test]
    fn test_runtime() {
        // Test creation
        let rf = Rayforce::new().unwrap();
        assert!(!rf.as_ptr().is_null());

        // Test version
        let v = rf.version();
        assert!(v > 0);

        // Test eval
        let result = rf.eval("42").unwrap();
        let val: i64 = result.try_into().unwrap();
        assert_eq!(val, 42);
    }
}
