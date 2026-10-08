//! `Poll::serve_ipc`: Rayforce's own IPC served from an embedding, each request
//! stopped when it runs past the budget or its client leaves, and each one's
//! end handed back whole. telmio3 TPR-161 and TPR-149.
//!
//! The server runs on the test thread; clients are raw sockets on helper
//! threads, for the reason `support::ipc` gives.

mod support;

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rayforce::{By, Clock, Ended, ErrorCode, Poll, Runtime};
use support::ipc::{self, Reply};
use support::wire::{char_vec, frame, long_atom, read_frame, RESPONSE, SYNC};

const BUDGET: Duration = Duration::from_secs(10);

/// Five columns of 20 M rows. Copying them forty times takes about 1.5 s on a
/// laptop, which is the stand-in for a query that runs too long. Slower
/// machines only make it longer.
const TABLE: &str = "(set t (table [a b c d e] (list (til 20000000) (til 20000000) \
                     (til 20000000) (til 20000000) (til 20000000))))";

/// Forty selects, each dropping one row so that each one copies the table.
fn slow() -> String {
    let mut q = "t".to_owned();
    for i in 0..40 {
        q = format!("(select {{from: {q} where: (!= a {i})}})");
    }
    format!("(count {q})")
}

/// How long a test lets a slow request get going before it acts on it.
const SETTLE: Duration = Duration::from_millis(200);

fn free_port() -> u16 {
    static NEXT: AtomicU16 = AtomicU16::new(0);
    loop {
        let port = 23000 + NEXT.fetch_add(1, Ordering::Relaxed);
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
}

/// Run `client` on a helper thread and the loop on this one until it is done.
fn with_client<T: Send + 'static>(poll: &Poll, client: impl FnOnce() -> T + Send + 'static) -> T {
    let handle = thread::spawn(client);
    while !handle.is_finished() {
        poll.run_for(20).unwrap();
    }
    handle.join().unwrap()
}

/// A clock the test moves by hand.
#[derive(Clone)]
struct Hand {
    base: Instant,
    ms: Arc<AtomicU64>,
}

impl Hand {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            ms: Arc::new(AtomicU64::new(0)),
        }
    }

    fn advance(&self, by: Duration) {
        self.ms.fetch_add(by.as_millis() as u64, Ordering::SeqCst);
    }
}

impl Clock for Hand {
    fn now(&self) -> Instant {
        self.base + Duration::from_millis(self.ms.load(Ordering::SeqCst))
    }
}

fn ended(ends: &[rayforce::RequestEnd]) -> Vec<Ended> {
    ends.iter().map(|e| e.ended).collect()
}

#[test]
fn a_request_is_answered_and_handed_back_whole() {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        let ipc = poll.serve_ipc(free_port()).budget(BUDGET).start()?;
        let port = ipc.port();
        // Longer than the 256 characters the core's query log keeps.
        let text = format!(";; {}\n(+ 1 2)", "x".repeat(300));
        let sent = text.clone();
        assert_eq!(
            with_client(&poll, move || ipc::ask(port, &sent)),
            Reply::Long(3)
        );

        let ends = ipc.drain();
        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].text, text);
        assert_eq!(ends[0].ended, Ended::Answered);
        assert!(ipc.drain().is_empty(), "an end is handed back once");
        Ok(())
    })
    .unwrap();
}

