//! Poll-driven Q subscriptions: attach a connection to the event loop, bind a
//! native handler, and receive frames the peer pushes unsolicited.
//!
//! Driven by a raw-TCP mock that speaks the Q wire protocol, so no `q` binary
//! and no `rayforce -q` server is needed. Wire builders mirror `tests/q.rs`.

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use rayforce::{env, q::QConnection, Poll, Runtime, Value};

// --- Q wire-format builders (server side) ---------------------------------

/// Symbol atom: type -11, NUL-terminated name.
fn sym_atom(s: &str) -> Vec<u8> {
    let mut b = vec![(-11i8) as u8];
    b.extend_from_slice(s.as_bytes());
    b.push(0);
    b
}

/// Symbol vector: type 11, attrs, int32 len, NUL-terminated names.
fn sym_vec(syms: &[&str]) -> Vec<u8> {
    let mut b = vec![11u8, 0u8];
    b.extend_from_slice(&(syms.len() as i32).to_le_bytes());
    for s in syms {
        b.extend_from_slice(s.as_bytes());
        b.push(0);
    }
    b
}

/// Long (i64) vector: type 7, attrs, int32 len, data.
fn long_vec(vals: &[i64]) -> Vec<u8> {
    let mut b = vec![7u8, 0u8];
    b.extend_from_slice(&(vals.len() as i32).to_le_bytes());
    for v in vals {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// General list: type 0, attrs, int32 len, then the elements.
fn list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0u8, 0u8];
    b.extend_from_slice(&(items.len() as i32).to_le_bytes());
    for it in items {
        b.extend_from_slice(it);
    }
    b
}

/// Dictionary: type 99, keys object, values object.
fn dict(keys: Vec<u8>, vals: Vec<u8>) -> Vec<u8> {
    let mut b = vec![99u8];
    b.extend(keys);
    b.extend(vals);
    b
}

const ASYNC: u8 = 0;
const RESPONSE: u8 = 2;

/// Wrap a serialized object in the 8-byte Q wire header.
fn frame(msgtype: u8, body: &[u8]) -> Vec<u8> {
    let size = (8 + body.len()) as u32;
    let mut b = vec![1u8, msgtype, 0u8, 0u8]; // little-endian, uncompressed
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(body);
    b
}

/// Read one complete wire message and discard it. False if the peer hung up.
fn drain_one(sock: &mut std::net::TcpStream) -> bool {
    let mut hdr = [0u8; 8];
    if sock.read_exact(&mut hdr).is_err() {
        return false;
    }
    let size = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let mut body = vec![0u8; size.saturating_sub(8)];
    sock.read_exact(&mut body).is_ok()
}

/// A publisher: handshake, answer one sync request, then push `pushes`.
///
/// When `close_after` is `None` it lingers until the *client* hangs up, which
/// is how a real tickerplant behaves. Passing a duration makes it drop the
/// connection instead — that is the only way to exercise disconnect detection.
///
/// The distinction matters: a peer that closes in the same breath as its
/// response leaves `q_conn_send` reporting `connection closed` rather than the
/// response it had already received, which is a race no live peer creates.
fn spawn_publisher(
    ack: Vec<u8>,
    pushes: Vec<Vec<u8>>,
    close_after: Option<std::time::Duration>,
) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            // Handshake: the client sends "user:pass" + capability + NUL (just
            // {0x03, 0x00} when anonymous); the server replies with one byte.
            let mut byte = [0u8; 1];
            while sock.read_exact(&mut byte).is_ok() && byte[0] != 0 {}
            let _ = sock.write_all(&[3u8]);

            // One sync request (the subscribe) -> one RESPONSE.
            if !drain_one(&mut sock) {
                return;
            }
            let _ = sock.write_all(&frame(RESPONSE, &ack));

            for p in &pushes {
                if sock.write_all(&frame(ASYNC, p)).is_err() {
                    return;
                }
            }

            match close_after {
                Some(d) => thread::sleep(d),
                // Block until the client goes away, so we never close first.
                None => {
                    let mut sink = [0u8; 64];
                    while matches!(sock.read(&mut sink), Ok(n) if n > 0) {}
                }
            }
            // Drop `sock`, closing the connection.
        }
    });
    port
}

// --- Handlers -------------------------------------------------------------
//
// A handler is bound as a *type*: the core takes a bare C function pointer with
// no user-data argument, so there is nowhere to put a closure's captures, and
// state lives in statics instead.

static UPD_CALLS: AtomicUsize = AtomicUsize::new(0);
static UPD_LAST_ARITY: AtomicI64 = AtomicI64::new(-1);

struct OnUpd;
impl env::VaryFn for OnUpd {
    fn call(args: env::Args<'_>) -> Value {
        UPD_CALLS.fetch_add(1, Ordering::SeqCst);
        UPD_LAST_ARITY.store(args.len() as i64, Ordering::SeqCst);
        Value::null()
    }
}

