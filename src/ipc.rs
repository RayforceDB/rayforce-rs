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

//! IPC client for talking to a Rayforce server.
//!
//! Wraps the public `ray_ipc_*` C symbols restored in rayforce 2.1:
//! `ray_ipc_connect` / `ray_ipc_close` / `ray_ipc_send` /
//! `ray_ipc_send_async` / `ray_ipc_send_verbose`.

use crate::error::{RayforceError, Result};
use crate::ffi::{self, RayObj};
use crate::*;
use std::ffi::CString;
use std::ptr;

/// A blocking IPC connection to a Rayforce server.
///
/// Handles are process-local integer slots managed by the engine.
/// Dropping the connection calls `ray_ipc_close`.
pub struct Connection {
    handle: i64,
    closed: bool,
}

unsafe impl Send for Connection {}

impl Connection {
    /// Connect to `host:port` without authentication.
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        Self::connect_inner(host, port, None, None)
    }

    /// Connect to `host:port` with `user`/`password` credentials.
    pub fn connect_with_auth(
        host: &str,
        port: u16,
        user: Option<&str>,
        password: &str,
    ) -> Result<Self> {
        Self::connect_inner(host, port, user, Some(password))
    }

    fn connect_inner(
        host: &str,
        port: u16,
        user: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self> {
        let host_c = CString::new(host).map_err(|_| RayforceError::InvalidString)?;
        let user_c = user
            .map(|u| CString::new(u).map_err(|_| RayforceError::InvalidString))
            .transpose()?;
        let pwd_c = password
            .map(|p| CString::new(p).map_err(|_| RayforceError::InvalidString))
            .transpose()?;

        let user_ptr = user_c.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null());
        let pwd_ptr = pwd_c.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null());

        let h = unsafe { ray_ipc_connect(host_c.as_ptr(), port, user_ptr, pwd_ptr) };
        match h {
            x if x >= 0 => Ok(Self { handle: x, closed: false }),
            -2 => Err(RayforceError::IoError(
                "auth required but no credentials provided".into(),
            )),
            -3 => Err(RayforceError::IoError("auth rejected".into())),
            -4 => Err(RayforceError::IoError("wire version mismatch".into())),
            _ => Err(RayforceError::IoError(format!("ipc connect failed ({host}:{port})"))),
        }
    }

    /// Raw handle slot (for embedders that want to call the C API directly).
    pub fn handle(&self) -> i64 {
        self.handle
    }

    /// True after [`close`](Self::close) has been called (or after Drop).
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Synchronously send `msg` and wait for the response.
    ///
    /// `msg` may be any `RayObj` (commonly a string source for the
    /// server to `eval_str`, or a list expression to be evaluated).
    /// The wrapper retains nothing extra: the caller's reference stays
    /// valid; the engine internally serialises a borrow of the message.
    pub fn send(&self, msg: &RayObj) -> Result<RayObj> {
        if self.closed {
            return Err(RayforceError::IoError("connection closed".into()));
        }
        unsafe {
            let result = ray_ipc_send(self.handle, msg.as_ptr());
            wrap_response(result)
        }
    }

    /// As [`send`](Self::send), but returns `[output, result]` — a list
    /// of the captured stdout from the server side and the actual
    /// result.  Useful for REPL-style integrations.
    pub fn send_verbose(&self, msg: &RayObj) -> Result<RayObj> {
        if self.closed {
            return Err(RayforceError::IoError("connection closed".into()));
        }
        unsafe {
            let result = ray_ipc_send_verbose(self.handle, msg.as_ptr());
            wrap_response(result)
        }
    }

    /// Fire-and-forget send.  No response is read; the server processes
    /// the message asynchronously.
    pub fn send_async(&self, msg: &RayObj) -> Result<()> {
        if self.closed {
            return Err(RayforceError::IoError("connection closed".into()));
        }
        unsafe {
            let rc = ray_ipc_send_async(self.handle, msg.as_ptr());
            if rc != ray_err_t_RAY_OK {
                return Err(RayforceError::from(rc));
            }
            Ok(())
        }
    }

    /// Convenience: send a Rayfall source string to the server and
    /// return the evaluated result.
    pub fn execute(&self, source: &str) -> Result<RayObj> {
        let msg = RayObj::from(source);
        self.send(&msg)
    }

    /// Close the connection.  Idempotent.  Called automatically on drop.
    pub fn close(&mut self) {
        if !self.closed {
            unsafe { ray_ipc_close(self.handle) };
            self.closed = true;
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
    }
}

/// Common post-processing for `ray_ipc_send` / `ray_ipc_send_verbose`:
/// translate NULL into a nil `RayObj`, surface engine-side errors as
/// `RayforceError::Ray`, and adopt the returned strong reference.
unsafe fn wrap_response(result: *mut ray_t) -> Result<RayObj> {
    if result.is_null() {
        return Ok(RayObj::from_raw(ptr::null_mut()));
    }
    if ffi::is_error(result) {
        let kind = ray_err_from_obj(result);
        let code = {
            let p = ray_err_code(result);
            if p.is_null() {
                "?".to_string()
            } else {
                std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        };
        ray_error_free(result);
        return Err(RayforceError::Ray {
            code: code.clone(),
            message: code,
            kind: Some(kind),
        });
    }
    Ok(RayObj::from_raw(result))
}
