//! IPC client for talking to a RayforceDB server over TCP.
//!
//! [`TcpClient`] wraps the core's blocking client API (`ray_ipc_connect` /
//! `ray_ipc_send_timeout` / `ray_ipc_send_async` / `ray_ipc_close`). Connection
//! handles are process-local; the client is `!Send`/`!Sync` like the rest of
//! the crate.

use crate::error::{check, materialize, RayError, Result};
use crate::runtime::assert_on_runtime_thread;
use crate::value::Value;
use rayforce_sys as sys;
use std::borrow::Cow;
use std::cell::Cell;
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::time::Duration;

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

/// Whether `handle` names a live connection in the runtime's poll.
///
/// A handle is a slot in the poll's selector table, and a slot is reused: the
/// next connection opened takes the lowest free one. So this answers whether
/// *a* connection sits there, not whether it is the one the handle was issued
/// for. Asked right after a send on `handle` returns, the two agree, unless the
/// exchange itself opened a connection — a frame the server pushed meanwhile,
/// evaluated here, calling `.ipc.open`.
unsafe fn is_live(handle: i64) -> bool {
    let mut info = MaybeUninit::<sys::ray_ipc_tx_info_t>::uninit();
    sys::ray_ipc_tx_info(handle, info.as_mut_ptr()) == sys::ray_err_t_RAY_OK
}

/// What [`TcpClient`] holds once the core has closed its connection. Never a
/// handle: those are slot indices, from 0.
const CLOSED: i64 = -1;

/// The core adds the timeout to its monotonic millisecond clock to get a
/// deadline, so the cap leaves that sum room to never overflow.
const MAX_TIMEOUT_MS: i64 = i64::MAX / 2;

/// `timeout` in the whole milliseconds `ray_ipc_send_timeout` takes, rounded
/// up: truncated, a sub-millisecond budget would become 0, which the core reads
/// as no deadline at all. Zero itself is refused for the same reason.
fn timeout_ms(timeout: Duration) -> Result<i64> {
    if timeout.is_zero() {
        return Err(RayError::binding(
            "TcpClient: a send timeout must be greater than zero",
        ));
    }
    let ms = timeout.as_nanos().div_ceil(1_000_000);
    Ok(i64::try_from(ms).map_or(MAX_TIMEOUT_MS, |ms| ms.min(MAX_TIMEOUT_MS)))
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
    /// [`CLOSED`] once the core has closed the connection under us; see
    /// [`TcpClient::round_trip`].
    handle: Cell<i64>,
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
                handle: Cell::new(handle),
                _not_send: PhantomData,
            })
        }
    }

    /// Send a Rayfall source string for the server to evaluate, returning the
    /// response.
    pub fn execute(&self, query: &str) -> Result<Value> {
        self.send(&Value::string(query))
    }

    /// [`execute`](Self::execute), giving up once `timeout` has passed; see
    /// [`send_timeout`](Self::send_timeout).
    pub fn execute_timeout(&self, query: &str, timeout: Duration) -> Result<Value> {
        self.send_timeout(&Value::string(query), timeout)
    }

    /// Send a pre-built message value, returning the response. Waits as long as
    /// the server takes.
    ///
    /// If the server goes away mid-exchange, this fails and the client is
    /// closed, as after a [`send_timeout`](Self::send_timeout) expiry.
    pub fn send(&self, msg: &Value) -> Result<Value> {
        self.round_trip(msg, 0)
    }

    /// [`send`](Self::send), giving up once `timeout` has passed.
    ///
    /// The deadline covers the whole round trip, the request write included.
    /// On expiry the core sends the server the same cancel as Ctrl-C, closes
    /// the connection, and this returns an `io` error. The client stays closed:
    /// every later call on it fails without touching the network, so connect
    /// again to carry on. Closing is the core's call, and a forced one — a
    /// response carries no request id, so a late one would be taken as the
    /// answer to the next request.
    ///
    /// `timeout` is rounded up to whole milliseconds. Zero is refused with a
    /// `binding` error before anything is sent.
    pub fn send_timeout(&self, msg: &Value, timeout: Duration) -> Result<Value> {
        self.round_trip(msg, timeout_ms(timeout)?)
    }

    /// One synchronous exchange; `timeout_ms` 0 waits as long as it takes.
    fn round_trip(&self, msg: &Value, timeout_ms: i64) -> Result<Value> {
        let handle = self.live_handle()?;
        unsafe {
            let r = sys::ray_ipc_send_timeout(handle, msg.as_ptr(), timeout_ms);
            if r.is_null() {
                return Err(RayError::binding("ipc send failed"));
            }
            let r = check(r);
            // An error is either the server's answer, with the connection
            // intact, or the core's own, some of which close it: an expired
            // deadline, or a peer gone mid-exchange. Only the slot tells them
            // apart — the text cannot, as a server can answer with any error.
            // A closed connection's slot goes to the next one opened, so from
            // here on the handle would send to, and drop would close, a
            // connection that is not ours.
            if r.is_err() && !is_live(handle) {
                self.handle.set(CLOSED);
            }
            Ok(Value::from_owned(materialize(r?)?))
        }
    }

    /// The handle, unless the core has closed the connection.
    fn live_handle(&self) -> Result<i64> {
        match self.handle.get() {
            CLOSED => Err(RayError::binding(
                "TcpClient: the connection is closed (a send timed out, or the \
                 server went away)",
            )),
            handle => Ok(handle),
        }
    }

    /// Fire-and-forget send (no response).
    pub fn send_async(&self, msg: &Value) -> Result<()> {
        let handle = self.live_handle()?;
        unsafe {
            let e = sys::ray_ipc_send_async(handle, msg.as_ptr());
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
        let handle = self.handle.get();
        if handle != CLOSED {
            unsafe { sys::ray_ipc_close(handle) }
        }
    }
}
