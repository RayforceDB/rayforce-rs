//! Rayforce's own IPC, served from an embedding with every request guarded:
//! the wire `rayforce -p` serves and `.ipc.open` and [`crate::TcpClient`]
//! speak, as opposed to the Q wire [`Poll::serve_q`] serves.
//!
//! # Why a guard
//!
//! The server evaluates one request at a time, on the loop's thread. A
//! request that runs for minutes holds every other client for minutes, and
//! in an RDB that includes the writer pushing on the Q wire. The core stops a
//! request only when its own client sends a cancel; nothing stops one that
//! runs too long, or whose client has gone. This module does both from the
//! outside, with no change to the core:
//!
//! - It installs `.ipc.on.sync` and `.ipc.on.async`, which the core calls with
//!   each request's payload in place of evaluating it. The core takes a hook
//!   only when it is a Rayfall lambda, so the hook is a one-line lambda handing
//!   the payload to a native function.
//! - A watcher thread calls `ray_request_interrupt` when the running request
//!   passes its budget, or when its client has closed the connection.
//! - The Q wire never reads these hooks, so what a writer pushes there is never
//!   budgeted and never cancelled.
//!
//! A writer still waits. When the running request ends, the core takes what is
//! ready on either wire in an order of its own, so a push can wait for the
//! request being evaluated and for others queued ahead of it, each at most a
//! budget. Bounding it to one is up to whoever sends the requests: one at a
//! time.
//!
//! The hooks are globals a client can redefine. A server that has to hold its
//! budget against its own clients serves them in restricted mode.

use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rayforce_sys as sys;

use crate::env::{bind_unary, UnaryFn};
use crate::error::{code_of, ErrorCode, RayError, Result};
use crate::poll::Poll;
use crate::raw;
use crate::runtime::{assert_on_runtime_thread, eval, on_runtime_thread};
use crate::value::Value;

/// The native function the hooks hand each payload to, and the hooks' own
/// parameter. Both are globals of the runtime, so the names are ones no query
/// is likely to reach for: a query that read `servePayload` would see the
/// request instead of its own global.
const GUARD: &str = ".serve.guard";
const PAYLOAD: &str = "servePayload";

/// How often the watcher looks at the running request. A cancel lands at most
/// this late, plus however long the core takes to reach its next check.
const TICK: Duration = Duration::from_millis(10);

/// The guard serving IPC in this process, if any. One, because the core's hooks
/// are globals and the native function they call cannot carry state of its own.
static INSTALLED: Mutex<Option<Arc<Shared>>> = Mutex::new(None);

/// A lock that a panic elsewhere does not poison: everything behind these is
/// replaced whole, so there is no half-written state to protect.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The time a request's evaluation is measured in. Injected so that a test
/// can move it, rather than wait out a budget.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Instant;
}

/// The monotonic clock, which a server uses unless it is given another.
pub struct Monotonic;

impl Clock for Monotonic {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// How a request ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// Evaluated, and its value sent back.
    Answered,
    /// Evaluated to an error, which was sent back. A cancel is not one of these.
    Failed(ErrorCode),
    /// Stopped before it finished, and `cancel` sent back if anyone was there.
    Cancelled(By),
}

/// Who stopped a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// It ran past the server's budget.
    Budget,
    /// Its client closed the connection while it ran.
    Hangup,
    /// Its client asked, with the core's out-of-band cancel.
    Client,
}

/// One request the server evaluated, from when it started to when it ended.
///
/// A request its client cancelled while it was still queued never starts, so
/// it has no end here. The core's query log records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestEnd {
    /// The request as the client sent it, whole: verbatim when it was a
    /// string, formatted back to source otherwise.
    pub text: String,
    /// How long it was evaluated for, on the server's [`Clock`]. The time it
    /// waited behind other requests is not in it.
    pub duration: Duration,
    pub ended: Ended,
}

