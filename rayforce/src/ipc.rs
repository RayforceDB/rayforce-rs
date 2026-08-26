//! IPC client for talking to a RayforceDB server over TCP.
//!
//! [`TcpClient`] wraps the core's blocking client API (`ray_ipc_connect` /
//! `ray_ipc_send` / `ray_ipc_send_async` / `ray_ipc_close`). Connection handles
//! are process-local; the client is `!Send`/`!Sync` like the rest of the crate.

use crate::error::{check, materialize, RayError, Result};
use crate::runtime::assert_live;
use crate::value::Value;
use rayforce_sys as sys;
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
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<TcpClient> {
        assert_live("TcpClient::connect");
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
                return Err(RayError::binding(format!(
                    "connect to {host}:{port} failed"
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