#[test]
fn an_error_is_answered_and_handed_back_as_failed() {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        let ipc = poll.serve_ipc(free_port()).budget(BUDGET).start()?;
        let port = ipc.port();
        let reply = with_client(&poll, move || ipc::ask(port, "(no_such_name 1)"));
        assert_eq!(reply, Reply::Error("name".into()));
        assert_eq!(ended(&ipc.drain()), [Ended::Failed(ErrorCode::Name)]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_request_past_its_budget_is_cancelled() {
    Runtime::scope(|rt| {
        rt.eval(TABLE)?;
        let poll = Poll::install()?;
        let hand = Hand::new();
        let ipc = poll
            .serve_ipc(free_port())
            .budget(BUDGET)
            .clock(hand.clone())
            .start()?;
        let port = ipc.port();
        let reply = with_client(&poll, move || {
            let mut sock = ipc::connect(port);
            ipc::send(&mut sock, &slow());
            thread::sleep(SETTLE);
            hand.advance(BUDGET);
            ipc::reply(&mut sock)
        });
        assert_eq!(reply, Reply::Error("cancel".into()));

        let ends = ipc.drain();
        assert_eq!(ended(&ends), [Ended::Cancelled(By::Budget)]);
        assert!(ends[0].duration >= BUDGET, "measured on the server's clock");
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_request_whose_client_hangs_up_is_cancelled() {
    Runtime::scope(|rt| {
        rt.eval(TABLE)?;
        let poll = Poll::install()?;
        let ipc = poll.serve_ipc(free_port()).budget(BUDGET).start()?;
        let port = ipc.port();
        let probe = with_client(&poll, move || {
            let mut sock = ipc::connect(port);
            ipc::send(&mut sock, &slow());
            thread::sleep(SETTLE);
            drop(sock);
            ipc::ask(port, "(+ 1 2)")
        });
        assert_eq!(probe, Reply::Long(3));
        assert_eq!(
            ended(&ipc.drain()),
            [Ended::Cancelled(By::Hangup), Ended::Answered]
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_cancel_from_the_client_is_reported_as_the_clients() {
    Runtime::scope(|rt| {
        rt.eval(TABLE)?;
        let poll = Poll::install()?;
        let ipc = poll.serve_ipc(free_port()).budget(BUDGET).start()?;
        let port = ipc.port();
        let reply = with_client(&poll, move || {
            let mut sock = ipc::connect(port);
            ipc::send(&mut sock, &slow());
            thread::sleep(SETTLE);
            ipc::cancel(&sock);
            ipc::reply(&mut sock)
        });
        assert_eq!(reply, Reply::Error("cancel".into()));
        assert_eq!(ended(&ipc.drain()), [Ended::Cancelled(By::Client)]);
        Ok(())
    })
    .unwrap();
}

/// The writer's side. A push on the Q wire never passes through the guard, so
/// nothing cancels it. It still waits for the query being evaluated, and maybe
/// for one queued behind it too: when the running query ends, the core takes
/// what is ready on either wire in an order of its own, which took the queued
/// query first in about one run in four. So the bound is the queries ahead of
/// the push, each at most a budget, not the running one alone.
#[test]
fn a_push_is_answered_once_the_queries_ahead_of_it_end() {
    // Room for the push's own round trip after the queued query's answer.
    const SLACK: Duration = Duration::from_millis(250);
    Runtime::scope(|rt| {
        rt.eval(TABLE)?;
        let poll = Poll::install()?;
        let ipc = poll.serve_ipc(free_port()).start()?;
        let listener = poll.serve_q(free_port())?;
        let (ipc_port, q_port) = (ipc.port(), listener.port());
        let (first, second, push, reply) = with_client(&poll, move || {
            let start = Instant::now();
            let query = move || {
                ipc::ask(ipc_port, &slow());
                start.elapsed()
            };
            let first = thread::spawn(query);
            thread::sleep(SETTLE);
            let second = thread::spawn(query);
            thread::sleep(SETTLE);
            let mut writer = TcpStream::connect(("127.0.0.1", q_port)).unwrap();
            writer.write_all(&[3, 0]).unwrap();
            std::io::Read::read_exact(&mut writer, &mut [0u8; 1]).unwrap();
            writer
                .write_all(&frame(SYNC, &char_vec("(+ 1 1)")))
                .unwrap();
            let reply = read_frame(&mut writer).expect("the push is answered");
            let push = start.elapsed();
            (first.join().unwrap(), second.join().unwrap(), push, reply)
        });
        assert_eq!(reply, (RESPONSE, long_atom(2)), "answered, not cancelled");
        assert!(
            push >= first,
            "the push waits for the query being evaluated: {push:?} against {first:?}"
        );
        assert!(
            push <= second + SLACK,
            "and at most for the one queued ahead of it: {push:?} against {second:?}"
        );
        assert_eq!(ended(&ipc.drain()), [Ended::Answered, Ended::Answered]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_zero_budget_is_refused() {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        assert!(poll
            .serve_ipc(free_port())
            .budget(Duration::ZERO)
            .start()
            .is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_second_server_in_one_process_is_refused() {
    Runtime::scope(|_rt| {
        let poll = Poll::install()?;
        let _first = poll.serve_ipc(free_port()).start()?;
        assert!(
            poll.serve_ipc(free_port()).start().is_err(),
            "the core's hooks are one per process"
        );
        Ok(())
    })
    .unwrap();
}