impl Poll {
    /// Serve Rayforce's IPC on `port`, every request guarded. Nothing is
    /// served until [`ServeIpc::start`].
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use rayforce::{Poll, Runtime};
    /// Runtime::scope(|_rt| {
    ///     let poll = Poll::install()?;
    ///     let ipc = poll.serve_ipc(5011).budget(Duration::from_secs(10)).start()?;
    ///     loop {
    ///         poll.run_for(100)?;   // requests are served in here
    ///         for end in ipc.drain() {
    ///             println!("{:?} in {:?}: {}", end.ended, end.duration, end.text);
    ///         }
    ///     }
    /// })?;
    /// # Ok::<(), rayforce::RayError>(())
    /// ```
    pub fn serve_ipc(&self, port: u16) -> ServeIpc<'_> {
        ServeIpc {
            poll: self,
            port,
            budget: None,
            clock: Arc::new(Monotonic),
        }
    }
}

/// What [`Poll::serve_ipc`] will serve, set up before it starts.
#[must_use = "nothing is served until `start`"]
pub struct ServeIpc<'p> {
    poll: &'p Poll,
    port: u16,
    budget: Option<Duration>,
    clock: Arc<dyn Clock>,
}

impl<'p> ServeIpc<'p> {
    /// The longest a request may evaluate before it is cancelled. Without
    /// one, a request runs until it ends, its client leaves, or its client
    /// cancels it.
    pub fn budget(mut self, budget: Duration) -> Self {
        self.budget = Some(budget);
        self
    }

    /// The clock the budget and each [`RequestEnd::duration`] are measured
    /// on. The monotonic clock by default.
    pub fn clock(mut self, clock: impl Clock) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    /// Listen on the port, install the guard and start its watcher.
    ///
    /// Refused when the budget is zero, which would cancel every request;
    /// when the port is 0 or cannot be bound; and when this process already
    /// serves IPC through here, because the core's hooks are one per process.
    pub fn start(self) -> Result<IpcServer<'p>> {
        assert_on_runtime_thread("ServeIpc::start");
        if !self.poll.is_current() {
            return Err(RayError::binding("IPC: the loop this was built on is gone"));
        }
        if self.budget == Some(Duration::ZERO) {
            return Err(RayError::binding(
                "IPC: a zero budget would cancel every request",
            ));
        }
        if self.port == 0 {
            return Err(RayError::binding("IPC: port 0 is not a port to serve"));
        }
        let shared = Arc::new(Shared {
            budget: self.budget,
            clock: self.clock,
            running: Mutex::new(None),
            ends: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        {
            let mut installed = lock(&INSTALLED);
            if installed.is_some() {
                return Err(RayError::binding(
                    "IPC: this process already serves IPC through serve_ipc",
                ));
            }
            *installed = Some(Arc::clone(&shared));
        }
        if let Err(error) = install_hooks().and_then(|()| listen(self.port)) {
            uninstall_hooks();
            return Err(error);
        }
        let watched = Arc::clone(&shared);
        let watcher = thread::Builder::new()
            .name("rayforce-ipc-guard".into())
            .spawn(move || watched.watch());
        let watcher = match watcher {
            Ok(watcher) => watcher,
            Err(error) => {
                uninstall_hooks();
                return Err(RayError::binding(format!(
                    "IPC: no watcher thread: {error}"
                )));
            }
        };
        Ok(IpcServer {
            port: self.port,
            shared,
            watcher: Some(watcher),
            _poll: PhantomData,
            _not_send: PhantomData,
        })
    }
}

/// Bind the native guard, then point both hooks at it. The core calls a hook
/// only when it is a Rayfall lambda (`hook_lookup`, src/core/ipc.c), so each is
/// a one-line lambda handing the payload on.
fn install_hooks() -> Result<()> {
    bind_unary::<Guard>(GUARD)?;
    for hook in [".ipc.on.sync", ".ipc.on.async"] {
        eval(&format!(
            "(set {hook} (fn [{PAYLOAD}] ({GUARD} {PAYLOAD})))"
        ))?;
    }
    Ok(())
}

/// The core's own listener. The bindings wrap the Q listener only, and Rayfall
/// is the documented way in for an embedder that owns the loop.
fn listen(port: u16) -> Result<()> {
    eval(&format!("(.sys.listen {port})")).map(drop)
}

/// Undo what `start` did. Best effort: a hook that cannot be deleted is still
/// unguarded once nothing is installed, because the guard then evaluates the
/// payload as the core would.
fn uninstall_hooks() {
    if on_runtime_thread() {
        for hook in [".ipc.on.sync", ".ipc.on.async"] {
            let _ = eval(&format!("(del {hook})"));
        }
    }
    *lock(&INSTALLED) = None;
}

/// Rayforce's IPC, served and guarded. See [`Poll::serve_ipc`].
///
/// Dropping it stops the watcher and removes the hooks, but the port goes on
/// serving, unguarded: as with [`crate::QListener`], the core offers no way to
/// stop listening. Keep it for as long as the loop runs.
///
/// `!Send`, like every handle on the loop.
pub struct IpcServer<'p> {
    port: u16,
    shared: Arc<Shared>,
    watcher: Option<JoinHandle<()>>,
    _poll: PhantomData<&'p Poll>,
    _not_send: PhantomData<*mut ()>,
}

