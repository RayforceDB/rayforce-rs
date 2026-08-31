//! Q IPC client — connect to a Q server, send a query, get the response decoded
//! into a rayforce [`Value`].
//!
//! Wraps the `rayforce-q` client (`q.c`/`q.h`), built by `rayforce-sys` from a
//! local checkout (env `RAYFORCE_Q_SRC`, default `~/rayforce-q`). Sync
//! request/reply only. Requires a live [`crate::Runtime`].
//!
//! ```no_run
//! use rayforce::{Runtime, q::QConnection};
//! Runtime::scope(|_rt| {
//!     let conn = QConnection::connect("localhost", 5010).unwrap();
//!     let fills = conn.execute("select from fixmsgs where i > 0").unwrap();
//!     Ok(())
//! })
//! # .unwrap();
//! ```

use std::ffi::CString;
use std::marker::PhantomData;

use rayforce_sys as sys;

use crate::error::{check, RayError, Result};
use crate::runtime::assert_on_runtime_thread;
use crate::value::Value;

/// An open connection to a Q server. Closed on drop.
///
/// Confined to its [`crate::Runtime::scope`], like a [`Value`]: `q_close` runs
/// on drop and reaches into the runtime, so the heap has to still be mapped
/// then. Being `!Send` is what keeps it inside — the bounds on `Runtime::scope`
/// are spelled in terms of `Send`.
///
/// # Safety
///
/// `!Send`/`!Sync`, and must stay so, twice over: `execute` interns symbols and
/// builds engine objects, which belong to the thread that owns the
/// [`crate::Runtime`]; and that marker is also what confines it to its scope.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<rayforce::QConnection>();
/// ```
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<rayforce::QConnection>();
/// ```
/// Control — `compile_fail` passes on *any* build failure, a rename included:
/// ```
/// fn assert_exists<T>() {}
/// assert_exists::<rayforce::QConnection>();
/// ```
pub struct QConnection {
    fd: i32,
    _not_send: PhantomData<*mut ()>,
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
        assert_on_runtime_thread("QConnection::connect");
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
        Ok(QConnection {
            fd,
            _not_send: PhantomData,
        })
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
        // Closing releases engine objects held for the connection, so this has
        // to run while the heap is still mapped. It does: a connection cannot
        // leave the scope that owns the heap.
        unsafe { sys::q_close(self.fd) };
    }
}

/// Decode a complete Q response message (8-byte wire header + body) that was
/// received by an **external transport** — e.g. a worker thread that owns a
/// plain `std::net::TcpStream` with read timeouts — into a [`Value`].
///
/// This is the engine-thread half of a split client: socket I/O is plain
/// bytes and thread-safe anywhere, while this call allocates engine objects
/// and must run on the thread that owns the [`crate::Runtime`] (like every
/// other constructor in this crate). A Q server-side error surfaces as `Err`.
pub fn decode_response(msg: &[u8]) -> Result<Value> {
    assert_on_runtime_thread("q::decode_response");
    // q_header_t (q.c): endianness, msgtype, compressed, reserved, u32 size.
    // `size` counts the whole message, header included. Little-endian wire only.
    const HEADER_LEN: usize = 8;
    if msg.len() < HEADER_LEN {
        return Err(RayError::binding("Q: message shorter than wire header"));
    }
    let size = u32::from_le_bytes([msg[4], msg[5], msg[6], msg[7]]) as usize;
    if size != msg.len() {
        return Err(RayError::binding(
            "Q: message length does not match wire header",
        ));
    }
    let compressed = i32::from(msg[2] != 0);
    let body = &msg[HEADER_LEN..];
    if body.is_empty() {
        return Err(RayError::binding("Q: empty response body"));
    }

    let mut err = [0i8; 256];
    let res = unsafe {
        sys::q_decode(
            body.as_ptr() as *mut u8,
            body.len() as i64,
            compressed,
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if res.is_null() {
        let msg = unsafe { std::ffi::CStr::from_ptr(err.as_ptr()) }.to_string_lossy();
        let msg = if msg.is_empty() {
            "q_decode failed".into()
        } else {
            msg
        };
        return Err(RayError::binding(format!("Q: {msg}")));
    }
    let ok = unsafe { check(res)? };
    Ok(unsafe { Value::from_owned(ok) })
}
