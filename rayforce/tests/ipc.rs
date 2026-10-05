//! Phase 8: IPC client against a spawned `rayforce` server.
//!
//! Mirrors the Python suite: spawn the `rayforce` binary in server-only mode on
//! a free port, connect, and exchange queries. Skips if no binary is available.

use rayforce::{ErrorCode, Runtime, TcpClient};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

fn binary_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RAYFORCE_BINARY") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    let home = std::env::var("HOME").ok()?;
    let pb = PathBuf::from(home).join("rayforce/rayforce");
    pb.is_file().then_some(pb)
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Spawn the server and wait until the port accepts connections.
fn spawn_server(bin: &PathBuf, port: u16) -> Server {
    let child = Command::new(bin)
        .arg("-p")
        .arg(port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn rayforce server");
    let mut server = Server(child);
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return server;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // Drop kills + waits the child before we panic.
    let _ = server.0.kill();
    let _ = server.0.wait();
    panic!("server did not become reachable on port {port}");
}

/// Whether a missing server binary is a hard error rather than a skip.
///
/// Skipping reports as a pass, which is indistinguishable from having run —
/// so on its own it lets this file's coverage lapse unnoticed, and this is the
/// only file that drives [`TcpClient`] against a real server. CI sets
/// `RAYFORCE_REQUIRE_SERVER=1` after building the binary, so a path that stops
/// resolving fails the job instead of quietly going green.
fn server_required() -> bool {
    matches!(std::env::var("RAYFORCE_REQUIRE_SERVER"), Ok(v) if !v.is_empty() && v != "0")
}

macro_rules! require_binary {
    () => {
        match binary_path() {
            Some(b) => b,
            None if server_required() => panic!(
                "RAYFORCE_REQUIRE_SERVER is set but no rayforce binary was found; \
                 point RAYFORCE_BINARY at one or run `make release` in the core checkout"
            ),
            None => {
                let _ = writeln!(std::io::stderr(), "skipping IPC test: no rayforce binary");
                return;
            }
        }
    };
}

#[test]
fn client_executes_arithmetic() {
    let bin = require_binary!();
    Runtime::scope(|_rt| {
        let port = free_port();
        let _server = spawn_server(&bin, port);

        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        let r = client.execute("(+ 1 2)").unwrap();
        assert_eq!(r.as_i64().unwrap(), 3);
        Ok(())
    })
    .unwrap();
}

#[test]
fn client_roundtrips_vector() {
    let bin = require_binary!();
    Runtime::scope(|_rt| {
        let port = free_port();
        let _server = spawn_server(&bin, port);

        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        let r = client.execute("(til 5)").unwrap();
        assert_eq!(r.as_slice::<i64>().unwrap(), &[0, 1, 2, 3, 4]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn client_reports_server_error() {
    let bin = require_binary!();
    Runtime::scope(|_rt| {
        let port = free_port();
        let _server = spawn_server(&bin, port);

        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        assert!(client.execute("(undefined_symbol_xyz)").is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn connect_failure_names_its_cause() {
    Runtime::scope(|_rt| {
        // free_port() hands back a port it has already released, so nothing is
        // listening and the kernel answers ECONNREFUSED — the one connect
        // failure reachable without a peer to misbehave for us.
        let port = free_port();
        let e = TcpClient::connect("127.0.0.1", port, "", "")
            .err()
            .expect("connect should have failed");
        let msg = e.to_string();
        assert!(
            msg.contains("connection refused"),
            "expected a named cause, got {msg:?}"
        );
        assert!(
            msg.contains(&port.to_string()),
            "message lost the port: {msg:?}"
        );
        Ok(())
    })
    .unwrap();
}

/// Bind a listener that completes the TCP accept, reads the client's two-byte
/// handshake, and answers with a wire version the core cannot speak. Returns
/// the port. `RAY_SERDE_WIRE_VERSION` is 3, so 0xFF is reliably wrong.
fn spawn_wrong_version_peer() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let mut hs = [0u8; 2];
            let _ = sock.read_exact(&mut hs);
            let _ = sock.write_all(&[0xFFu8, 0x00]);
        }
    });
    port
}

#[test]
fn connect_reports_a_wire_version_mismatch() {
    // Distinct from a refusal: the peer is listening and answers, it just
    // speaks a protocol this build would misparse every atom of. Collapsing
    // the two into "failed" is what this test exists to prevent.
    Runtime::scope(|_rt| {
        let port = spawn_wrong_version_peer();
        let e = TcpClient::connect("127.0.0.1", port, "", "")
            .err()
            .expect("connect should have failed");
        let msg = e.to_string();
        assert!(
            msg.contains("wire version mismatch"),
            "expected a wire-version cause, got {msg:?}"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_client_is_closed_before_its_scope_ends() {
    // The hole no generation check ever covered: `TcpClient::drop` calls
    // `ray_ipc_close`, which reaches into the engine, and nothing ordered it
    // against the unmap. The scope does: the closure's locals are dropped
    // before it returns, and only then is the runtime torn down. Left implicit
    // on purpose — an explicit `drop(client)` would not test the ordering.
    let bin = require_binary!();
    let port = free_port();
    let _server = spawn_server(&bin, port);

    Runtime::scope(|_rt| {
        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        assert_eq!(client.execute("(+ 1 2)").unwrap().as_i64().unwrap(), 3);
        Ok(())
    })
    .unwrap();

    // The close ran against a mapped heap, so the next scope starts cleanly.
    Runtime::scope(|rt| rt.eval("1")?.as_i64()).unwrap();
}

/// Bind a listener that completes the handshake as a server without
/// authentication does — `RAY_SERDE_WIRE_VERSION` (3), then no auth required —
/// and then swallows every request without answering, until the client hangs
/// up. Returns the port.
fn spawn_silent_peer() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let mut hs = [0u8; 2];
            if sock.read_exact(&mut hs).is_ok() && sock.write_all(&[3u8, 0x00]).is_ok() {
                let _ = std::io::copy(&mut sock, &mut std::io::sink());
            }
        }
    });
    port
}

#[test]
fn send_timeout_answers_within_its_budget() {
    let bin = require_binary!();
    Runtime::scope(|_rt| {
        let port = free_port();
        let _server = spawn_server(&bin, port);

        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        let r = client
            .execute_timeout("(+ 1 2)", Duration::from_secs(5))
            .unwrap();
        assert_eq!(r.as_i64().unwrap(), 3);
        Ok(())
    })
    .unwrap();
}

#[test]
fn send_timeout_expires_and_closes_the_client() {
    Runtime::scope(|_rt| {
        let port = spawn_silent_peer();
        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();

        let budget = Duration::from_millis(200);
        let started = Instant::now();
        let e = client
            .execute_timeout("(+ 1 2)", budget)
            .expect_err("a peer that never answers should time out");
        let waited = started.elapsed();
        assert_eq!(e.code, ErrorCode::Io, "{e}");
        assert!(e.message.contains("timed out"), "{e}");
        assert!(waited >= budget, "gave up early, after {waited:?}");
        assert!(waited < Duration::from_secs(5), "overran, {waited:?}");

        // Closed for good: no second exchange, and no wait for one.
        let started = Instant::now();
        assert!(client.execute("(+ 1 2)").is_err());
        assert!(started.elapsed() < budget);
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_zero_send_timeout_is_refused() {
    // The core reads a 0 ms timeout as no deadline, so passing it through would
    // turn "give up at once" into "wait forever" — against this peer, a hang.
    Runtime::scope(|_rt| {
        let port = spawn_silent_peer();
        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        let e = client
            .execute_timeout("(+ 1 2)", Duration::ZERO)
            .expect_err("a zero timeout should be refused");
        assert!(e.message.contains("greater than zero"), "{e}");
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_server_error_leaves_the_connection_open() {
    // A failed send closes the client only when the core closed the connection.
    // An error the server answers with is not that, whatever it says.
    let bin = require_binary!();
    Runtime::scope(|_rt| {
        let port = free_port();
        let _server = spawn_server(&bin, port);

        let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        assert!(client
            .execute_timeout("(undefined_symbol_xyz)", Duration::from_secs(5))
            .is_err());
        assert_eq!(client.execute("(+ 1 2)").unwrap().as_i64().unwrap(), 3);
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_timed_out_client_does_not_reach_the_next_connection() {
    // The expiry frees the client's slot in the poll, and the next connection
    // takes it, so the stale client's handle now names that one. Sending on
    // it would deliver to the wrong server, and dropping it would close a
    // connection someone else holds.
    let bin = require_binary!();
    let port = free_port();
    let _server = spawn_server(&bin, port);

    Runtime::scope(|_rt| {
        let silent = spawn_silent_peer();
        let stale = TcpClient::connect("127.0.0.1", silent, "", "").unwrap();
        assert!(stale
            .execute_timeout("(+ 1 2)", Duration::from_millis(100))
            .is_err());

        let live = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
        assert!(stale.execute("(+ 1 2)").is_err());
        drop(stale);
        assert_eq!(live.execute("(+ 1 2)").unwrap().as_i64().unwrap(), 3);
        Ok(())
    })
    .unwrap();
}

#[test]
fn a_client_whose_server_died_does_not_reach_the_next_connection() {
    // The same hazard without a deadline: when the peer is gone, the exchange
    // reads its EOF and the core deregisters the connection, freeing the slot.
    // (If the write fails first instead, the slot stays registered and still
    // ours, and the drop below closes it — either way nothing crosses over.)
    let bin = require_binary!();
    let second = free_port();
    let _second = spawn_server(&bin, second);

    Runtime::scope(|_rt| {
        let first = free_port();
        let server = spawn_server(&bin, first);
        let stale = TcpClient::connect("127.0.0.1", first, "", "").unwrap();
        drop(server);
        assert!(stale.execute("(+ 1 2)").is_err());

        let live = TcpClient::connect("127.0.0.1", second, "", "").unwrap();
        assert!(stale.execute("(+ 1 2)").is_err());
        drop(stale);
        assert_eq!(live.execute("(+ 1 2)").unwrap().as_i64().unwrap(), 3);
        Ok(())
    })
    .unwrap();
}