impl IpcServer<'_> {
    /// The port being served.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The requests that ended since the last call, oldest first.
    pub fn drain(&self) -> Vec<RequestEnd> {
        std::mem::take(&mut *lock(&self.shared.ends))
    }
}

impl Drop for IpcServer<'_> {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
        uninstall_hooks();
    }
}

/// What the hooks, the watcher and the handle share.
struct Shared {
    budget: Option<Duration>,
    clock: Arc<dyn Clock>,
    /// The request being evaluated. Behind a lock the watcher takes too, so an
    /// interrupt can only land while a request is registered: `finish` clears
    /// the core's flag under the same lock, and a flag left set would cancel
    /// whatever the loop evaluates next, a write on the Q wire included.
    running: Mutex<Option<Running>>,
    ends: Mutex<Vec<RequestEnd>>,
    stop: AtomicBool,
}

struct Running {
    fd: i64,
    time_started: Instant,
    stopped_by: Option<By>,
}

impl Shared {
    /// Register the request about to be evaluated. False when one already is:
    /// a query that reaches the guard again from inside itself is part of the
    /// request that is running, not one of its own.
    fn begin(&self, fd: i64) -> bool {
        let mut running = lock(&self.running);
        if running.is_some() {
            return false;
        }
        *running = Some(Running {
            fd,
            time_started: self.clock.now(),
            stopped_by: None,
        });
        true
    }

    /// Unregister the request and keep how it ended.
    fn finish(&self, text: String, answer: &Value) {
        let Some(run) = ({
            let mut running = lock(&self.running);
            // SAFETY: the core's cancel flags are atomics, safe from any thread.
            unsafe { sys::ray_clear_interrupt() };
            running.take()
        }) else {
            return;
        };
        let duration = self.clock.now().saturating_duration_since(run.time_started);
        let raw = answer.as_ptr();
        // SAFETY: the answer is an owned core object, or null.
        let code = unsafe { (!raw.is_null() && raw::is_err(raw)).then(|| code_of(raw)) };
        let ended = match (code, run.stopped_by) {
            (Some(ErrorCode::Cancel), Some(by)) => Ended::Cancelled(by),
            (Some(ErrorCode::Cancel), None) => Ended::Cancelled(By::Client),
            (Some(code), _) => Ended::Failed(code),
            (None, _) => Ended::Answered,
        };
        lock(&self.ends).push(RequestEnd {
            text,
            duration,
            ended,
        });
    }

    fn watch(&self) {
        while !self.stop.load(Ordering::SeqCst) {
            self.check();
            thread::sleep(TICK);
        }
    }

    /// Stop the running request if it is past its budget or its client is gone.
    fn check(&self) {
        let mut running = lock(&self.running);
        let Some(run) = running.as_mut().filter(|run| run.stopped_by.is_none()) else {
            return;
        };
        let elapsed = self.clock.now().saturating_duration_since(run.time_started);
        let by = if self.budget.is_some_and(|budget| elapsed >= budget) {
            Some(By::Budget)
        } else if peer_gone(run.fd) {
            Some(By::Hangup)
        } else {
            None
        };
        if let Some(by) = by {
            run.stopped_by = Some(by);
            // SAFETY: documented as callable from a cancellation thread
            // (include/rayforce.h); it only stores to the core's atomics.
            unsafe { sys::ray_request_interrupt() };
        }
    }
}

