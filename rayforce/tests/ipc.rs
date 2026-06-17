//! Phase 8: IPC client against a spawned `rayforce` server.
//!
//! Mirrors the Python suite: spawn the `rayforce` binary in server-only mode on
//! a free port, connect, and exchange queries. Skips if no binary is available.

use rayforce::{Runtime, TcpClient};
use std::io::Write;
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

macro_rules! require_binary {
    () => {
        match binary_path() {
            Some(b) => b,
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
    let _rt = Runtime::new().unwrap();
    let port = free_port();
    let _server = spawn_server(&bin, port);

    let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
    let r = client.execute("(+ 1 2)").unwrap();
    assert_eq!(r.as_i64().unwrap(), 3);
}

#[test]
fn client_roundtrips_vector() {
    let bin = require_binary!();
    let _rt = Runtime::new().unwrap();
    let port = free_port();
    let _server = spawn_server(&bin, port);

    let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
    let r = client.execute("(til 5)").unwrap();
    assert_eq!(r.as_slice::<i64>().unwrap(), &[0, 1, 2, 3, 4]);
}

#[test]
fn client_reports_server_error() {
    let bin = require_binary!();
    let _rt = Runtime::new().unwrap();
    let port = free_port();
    let _server = spawn_server(&bin, port);

    let client = TcpClient::connect("127.0.0.1", port, "", "").unwrap();
    assert!(client.execute("(undefined_symbol_xyz)").is_err());
}

#[test]
fn connect_failure_is_an_error() {
    let _rt = Runtime::new().unwrap();
    // Nothing listening on this port.
    let port = free_port();
    assert!(TcpClient::connect("127.0.0.1", port, "", "").is_err());
}