static UNARY_CALLS: AtomicUsize = AtomicUsize::new(0);

struct OnEod;
impl env::UnaryFn for OnEod {
    fn call(_arg: Value) -> Value {
        UNARY_CALLS.fetch_add(1, Ordering::SeqCst);
        Value::null()
    }
}

/// Pump the loop until `f` holds or the budget runs out.
fn pump_until(poll: &Poll, mut f: impl FnMut() -> bool) -> bool {
    for _ in 0..100 {
        if f() {
            return true;
        }
        poll.run_for(20).unwrap();
    }
    f()
}

// --- Tests ----------------------------------------------------------------

#[test]
fn install_adopts_an_existing_poll() {
    Runtime::scope(|_rt| {
        let first = Poll::install().unwrap();
        // A second handle must adopt the runtime's poll, not replace it — otherwise
        // it would strand every selector already registered on the first.
        let second = Poll::install().unwrap();
        second.run_for(1).unwrap();
        first.run_for(1).unwrap();
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_second_install_survives_the_first_being_dropped() {
    // Both handles name the runtime's poll; neither owns it. The first handle
    // used to run `ray_poll_destroy` on the way out, leaving the second
    // pointing at freed memory. `Poll` has no `Drop` at all now, which is what
    // makes that unreachable rather than merely unlikely.
    Runtime::scope(|_rt| {
        let second = {
            let first = Poll::install()?;
            assert!(first.run_for(1).is_ok());
            Poll::install()?
        };
        assert!(second.run_for(1).is_ok());
        Ok(())
    })
    .unwrap();
}

thread_local! {
    /// The one way a handle gets out of a scope. A closure that stashes into a
    /// `thread_local!` captures nothing, so it still satisfies
    /// `Runtime::scope`'s `Send` bound — the hole documented on `scope` itself.
    static ESCAPED: RefCell<Option<Poll>> = const { RefCell::new(None) };
}

#[test]
fn a_poll_smuggled_out_of_its_scope_is_inert() {
    // Returning a `Poll` from the closure does not compile — that is the
    // `compile_fail` doctest on `Poll`. This is the case the type system cannot
    // see, so it is the one `is_current()` exists for: the handle survives, and
    // every method on it must refuse rather than touch a destroyed loop.
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        ESCAPED.with(|e| *e.borrow_mut() = Some(poll));
        Ok(())
    })
    .unwrap();

    ESCAPED.with(|e| {
        let e = e.borrow();
        let poll = e.as_ref().expect("stashed while the scope was open");
        assert!(!poll.is_current());
        assert!(poll.run_for(1).is_err());
        poll.exit(0); // must not touch the freed loop
    });
}

#[test]
fn install_needs_a_live_runtime() {
    assert!(!rayforce::on_runtime_thread());
    assert!(Poll::install().is_err());
}

#[test]
fn bound_functions_are_callable_by_name() {
    Runtime::scope(|_rt| {
        UPD_CALLS.store(0, Ordering::SeqCst);
        UNARY_CALLS.store(0, Ordering::SeqCst);

        env::bind_vary::<OnUpd>("t_upd").unwrap();
        env::bind_unary::<OnEod>("t_eod").unwrap();

        rayforce::eval("(t_upd 1 2 3)").unwrap();
        assert_eq!(UPD_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(UPD_LAST_ARITY.load(Ordering::SeqCst), 3);

        rayforce::eval("(t_eod 1)").unwrap();
        assert_eq!(UNARY_CALLS.load(Ordering::SeqCst), 1);
        Ok(())
    })
    .unwrap();
}

#[test]
fn subscription_receives_pushed_frames() {
    Runtime::scope(|_rt| {
        UPD_CALLS.store(0, Ordering::SeqCst);
        UPD_LAST_ARITY.store(-1, Ordering::SeqCst);

        // The dict-form shape: one argument, a dict keyed by table name.
        let payload = dict(sym_vec(&["trade"]), list(&[long_vec(&[10, 11, 12])]));
        let push = list(&[sym_atom("upd"), payload.clone()]);
        let ack = dict(sym_vec(&["trade"]), list(&[long_vec(&[])]));
        let port = spawn_publisher(ack, vec![push.clone(), push.clone(), push], None);

        let poll = Poll::install().unwrap();
        env::bind_vary::<OnUpd>("upd").unwrap();

        let sub = QConnection::connect("127.0.0.1", port)
            .unwrap()
            .attach(&poll)
            .unwrap();

        // The ack comes back as a dict in the same shape as a push.
        let reply = sub.execute(".net.sub[0]").unwrap();
        assert!(
            reply.is_dict(),
            "ack should be a dict, got {}",
            reply.format()
        );

        // Pushes may already have been dispatched from inside `execute`, which
        // pumps the connection while it waits rather than swallowing frames.
        assert!(
            pump_until(&poll, || UPD_CALLS.load(Ordering::SeqCst) >= 3),
            "expected 3 pushed batches, saw {}",
            UPD_CALLS.load(Ordering::SeqCst)
        );
        // One argument, not two: the dict form.
        assert_eq!(UPD_LAST_ARITY.load(Ordering::SeqCst), 1);
        Ok(())
    })
    .unwrap();
}

#[test]
fn subscription_notices_the_peer_going_away() {
    Runtime::scope(|_rt| {
        UPD_CALLS.store(0, Ordering::SeqCst);

        let push = list(&[sym_atom("upd"), long_vec(&[1])]);
        // Close 300 ms after the push, well after `execute` has its response.
        let port = spawn_publisher(
            long_vec(&[]),
            vec![push],
            Some(std::time::Duration::from_millis(300)),
        );

        let poll = Poll::install().unwrap();
        env::bind_vary::<OnUpd>("upd").unwrap();

        let sub = QConnection::connect("127.0.0.1", port)
            .unwrap()
            .attach(&poll)
            .unwrap();
        sub.execute(".u.sub[`trade;`]").unwrap();
        assert!(sub.is_alive());

        // The publisher closes after its last push. A disconnect is not an error
        // and does not interrupt the loop — the selector just stops resolving.
        assert!(
            pump_until(&poll, || !sub.is_alive()),
            "subscription still reports alive after the peer closed"
        );
        // Sending on a dead subscription errors rather than hanging.
        assert!(sub.execute("1+1").is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_stale_subscription_does_not_close_its_successor() {
    // `ray_poll_register` hands out the first free slot, so the id of a
    // connection that just went away goes straight to the next attach. A
    // `Subscription` that compared ids alone would report the dead peer alive,
    // and its `Drop` would then close the live connection that inherited the
    // slot. Deliberately *not* calling `first.is_alive()` before the second
    // attach: that is the reconnect loop this bug is reachable from.
    Runtime::scope(|_rt| {
        let poll = Poll::install().unwrap();
        let ack = dict(sym_vec(&["trade"]), list(&[long_vec(&[])]));

        let port1 = spawn_publisher(ack.clone(), vec![], Some(Duration::from_millis(50)));
        let first = QConnection::connect("127.0.0.1", port1)
            .unwrap()
            .attach(&poll)
            .unwrap();
        first.execute(".u.sub[`trade;`]").unwrap();

        // Let the peer go away and the rx machine deregister its selector.
        assert!(
            pump_until(&poll, || !first.is_alive()),
            "publisher 1 should have gone away"
        );

        let port2 = spawn_publisher(ack, vec![], None);
        let second = QConnection::connect("127.0.0.1", port2)
            .unwrap()
            .attach(&poll)
            .unwrap();

        assert!(!first.is_alive(), "the dead peer must not report alive");
        drop(first);
        assert!(
            second.is_alive(),
            "dropping the stale handle must not close the live connection"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn binding_a_handler_needs_a_live_runtime() {
    assert!(!rayforce::on_runtime_thread());
    assert!(env::bind_vary::<OnUpd>("upd_no_rt").is_err());
    assert!(env::bind_unary::<OnEod>("eod_no_rt").is_err());
}

#[test]
fn an_empty_binding_name_is_rejected() {
    Runtime::scope(|_rt| {
        assert!(env::bind_vary::<OnUpd>("").is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn dropping_a_subscription_leaves_the_poll_usable() {
    Runtime::scope(|_rt| {
        let port = spawn_publisher(long_vec(&[]), vec![], None);
        let poll = Poll::install().unwrap();
        let sub = QConnection::connect("127.0.0.1", port)
            .unwrap()
            .attach(&poll)
            .unwrap();
        sub.execute("1+1").unwrap();
        drop(sub);
        // Dropping deregisters the selector; the poll must survive it.
        poll.run_for(10).unwrap();
        Ok(())
    })
    .unwrap();
}

#[test]
fn attaching_leaves_the_next_scope_startable() {
    // `attach` forgets the `QConnection` to suppress its `q_close`, handing the
    // fd to the poll. Get the ownership transfer wrong and the selector or the
    // fd outlives the scope, and the teardown on the way out — the poll first,
    // then the heap — leaves the engine unusable. Nothing reports it at the
    // time; it surfaces as the *next* scope failing to start.
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        let ack = dict(sym_vec(&["trade"]), list(&[long_vec(&[])]));
        let port = spawn_publisher(ack, vec![], Some(Duration::from_millis(50)));
        let sub = QConnection::connect("127.0.0.1", port)?.attach(&poll)?;
        drop(sub);
        Ok(())
    })
    .unwrap();

    Runtime::scope(|rt| rt.eval("1")?.as_i64()).unwrap();
}
