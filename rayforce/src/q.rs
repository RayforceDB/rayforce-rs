//! Q IPC client — connect to a Q server, send a query, get the response decoded
//! into a rayforce [`Value`].
//!
//! Wraps the `rayforce-q` client (`q.c`/`q.h`), built by `rayforce-sys` from a
//! local checkout (env `RAYFORCE_Q_SRC`, default `~/rayforce-q`). Requires a
//! live [`crate::Runtime`].
//!
//! [`QConnection`] is sync request/reply. To *receive* — a tickerplant or a
//! dict-form publisher pushing at you — attach it to the event loop with
//! [`QConnection::attach`] and drive a [`Subscription`]; see that type for why
//! the plain client cannot do it.
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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::marker::PhantomData;

use rayforce_sys as sys;

use crate::error::{check, RayError, Result};
use crate::poll::Poll;
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

thread_local! {
    /// The attach that currently owns each selector id.
    ///
    /// Selector ids are slot indices that `ray_poll_register` reuses the moment
    /// a connection goes away, and the rx state the slot points at is malloc'd,
    /// so it can come back at the same address too — the pair is not an
    /// identity on its own. A counter bumped on every attach is, for every
    /// connection this crate opened.
    static ATTACHED: RefCell<HashMap<i64, u64>> = RefCell::new(HashMap::new());
    static NEXT_TOKEN: Cell<u64> = const { Cell::new(1) };
}

fn claim_selector(id: i64) -> u64 {
    let token = NEXT_TOKEN.with(|n| {
        let t = n.get();
        n.set(t + 1);
        t
    });
    ATTACHED.with(|a| a.borrow_mut().insert(id, token));
    token
}

fn selector_is_claimed_by(id: i64, token: u64) -> bool {
    ATTACHED.with(|a| a.borrow().get(&id) == Some(&token))
}

/// Give up this handle's claim, so the map does not grow with each reconnect.
/// Only clears the entry if it is still ours — a successor may already own it.
fn release_selector(id: i64, token: u64) {
    ATTACHED.with(|a| {
        let mut a = a.borrow_mut();
        if a.get(&id) == Some(&token) {
            a.remove(&id);
        }
    });
}

impl QConnection {
    /// Hand this connection to the event loop and return a [`Subscription`].
    ///
    /// Consumes the `QConnection`: from here the poll owns the fd and closing
    /// it directly is illegal, so there is no way to hold both handles at once.
    pub fn attach(self, poll: &Poll) -> Result<Subscription<'_>> {
        assert_on_runtime_thread("QConnection::attach");
        let fd = self.fd;
        // The poll takes the fd on success; suppress our `q_close`.
        std::mem::forget(self);
        // Nothing else is owed here. A handle carries no heap bookkeeping to
        // hand back — what keeps engine objects inside their scope is that the
        // types are `!Send`, so `Runtime::scope`'s bound refuses to let one
        // leave. The `Subscription` reaches the engine only through `Poll`,
        // which checks `on_runtime_thread()` before every call.
        let id = unsafe { sys::q_conn_attach(poll.as_ptr(), fd) };
        if id < 0 {
            // Attach failed, so the fd is still ours to close.
            unsafe { sys::q_close(fd) };
            return Err(RayError::binding("Q: could not attach to the event loop"));
        }
        // The selector's `data` is the per-connection state q_conn_attach just
        // allocated. Hold on to it: ids are slot indices and get reused, so it
        // is the only way to tell our connection from its successor.
        let Some(data) = poll.selector_data(id) else {
            unsafe { sys::q_conn_close(poll.as_ptr(), id) };
            return Err(RayError::binding("Q: attached selector did not resolve"));
        };
        let token = claim_selector(id);
        Ok(Subscription {
            poll,
            id,
            data,
            token,
        })
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

/// A Q connection running on the [`Poll`], able to receive pushed frames.
///
/// # Why this exists
///
/// [`QConnection::execute`] is blocking request/response over a raw socket, so
/// a frame the peer pushes unsolicited would be read as the answer to the next
/// call. That makes a subscription impossible with the client alone — you send
/// `.u.sub` and then every batch that arrives corrupts the next round-trip.
///
/// Attaching the fd to the event loop moves the reads into the loop, which
/// routes each frame by message type: a RESPONSE wakes a parked [`send`], and
/// anything else is dispatched to the function the publisher named. Bind that
/// name with [`crate::env::bind_vary`] and pushed data lands in your code.
///
/// ```no_run
/// # use rayforce::{env, q::QConnection, Poll, Runtime, Value};
/// struct OnUpd;
/// impl env::VaryFn for OnUpd {
///     fn call(args: env::Args<'_>) -> Value {
///         println!("batch: {} arg(s)", args.len());
///         Value::null()
///     }
/// }
///
/// Runtime::scope(|_rt| {
///     let poll = Poll::install()?;
///     env::bind_vary::<OnUpd>("upd")?;
///
///     let sub = QConnection::connect("localhost", 5010)?.attach(&poll)?;
///     sub.execute(".u.sub[`trade;`]")?;
///     while sub.is_alive() {
///         poll.run_for(200)?;   // on_upd fires from in here
///     }
///     Ok(())
/// })?;
/// # Ok::<(), rayforce::RayError>(())
/// ```
///
/// [`send`]: Subscription::send
/// Borrows the [`Poll`] it runs on, so it cannot outlive the event loop that
/// owns its socket — the teardown order that C callers have to remember is a
/// compile error here.
pub struct Subscription<'p> {
    poll: &'p Poll,
    /// The poll selector this connection was registered under.
    id: i64,
    /// The rx machine's per-connection state. Part of this connection's
    /// identity: a selector that resolves but points at different state is a
    /// different connection.
    data: *mut c_void,
    /// The other part. `data` alone is not enough — it is a freed-and-reused
    /// allocation, so a successor can land on the same address.
    token: u64,
}