/// Whether the client on `fd` has closed its end. The core's sockets are
/// non-blocking, so a peek with nothing to read is `WouldBlock`, and only an
/// orderly close reads as zero bytes. Called under the `running` lock, so the
/// loop's thread, busy with the request, cannot close the socket meanwhile.
#[cfg(unix)]
fn peer_gone(fd: i64) -> bool {
    use std::mem::ManuallyDrop;
    use std::net::TcpStream;
    use std::os::fd::FromRawFd;

    let Ok(fd) = i32::try_from(fd) else {
        return false;
    };
    if fd < 0 {
        return false;
    }
    // SAFETY: the socket stays the core's: ManuallyDrop never closes it.
    let sock = ManuallyDrop::new(unsafe { TcpStream::from_raw_fd(fd) });
    matches!(sock.peek(&mut [0u8; 1]), Ok(0))
}

#[cfg(not(unix))]
fn peer_gone(_fd: i64) -> bool {
    false
}

/// The native function the hooks call with each request's payload.
struct Guard;

impl UnaryFn for Guard {
    fn call(payload: Value) -> Value {
        let installed = lock(&INSTALLED).clone();
        let Some(shared) = installed else {
            return evaluate(&payload);
        };
        let registered = shared.begin(connection_fd());
        let answer = evaluate(&payload);
        if registered {
            shared.finish(text_of(&payload), &answer);
        }
        answer
    }
}

/// Whether `payload` is a string atom, which the core evaluates as source.
fn is_source(payload: &Value) -> bool {
    let raw = payload.as_ptr();
    // SAFETY: a live object the core handed the hook.
    !raw.is_null() && i64::from(unsafe { raw::type_code(raw) }) == -i64::from(sys::RAY_STR)
}

/// Evaluate a payload as the core does without a hook: a string as source, up
/// to its first NUL as the core's copy ends there, anything else as an
/// expression. An error comes back as the core's own object, for the client.
fn evaluate(payload: &Value) -> Value {
    let raw = if is_source(payload) {
        let text = payload.as_string().unwrap_or_default();
        let text = text.split('\0').next().unwrap_or_default();
        let source = CString::new(text).unwrap_or_default();
        // SAFETY: a NUL-terminated string, on the runtime's thread.
        unsafe { sys::ray_eval_str(source.as_ptr()) }
    } else {
        // SAFETY: a live object, on the runtime's thread.
        unsafe { sys::ray_eval(payload.as_ptr()) }
    };
    if raw.is_null() {
        Value::null()
    } else {
        // SAFETY: ray_eval and ray_eval_str hand back an owned reference.
        unsafe { Value::from_owned(raw) }
    }
}

/// The request as its client sent it: the source when it was a string, the
/// expression formatted back to source otherwise.
fn text_of(payload: &Value) -> String {
    if is_source(payload) {
        payload.as_string().unwrap_or_default()
    } else {
        payload.format()
    }
}

/// The socket of the connection whose request this is: `.ipc.handle` names
/// the connection while a hook runs, and the loop resolves it to its selector.
/// -1 when it cannot be found, which only costs the hangup check.
fn connection_fd() -> i64 {
    let Ok(handle) = eval("(.ipc.handle)").and_then(|v| v.as_i64()) else {
        return -1;
    };
    // SAFETY: the runtime's own poll, on its thread; a selector is read once,
    // while the loop is inside this request and cannot free it.
    unsafe {
        let poll = sys::ray_runtime_get_poll();
        if poll.is_null() {
            return -1;
        }
        let selector = sys::ray_poll_get(poll.cast(), handle);
        if selector.is_null() {
            -1
        } else {
            (*selector).fd
        }
    }
}
