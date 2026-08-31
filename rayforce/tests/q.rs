//! Verify the `rayforce-q` Q IPC client against a mock Q server (raw TCP that
//! speaks the Q wire protocol). No `q` binary required.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use rayforce::{q::QConnection, Runtime, Table};

// --- Q wire-format builders (server side) ---------------------------------

/// Long (i64) vector: type 7, attrs, int32 len, data.
fn long_vec(vals: &[i64]) -> Vec<u8> {
    let mut b = vec![7u8, 0u8];
    b.extend_from_slice(&(vals.len() as i32).to_le_bytes());
    for v in vals {
        b.extend_from_slice(&v.to_le_bytes());
    }
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

/// Table: type 98, attrs(0), dict-marker(99), KS-keys, general-list of columns.
fn table(names: &[&str], cols: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![98u8, 0u8, 99u8];
    b.extend(sym_vec(names));
    b.push(0u8); // general list
    b.push(0u8); // attrs
    b.extend_from_slice(&(cols.len() as i32).to_le_bytes());
    for c in cols {
        b.extend_from_slice(c);
    }
    b
}

/// Wrap a serialized object as a Q response message (8-byte header + body).
fn msg(body: &[u8]) -> Vec<u8> {
    let size = (8 + body.len()) as u32;
    let mut b = vec![1u8, 2u8, 0u8, 0u8]; // little-endian, response, uncompressed
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(body);
    b
}

/// Bind a mock Q server that answers one request with `response`; returns the port.
fn spawn_mock(response: Vec<u8>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            // handshake: client sends 2 bytes, server replies with 1.
            let mut hs = [0u8; 2];
            let _ = sock.read_exact(&mut hs);
            let _ = sock.write_all(&[3u8]);
            // read the query message (header then body) and discard it.
            let mut hdr = [0u8; 8];
            if sock.read_exact(&mut hdr).is_ok() {
                let size = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
                let mut body = vec![0u8; size.saturating_sub(8)];
                let _ = sock.read_exact(&mut body);
            }
            let _ = sock.write_all(&response);
        }
    });
    port
}

#[test]
fn q_pulls_a_table() {
    Runtime::scope(|_rt| {
        let response = msg(&table(
            &["seq", "sym"],
            &[long_vec(&[1, 2, 3]), sym_vec(&["AAPL", "MSFT", "GOOG"])],
        ));
        let port = spawn_mock(response);

        let conn = QConnection::connect("127.0.0.1", port).unwrap();
        let v = conn.execute("select from fixmsgs where i > 0").unwrap();

        let t = Table::from_value(v).unwrap();
        assert_eq!(t.shape(), (3, 2));
        assert_eq!(
            t.column("seq").unwrap().as_slice::<i64>().unwrap(),
            &[1, 2, 3]
        );
        let sym = t.column("sym").unwrap();
        assert_eq!(sym.get(0).unwrap().as_sym().unwrap(), "AAPL");
        assert_eq!(sym.get(2).unwrap().as_sym().unwrap(), "GOOG");
        Ok(())
    })
    .unwrap();
}

#[test]
fn q_surfaces_server_error() {
    Runtime::scope(|_rt| {
        // Q error frame: type -128 then a NUL-terminated message.
        let mut body = vec![(-128i8) as u8];
        body.extend_from_slice(b"type\0");
        let port = spawn_mock(msg(&body));

        let conn = QConnection::connect("127.0.0.1", port).unwrap();
        assert!(conn.execute("1+`a").is_err());
        Ok(())
    })
    .unwrap();
}
