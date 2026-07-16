//! Q IPC client — connect to a Q server, send a query, get the response decoded
//! into a rayforce [`Value`].
//!
//! Wraps the `rayforce-q` client (`q.c`/`q.h`), built by `rayforce-sys` from a
//! local checkout (env `RAYFORCE_Q_SRC`, default `~/rayforce-q`). Sync
//! request/reply only. Requires a live [`crate::Runtime`].
//!
//! ```no_run
//! use rayforce::{Runtime, q::QConnection};
//! let _rt = Runtime::new().unwrap();
//! let conn = QConnection::connect("localhost", 5010).unwrap();
//! let fills = conn.execute("select from fixmsgs where i > 0").unwrap();
//! ```

use std::ffi::CString;

use rayforce_sys as sys;

use crate::error::{check, RayError, Result};
use crate::value::Value;

/// An open connection to a Q server. Closed on drop.
pub struct QConnection {
    fd: i32,
}

impl QConnection {
    /// Open a TCP connection and perform the Q login handshake (no auth).
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        Self::connect_with(host, port, "", "", 0)
    }

    /// Open a TCP connection with optional credentials and a connect/op timeout
    /// (`timeout_ms <= 0` blocks). Empty `user`/`password` degrade to the
    /// no-auth handshake.
    pub fn connect_with(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        timeout_ms: i32,
    ) -> Result<Self> {
        let host_c = CString::new(host).map_err(|_| RayError::binding("Q host contains NUL"))?;
        let user_c = CString::new(user).map_err(|_| RayError::binding("Q user contains NUL"))?;
        let pass_c =
            CString::new(password).map_err(|_| RayError::binding("Q password contains NUL"))?;
        let fd = unsafe {
            sys::q_connect(
                host_c.as_ptr(),
                i32::from(port),
                user_c.as_ptr(),
                pass_c.as_ptr(),
                timeout_ms,
            )
        };
        if fd < 0 {
            let reason = match fd {
                sys::Q_ERR_TIMEOUT => "timed out",
                sys::Q_ERR_HANDSHAKE => "handshake/auth failed",
                _ => "connection failed",
            };
            return Err(RayError::binding(format!(
                "Q: connect to {host}:{port} {reason}"
            )));
        }
        Ok(QConnection { fd })
    }

    /// Send a query string for remote evaluation; return the response decoded
    /// into a [`Value`] (atoms, vectors, tables, …). A server-side error
    /// surfaces as `Err`.
    pub fn execute(&self, query: &str) -> Result<Value> {
        let q = Value::string(query);
        let mut err = [0 as std::os::raw::c_char; 256];
        // null: transport/serialization failure (reason in `err`). Otherwise an
        // owned result, possibly a RAY_ERROR that `check` turns into `Err`.
        let res = unsafe { sys::q_send(self.fd, q.as_ptr(), err.as_mut_ptr(), err.len()) };
        if res.is_null() {
            let msg = unsafe { std::ffi::CStr::from_ptr(err.as_ptr()) }.to_string_lossy();
            let msg = if msg.is_empty() {
                "q_send failed".into()
            } else {
                msg
            };
            return Err(RayError::binding(format!("Q: {msg}")));
        }
        let ok = unsafe { check(res)? };
        Ok(unsafe { Value::from_owned(ok) })
    }
}

impl Drop for QConnection {
    fn drop(&mut self) {
        unsafe { sys::q_close(self.fd) };
    }
}
