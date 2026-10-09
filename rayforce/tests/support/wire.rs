//! Q wire-format builders and a frame reader, so a test can be either side of
//! the protocol on a raw socket: the publisher a `Subscription` listens to, or
//! the client a `QListener` serves. No `q` binary and no `rayforce -q` needed.

use std::io::Read;
use std::net::TcpStream;

pub const ASYNC: u8 = 0;
pub const SYNC: u8 = 1;
pub const RESPONSE: u8 = 2;

/// Symbol atom: type -11, NUL-terminated name.
pub fn sym_atom(s: &str) -> Vec<u8> {
    let mut b = vec![(-11i8) as u8];
    b.extend_from_slice(s.as_bytes());
    b.push(0);
    b
}

/// Long atom: type -7, eight little-endian bytes.
pub fn long_atom(v: i64) -> Vec<u8> {
    let mut b = vec![(-7i8) as u8];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// Symbol vector: type 11, attrs, int32 len, NUL-terminated names.
pub fn sym_vec(syms: &[&str]) -> Vec<u8> {
    let mut b = vec![11u8, 0u8];
    b.extend_from_slice(&(syms.len() as i32).to_le_bytes());
    for s in syms {
        b.extend_from_slice(s.as_bytes());
        b.push(0);
    }
    b
}

/// Long (i64) vector: type 7, attrs, int32 len, data.
pub fn long_vec(vals: &[i64]) -> Vec<u8> {
    let mut b = vec![7u8, 0u8];
    b.extend_from_slice(&(vals.len() as i32).to_le_bytes());
    for v in vals {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// Char vector: type 10, attrs, int32 len, bytes. What a peer sends to have a
/// string evaluated.
pub fn char_vec(s: &str) -> Vec<u8> {
    let mut b = vec![10u8, 0u8];
    b.extend_from_slice(&(s.len() as i32).to_le_bytes());
    b.extend_from_slice(s.as_bytes());
    b
}

/// General list: type 0, attrs, int32 len, then the elements.
pub fn list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0u8, 0u8];
    b.extend_from_slice(&(items.len() as i32).to_le_bytes());
    for it in items {
        b.extend_from_slice(it);
    }
    b
}

/// Dictionary: type 99, keys object, values object.
pub fn dict(keys: Vec<u8>, vals: Vec<u8>) -> Vec<u8> {
    let mut b = vec![99u8];
    b.extend(keys);
    b.extend(vals);
    b
}

/// Wrap a serialized object in the 8-byte Q wire header.
pub fn frame(msgtype: u8, body: &[u8]) -> Vec<u8> {
    let size = (8 + body.len()) as u32;
    let mut b = vec![1u8, msgtype, 0u8, 0u8]; // little-endian, uncompressed
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(body);
    b
}

/// Read one complete wire message and return its type byte and body, or
/// `None` if the peer hung up first.
pub fn read_frame(sock: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let mut hdr = [0u8; 8];
    sock.read_exact(&mut hdr).ok()?;
    let size = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let mut body = vec![0u8; size.saturating_sub(8)];
    sock.read_exact(&mut body).ok()?;
    Some((hdr[1], body))
}

/// Read one complete wire message and discard it. False if the peer hung up.
pub fn drain_one(sock: &mut TcpStream) -> bool {
    read_frame(sock).is_some()
}
