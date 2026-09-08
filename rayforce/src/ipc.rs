//! IPC client for talking to a RayforceDB server over TCP.
//!
//! [`TcpClient`] wraps the core's blocking client API (`ray_ipc_connect` /
//! `ray_ipc_send` / `ray_ipc_send_async` / `ray_ipc_close`). Connection handles
//! are process-local; the client is `!Send`/`!Sync` like the rest of the crate.

use crate::error::{check, materialize, RayError, Result};
use crate::runtime::assert_on_runtime_thread;
use crate::value::Value;
use rayforce_sys as sys;
use std::borrow::Cow;
use std::ffi::CString;
use std::marker::PhantomData;

/// Ensure the runtime has a poll object (required before connecting).
unsafe fn ensure_poll() {
    if sys::ray_runtime_get_poll().is_null() {
        // Since core v2.5.8 the public header types `ray_poll_create` as
        // `ray_poll_t*`, while `ray_runtime_set_poll` still takes `void*` —
        // so the handle needs an explicit cast on the way through.
        let p = sys::ray_poll_create();
        sys::ray_runtime_set_poll(p.cast());
    }
}

/// A synchronous IPC connection to a RayforceDB server.
///
/// Confined to its [`crate::Runtime::scope`], like a [`Value`]: `ray_ipc_close`
/// runs on drop and reaches into the runtime, so the heap has to still be mapped
/// then. Being `!Send` is what keeps it inside — the bounds on `Runtime::scope`
/// are spelled in terms of `Send`.
///
/// # Safety
///
/// `!Send`/`!Sync`, and must stay so, twice over: it builds engine objects,
/// which belong to the runtime thread; and that marker is also what confines it
/// to its scope.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<rayforce::TcpClient>();
/// ```
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<rayforce::TcpClient>();
/// ```
/// Control — `compile_fail` passes on *any* build failure, a rename included:
/// ```
/// fn assert_exists<T>() {}
/// assert_exists::<rayforce::TcpClient>();
/// ```
pub struct TcpClient {
    handle: i64,
    _not_send: PhantomData<*mut ()>,
}

impl TcpClient {
    /// Connect to `host:port`, optionally authenticating. Requires a live
    /// [`crate::Runtime`].
    ///
    /// A failure names its cause: the server refused the connection, demanded
    /// credentials, rejected the ones given, speaks a different wire version,
    /// did not answer in time, or failed with some other OS error (whose text
    /// is included). Two of those read less plainly than they look:
    ///
    /// - `timed out` also covers a server that is alive and listening but busy
    ///   inside a long evaluation — the core folds `EAGAIN`/`EWOULDBLOCK` in
    ///   with `ETIMEDOUT`, because from here they are the same silence. The
    ///   budget is the core's 5s default; this call does not set one.
    /// - The OS error text behind the last case comes from `errno`, which the
    ///   core does not reliably bridge from `WSAGetLastError()` on Windows.
    ///   Only the Unix targets are built and tested here. A `host` that fails
    ///   to resolve arrives through it as `No route to host`, because that is
    ///   the `errno` the core stamps on a name-resolution failure.
    ///
    /// `server requires authentication` is not reachable through this method:
    /// the core raises it when handed a null password, and an empty `password`
    /// still arrives as a valid pointer to an empty string, which the server
    /// rejects as a bad credential instead.
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<TcpClient> {
        assert_on_runtime_thread("TcpClient::connect");
        let host_c = CString::new(host).map_err(|_| RayError::binding("host contains NUL"))?;
        let user_c = CString::new(user).map_err(|_| RayError::binding("user contains NUL"))?;
        let pass_c =
            CString::new(password).map_err(|_| RayError::binding("password contains NUL"))?;
        unsafe {
            ensure_poll();
            // engine added a 5th arg (timeout_ms) after commit a7294066; 0 = block.
            let handle =
                sys::ray_ipc_connect(host_c.as_ptr(), port, user_c.as_ptr(), pass_c.as_ptr(), 0);
            if handle < 0 {
                // Borrowed for the fixed reasons, owned only for the errno
                // one, so the common paths do not allocate. Nothing runs
                // between the call returning and the errno read but this
                // match, which cannot disturb it.
                let reason: Cow<'static, str> = match handle {
                    sys::RAY_IPC_ERR_AUTH_REQUIRED => "server requires authentication".into(),
                    sys::RAY_IPC_ERR_AUTH_FAILED => "authentication failed".into(),
                    sys::RAY_IPC_ERR_WIRE_VERSION => "wire version mismatch".into(),
                    sys::RAY_IPC_ERR_TIMEOUT => "timed out".into(),
                    sys::RAY_IPC_ERR_OS => std::io::Error::last_os_error().to_string().into(),
                    // RAY_IPC_ERR_REFUSED is the core's catch-all as much as
                    // it is its "refused", and a future core may return a code
                    // this build has never heard of.
                    _ => "connection refused".into(),
                };
                return Err(RayError::binding(format!(
                    "connect to {host}:{port} failed: {reason}"
                )));
            }
            Ok(TcpClient {
                handle,
                _not_send: PhantomData,
            })
        }
    }

    /// Send a Rayfall source string for the server to evaluate, returning the
    /// response.
    pub fn execute(&self, query: &str) -> Result<Value> {
        self.send(&Value::string(query))
    }

    /// Send a pre-built message value, returning the response.
    pub fn send(&self, msg: &Value) -> Result<Value> {
        unsafe {
            let r = sys::ray_ipc_send(self.handle, msg.as_ptr());
            if r.is_null() {
                return Err(RayError::binding("ipc send failed"));
            }
            Ok(Value::from_owned(materialize(check(r)?)?))
        }
    }

    /// Fire-and-forget send (no response).
    pub fn send_async(&self, msg: &Value) -> Result<()> {
        unsafe {
            let e = sys::ray_ipc_send_async(self.handle, msg.as_ptr());
            if e != sys::ray_err_t_RAY_OK {
                return Err(RayError::binding(format!(
                    "ipc send_async failed (err {e})"
                )));
            }
        }
        Ok(())
    }

    /// Close the connection (also done on drop).
    pub fn close(self) {
        drop(self);
    }
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        // Closing a connection releases engine objects held for it, so this
        // has to run while the heap is still mapped. It does: a client cannot
        // leave the scope that owns the heap.
        unsafe { sys::ray_ipc_close(self.handle) }
    }
}
