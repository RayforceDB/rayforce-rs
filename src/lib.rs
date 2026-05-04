/*
*   Copyright (c) 2025-2026 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

//! # Rayforce 2.0 Rust bindings
//!
//! ```rust,no_run
//! use rayforce::{Rayforce, Result};
//!
//! fn main() -> Result<()> {
//!     let rf = Rayforce::new()?;
//!     let result = rf.eval("sum 1 2 3 4 5")?;
//!     println!("Sum: {}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## What changed from the 1.0 bindings
//!
//! - The high-level query builder (`Table::select()` / `update()` / ...)
//!   has been removed because the C functions backing it
//!   (`ray_select`/`ray_update`/...) are no longer in the 2.0 public
//!   API.  Run queries via [`Rayforce::eval`] with a Rayfall source
//!   string for now.
//! - The `ipc` module is gone for the same reason: 2.0 does not yet
//!   expose `ray_hopen`/`ray_hclose`/`ray_write`/`ray_read` as public C
//!   symbols.  Both are tracked as future work once the upstream
//!   project re-publishes them.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod error;
pub mod ffi;
pub mod types;

pub use error::{RayforceError, Result};
pub use ffi::RayObj;
pub use types::*;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Builder for creating a Rayforce runtime with custom CLI args.
pub struct RayforceBuilder {
    args: Vec<CString>,
}

impl RayforceBuilder {
    pub fn new() -> Self {
        Self {
            args: vec![CString::new("rayforce").unwrap()],
        }
    }

    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(CString::new(arg).unwrap());
        self
    }

    pub fn build(self) -> Result<Rayforce> {
        unsafe {
            let mut c_args: Vec<*mut c_char> = self
                .args
                .iter()
                .map(|arg| arg.as_ptr() as *mut c_char)
                .collect();
            c_args.push(ptr::null_mut());

            let rt = ray_runtime_create(c_args.len() as i32 - 1, c_args.as_mut_ptr());
            if rt.is_null() {
                Err(RayforceError::RuntimeCreationFailed)
            } else {
                Ok(Rayforce { runtime: rt })
            }
        }
    }
}

impl Default for RayforceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for the rayforce 2.0 runtime.
///
/// Only one runtime can exist per process at a time. Dropping this
/// handle calls `ray_runtime_destroy`, which tears down the heap, env,
/// and symbol table.
pub struct Rayforce {
    runtime: *mut ray_runtime_t,
}

unsafe impl Send for Rayforce {}
unsafe impl Sync for Rayforce {}

impl Rayforce {
    /// Create a runtime with default settings.
    pub fn new() -> Result<Self> {
        RayforceBuilder::new().build()
    }

    pub fn builder() -> RayforceBuilder {
        RayforceBuilder::new()
    }

    /// Engine version as a "major.minor.patch" string.
    pub fn version(&self) -> String {
        unsafe {
            let p = ray_version_string();
            if p.is_null() {
                String::new()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }

    pub fn version_major(&self) -> i32 {
        unsafe { ray_version_major() }
    }

    pub fn version_minor(&self) -> i32 {
        unsafe { ray_version_minor() }
    }

    pub fn version_patch(&self) -> i32 {
        unsafe { ray_version_patch() }
    }

    /// Raw runtime pointer (for embedders that need to call
    /// engine-specific FFI directly).
    pub fn as_ptr(&self) -> *mut ray_runtime_t {
        self.runtime
    }

    /// Parse and evaluate a Rayfall source string.
    ///
    /// Returns the evaluated value or a `RayforceError::Ray` on error.
    /// `NULL`/void results are returned as a `RayObj` wrapping a NULL
    /// pointer (call [`RayObj::is_nil`] to detect this).
    pub fn eval(&self, code: &str) -> Result<RayObj> {
        let c_str = CString::new(code).map_err(|_| RayforceError::InvalidString)?;
        unsafe {
            let obj = ray_eval_str(c_str.as_ptr());
            if obj.is_null() {
                return Ok(RayObj::from_raw(ptr::null_mut()));
            }
            if ffi::is_error(obj) {
                let kind = ray_err_from_obj(obj);
                let code = {
                    let p = ray_err_code(obj);
                    if p.is_null() {
                        "?".to_string()
                    } else {
                        CStr::from_ptr(p).to_string_lossy().into_owned()
                    }
                };
                ray_error_free(obj);
                return Err(RayforceError::Ray {
                    code: code.clone(),
                    message: code,
                    kind: Some(kind),
                });
            }
            Ok(RayObj::from_raw(obj))
        }
    }
}

impl Drop for Rayforce {
    fn drop(&mut self) {
        unsafe {
            if !self.runtime.is_null() {
                ray_runtime_destroy(self.runtime);
                self.runtime = ptr::null_mut();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime() {
        let rf = Rayforce::new().unwrap();
        assert!(!rf.as_ptr().is_null());
        assert!(rf.version_major() >= 2);
        let result = rf.eval("42").unwrap();
        let val: i64 = result.try_into().unwrap();
        assert_eq!(val, 42);
    }
}
