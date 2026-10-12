//! A raw client for Rayforce's own IPC, the wire `rdb-query` speaks: a 2-byte
//! handshake, then each request a SYNC frame holding one string atom. Raw
//! sockets on helper threads, because a `TcpClient` needs a runtime on its
//! thread and the one runtime a process may hold is the server's.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::fd::AsRawFd;

const PREFIX: u32 = 0xCEFA_DEFA;
const VERSION: u8 = 3;
const SYNC: u8 = 1;
const STR: i8 = -13;
const I64: i8 = -5;
const ERROR: u8 = 127;

/// What the server answered, decoded only as far as the tests read it.
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    Long(i64),
    /// The error's code, `cancel` or `name` for instance.
    Error(String),
    /// Any other type, by its type byte.
    Other(i8),
}

pub fn connect(port: u16) -> TcpStream {
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("the port is served");
    sock.set_nodelay(true).unwrap();
    sock.write_all(&[VERSION, 0]).unwrap();
    // The server answers with its version and whether it wants credentials.
    let mut handshake = [0u8; 2];
    sock.read_exact(&mut handshake)
        .expect("the handshake is answered");
    sock
}

pub fn send(sock: &mut TcpStream, text: &str) {
    let mut payload = vec![STR as u8, 0];
    payload.extend_from_slice(&(text.len() as i64).to_le_bytes());
    payload.extend_from_slice(text.as_bytes());
    let mut frame = PREFIX.to_le_bytes().to_vec();
    frame.extend_from_slice(&[VERSION, 0, 0, SYNC]);
    frame.extend_from_slice(&(payload.len() as i64).to_le_bytes());
    frame.extend_from_slice(&payload);
    sock.write_all(&frame).unwrap();
}

pub fn reply(sock: &mut TcpStream) -> Reply {
    let mut header = [0u8; 16];
    sock.read_exact(&mut header).expect("an answer");
    let size = i64::from_le_bytes(header[8..16].try_into().unwrap()) as usize;
    let mut body = vec![0u8; size];
    sock.read_exact(&mut body).unwrap();
    match body[0] {
        ERROR => {
            let code = &body[1..9.min(body.len())];
            let end = code.iter().position(|&b| b == 0).unwrap_or(code.len());
            Reply::Error(String::from_utf8_lossy(&code[..end]).into_owned())
        }
        b if b as i8 == I64 => Reply::Long(i64::from_le_bytes(body[2..10].try_into().unwrap())),
        b => Reply::Other(b as i8),
    }
}

/// Connect, send one request and wait for its answer.
pub fn ask(port: u16, text: &str) -> Reply {
    let mut sock = connect(port);
    send(&mut sock, text);
    reply(&mut sock)
}

/// The core's own cancel: one byte of TCP urgent data, which raises SIGURG on
/// the server while it evaluates.
pub fn cancel(sock: &TcpStream) {
    let byte = b'!';
    let sent = unsafe { libc::send(sock.as_raw_fd(), (&raw const byte).cast(), 1, libc::MSG_OOB) };
    assert_eq!(sent, 1, "the urgent byte is sent");
}