impl Subscription<'_> {
    /// Sync round-trip on the attached connection.
    ///
    /// Unlike a bare socket exchange this *pumps* the connection while it
    /// waits, dispatching rather than swallowing any frame that arrives before
    /// the response — so a handler bound with [`crate::env::bind_vary`] can
    /// fire before this call returns. A subscribe whose first batch overtakes
    /// its own acknowledgement is normal, not a race.
    ///
    /// Do not call this from inside a handler: the C layer refuses a nested
    /// sync send on a busy handle, and you will get an error rather than a
    /// deadlock.
    pub fn send(&self, msg: &Value) -> Result<Value> {
        if !self.is_alive() {
            return Err(RayError::binding("Q: subscription is closed"));
        }
        let res = unsafe { sys::q_conn_send(self.poll.as_ptr(), self.id, msg.as_ptr()) };
        if res.is_null() {
            return Err(RayError::binding(
                "Q: send on the attached connection failed",
            ));
        }
        let ok = unsafe { check(res)? };
        Ok(unsafe { Value::from_owned(ok) })
    }

    /// Send `expr` as a char vector for the peer to evaluate, and return its
    /// answer.
    ///
    /// The peer evaluates the string itself — q on a kdb+ process, Rayfall on a
    /// `rayforce -q` server — which is also how you tell the two apart.
    pub fn execute(&self, expr: &str) -> Result<Value> {
        self.send(&Value::string(expr))
    }

    /// Is the connection still up?
    ///
    /// A peer disconnect is not an error and does not interrupt
    /// [`Poll::run_for`]: the rx machine simply deregisters the selector, and
    /// this turns false. Poll it between slices — that is the only notice you
    /// get, and without it a process behind a VPN whose tunnel was reaped sits
    /// there looking healthy while receiving nothing.
    pub fn is_alive(&self) -> bool {
        selector_is_claimed_by(self.id, self.token) && self.poll.holds(self.id, self.data)
    }

    /// Close the connection now rather than at the end of the scope.
    pub fn close(self) {
        // Consuming `self` runs `Drop`, which is where the close lives.
    }
}

impl Drop for Subscription<'_> {
    fn drop(&mut self) {
        // `is_alive` checks the selector's identity, not just its slot, so this
        // cannot deregister a connection that inherited our id after the peer
        // went away — nor touch a poll whose runtime is already gone.
        if self.is_alive() {
            unsafe { sys::q_conn_close(self.poll.as_ptr(), self.id) };
        }
        release_selector(self.id, self.token);
    }
}

/// Encode `msg` as a complete Q wire message — the 8-byte header followed by
/// the serialized body — ready to hand to any transport.
///
/// The mirror of [`decode_response`], and the half a *publisher* needs: build a
/// value, encode it, write the bytes on a socket you own.
///
/// # The message-type byte
///
/// The header is stamped `SYNC`, because that is what the underlying encoder
/// emits and it has no way to know your intent. A publisher pushing
/// unsolicited must retype byte 1 in place before writing:
///
/// ```no_run
/// # use rayforce::{q, Runtime, Value};
/// Runtime::scope(|_rt| {
///     const ASYNC: u8 = 0;
///     let mut frame = q::encode(&Value::i64(42))?;
///     frame[1] = ASYNC;          // 0 async, 1 sync, 2 response
///     Ok(())
/// })?;
/// # Ok::<(), rayforce::RayError>(())
/// ```
///
/// Touches the symbol table, so it must run on the thread that owns the
/// [`crate::Runtime`].
///
/// # Dictionaries
///
/// The encoder has no native dictionary branch: it expects a dict as a
/// 2-element list carrying the dict attribute, and that marker does not survive
/// the wire. A peer expecting a true q dict (type 99) will not see one. Encode
/// the keys and values separately and splice the `0x63` marker between their
/// bodies if you need to emit that shape.
pub fn encode(msg: &Value) -> Result<Vec<u8>> {
    let mut buf: *mut u8 = std::ptr::null_mut();
    let mut len: i64 = 0;
    let mut err = [0 as std::os::raw::c_char; 256];
    let rc = unsafe {
        sys::q_encode(
            msg.as_ptr(),
            &mut buf,
            &mut len,
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if rc < 0 || buf.is_null() {
        let reason = unsafe { std::ffi::CStr::from_ptr(err.as_ptr()) }.to_string_lossy();
        let reason = if reason.is_empty() {
            "q_encode failed".into()
        } else {
            reason
        };
        return Err(RayError::binding(format!("Q: {reason}")));
    }
    // The buffer is malloc'd by C; copy it out and hand it straight back.
    let out = unsafe { std::slice::from_raw_parts(buf, len as usize) }.to_vec();
    unsafe { libc_free(buf.cast()) };
    Ok(out)
}

/// `free(3)`. The encoder allocates with `malloc`, so its buffer must go back
/// to the same allocator rather than through Rust's.
unsafe fn libc_free(p: *mut std::ffi::c_void) {
    extern "C" {
        fn free(p: *mut std::ffi::c_void);
    }
    free(p);
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
    if msg[0] != 1 {
        return Err(RayError::binding(
            "Q: only little-endian response messages are supported",
        ));
    }
    if msg[1] != 2 {
        return Err(RayError::binding(format!(
            "Q: expected response message type 2, got {}",
            msg[1]
        )));
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
