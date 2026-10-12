//! The q listener: `Poll::serve_q` accepts q peers on a port, evaluates what
//! they send synchronously, and dispatches what they push asynchronously into
//! the runtime's environment. This is what turns a runtime into an RDB that a q
//! publisher can write into.
//!
//! Driven by raw TCP peers on helper threads. A `QConnection` on the runtime
//! thread could not be the peer: its `execute` blocks for a response that only
//! the poll can produce, and the poll runs on the same thread.

mod support;

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use rayforce::{Poll, Runtime};
use support::wire::{
    char_vec, dict, frame, list, long_atom, long_vec, read_frame, sym_atom, sym_vec, ASYNC,
    RESPONSE, SYNC,
};

/// A port for a listener. `q_serve` refuses 0, so the number has to be
/// chosen here: a counter over a range below every ephemeral range, which no
/// two calls in this process can share, and a bind to skip a port some other
/// process on the box already holds.
fn free_port() -> u16 {
    static NEXT: AtomicU16 = AtomicU16::new(0);
    const BASE: u16 = 21000;
    loop {
        let port = BASE + NEXT.fetch_add(1, Ordering::Relaxed);
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
}

/// Pump the loop until `f` yields or the budget runs out.
fn pump_until<T>(poll: &Poll, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    for _ in 0..100 {
        if let Some(v) = f() {
            return Some(v);
        }
        poll.run_for(20).unwrap();
    }
    f()
}

/// Connect and log in anonymously on a helper thread, handing the socket back
/// once the listener has answered the handshake. On a thread because that
/// answer only comes once the poll has run, which the test does meanwhile.
fn connect(port: u16) -> Receiver<TcpStream> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("the port is served");
        // Anonymous login: an empty name, capability byte 3, NUL.
        sock.write_all(&[3, 0]).unwrap();
        let mut cap = [0u8; 1];
        std::io::Read::read_exact(&mut sock, &mut cap).expect("the handshake is answered");
        let _ = tx.send(sock);
    });
    rx
}

/// Send one SYNC frame and collect its RESPONSE on a helper thread, for the
/// same reason as `connect`.
fn send_sync(sock: &TcpStream, body: Vec<u8>) -> Receiver<(u8, Vec<u8>)> {
    let (tx, rx) = mpsc::channel();
    let mut sock = sock.try_clone().unwrap();
    thread::spawn(move || {
        sock.write_all(&frame(SYNC, &body)).unwrap();
        if let Some(reply) = read_frame(&mut sock) {
            let _ = tx.send(reply);
        }
    });
    rx
}

#[test]
fn a_sync_string_is_evaluated_and_answered() {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        let port = free_port();
        let _listener = poll.serve_q(port)?;

        let peer = connect(port);
        let sock = pump_until(&poll, || peer.try_recv().ok()).expect("logged in");
        let reply = send_sync(&sock, char_vec("(+ 1 2)"));
        let (msgtype, body) = pump_until(&poll, || reply.try_recv().ok()).expect("answered");

        assert_eq!(msgtype, RESPONSE);
        assert_eq!(body, long_atom(3));
        Ok(())
    })
    .unwrap();
}

/// The contract an RDB is built on: a q publisher pushes `(upd; dict)` and a
/// function defined in Rayfall receives the dict, keys and tables intact.
#[test]
fn an_async_push_reaches_a_rayfall_function() {
    Runtime::scope(|rt| {
        rt.eval("(set upd (fn [p] (set got p)))")?;
        let poll = Poll::install()?;
        let port = free_port();
        let _listener = poll.serve_q(port)?;

        let peer = connect(port);
        let mut sock = pump_until(&poll, || peer.try_recv().ok()).expect("logged in");
        let payload = dict(sym_vec(&["trade"]), list(&[long_vec(&[10, 11, 12])]));
        sock.write_all(&frame(ASYNC, &list(&[sym_atom("upd"), payload])))
            .unwrap();

        let got = pump_until(&poll, || rayforce::get_global("got").ok()).expect("upd ran");
        assert!(got.is_dict(), "got {}", got.format());
        assert_eq!(got.dict_keys()?.get(0)?.as_sym()?, "trade");
        assert_eq!(got.dict_values()?.get(0)?.as_slice::<i64>()?, &[10, 11, 12]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn port_zero_is_refused() {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        assert!(poll.serve_q(0).is_err());
        Ok(())
    })
    .unwrap();
}
