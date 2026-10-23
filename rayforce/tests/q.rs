//! Verify the `rayforce-q` Q IPC client against a mock Q server (raw TCP that
//! speaks the Q wire protocol). No `q` binary required.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use rayforce::q::{ConnectOpts, Keepalive};
use rayforce::{q, q::QConnection, Runtime, Table, Value};

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

/// Float (f64) vector: type 9, attrs, int32 len, data.
fn float_vec(vals: &[f64]) -> Vec<u8> {
    let mut b = vec![9u8, 0u8];
    b.extend_from_slice(&(vals.len() as i32).to_le_bytes());
    for v in vals {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// Timespan vector: type 16, nanoseconds as i64. `q_decode` has no branch for
/// it, so a message carrying one decodes only when the column is skipped.
fn timespan_vec(vals: &[i64]) -> Vec<u8> {
    let mut b = vec![16u8, 0u8];
    b.extend_from_slice(&(vals.len() as i32).to_le_bytes());
    for v in vals {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// String column: a general list (type 0) of char vectors (type 10).
fn string_col(vals: &[&str]) -> Vec<u8> {
    let mut b = vec![0u8, 0u8];
    b.extend_from_slice(&(vals.len() as i32).to_le_bytes());
    for v in vals {
        b.extend_from_slice(&[10u8, 0u8]);
        b.extend_from_slice(&(v.len() as i32).to_le_bytes());
        b.extend_from_slice(v.as_bytes());
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

/// Keyed table: a dict (type 99) from a key table to a value table.
fn keyed(key: Vec<u8>, val: Vec<u8>) -> Vec<u8> {
    let mut b = vec![99u8];
    b.extend(key);
    b.extend(val);
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

fn msg_big_endian(body: &[u8]) -> Vec<u8> {
    let size = (8 + body.len()) as u32;
    let mut b = vec![0u8, 2u8, 0u8, 0u8]; // big-endian, response, uncompressed
    b.extend_from_slice(&size.to_be_bytes());
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

#[test]
fn decode_response_rejects_big_endian_messages() {
    Runtime::scope(|_rt| {
        let err = q::decode_response(&msg_big_endian(&long_vec(&[1, 2, 3]))).unwrap_err();
        assert!(
            err.to_string().contains("little-endian"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn encode_round_trips_through_decode() {
    Runtime::scope(|_rt| {
        // `q::encode` produces a complete message; `q::decode_response` consumes
        // one. Together they are the two halves of a transport you own yourself.
        let v = rayforce::Value::vec(&[1i64, 2, 3]);
        let mut frame = rayforce::q::encode(&v).unwrap();

        // Header: little-endian, SYNC (the encoder always stamps SYNC), not
        // compressed, and a size covering the whole message.
        assert_eq!(frame[0], 1, "little-endian flag");
        assert_eq!(frame[1], 1, "encoder stamps SYNC");
        assert_eq!(frame[2], 0, "uncompressed");
        assert_eq!(
            u32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]) as usize,
            frame.len()
        );

        // `decode_response` accepts only a RESPONSE, so retype byte 1 first.
        frame[1] = 2;
        let back = rayforce::q::decode_response(&frame).unwrap();
        assert_eq!(back.as_slice::<i64>().unwrap(), &[1, 2, 3]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_rejects_non_response_messages() {
    Runtime::scope(|_rt| {
        let mut response = msg(&long_vec(&[1, 2, 3]));
        response[1] = 1; // sync request, not response

        let err = q::decode_response(&response).unwrap_err();
        assert!(
            err.to_string().contains("expected response message type 2"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}

// --- projected decode (q::decode_response_cols) ----------------------------

/// A five-column table of every column shape the projected decode treats
/// differently: fixed width, symbol, string, a timespan `q_decode` refuses, and
/// a float.
fn wide_table() -> Vec<u8> {
    table(
        &["seq", "sym", "note", "span", "px"],
        &[
            long_vec(&[1, 2, 3]),
            sym_vec(&["AAPL", "MSFT", "GOOG"]),
            string_col(&["a", "bc", ""]),
            timespan_vec(&[1, 2, 3]),
            float_vec(&[1.5, 2.5, 3.5]),
        ],
    )
}

/// The wire form of a column, to compare two decodes by.
fn wire(v: &Value) -> Vec<u8> {
    v.serialize().unwrap()
}

#[test]
fn decode_response_cols_keeps_the_named_columns_in_order() {
    Runtime::scope(|_rt| {
        let full = msg(&wide_table());
        // The timespan column is why the whole message cannot be decoded.
        assert!(q::decode_response(&full).is_err());

        let v = q::decode_response_cols(&full, &["px", "note", "seq", "sym"]).unwrap();
        let t = Table::from_value(v).unwrap();
        assert_eq!(t.shape(), (3, 4));
        assert_eq!(t.column_names(), ["px", "note", "seq", "sym"]);
        assert_eq!(
            t.column("px").unwrap().as_slice::<f64>().unwrap(),
            &[1.5, 2.5, 3.5]
        );
        assert_eq!(
            t.column("seq").unwrap().as_slice::<i64>().unwrap(),
            &[1, 2, 3]
        );
        let sym = t.column("sym").unwrap();
        assert_eq!(sym.get(1).unwrap().as_sym().unwrap(), "MSFT");
        let note = t.column("note").unwrap();
        assert_eq!(note.get(0).unwrap().as_string().unwrap(), "a");
        assert_eq!(note.get(1).unwrap().as_string().unwrap(), "bc");
        assert_eq!(note.get(2).unwrap().as_string().unwrap(), "");
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_cols_equals_decoding_the_projection() {
    Runtime::scope(|_rt| {
        // What q would have sent for `sym`note`px#t`, decoded whole.
        let projection = msg(&table(
            &["sym", "note", "px"],
            &[
                sym_vec(&["AAPL", "MSFT", "GOOG"]),
                string_col(&["a", "bc", ""]),
                float_vec(&[1.5, 2.5, 3.5]),
            ],
        ));
        let want = Table::from_value(q::decode_response(&projection).unwrap()).unwrap();

        let names = vec!["sym".to_string(), "note".to_string(), "px".to_string()];
        let got = q::decode_response_cols(&msg(&wide_table()), &names).unwrap();
        let got = Table::from_value(got).unwrap();

        assert_eq!(got.column_names(), want.column_names());
        for (g, w) in got.columns().unwrap().iter().zip(want.columns().unwrap()) {
            assert_eq!(wire(g), wire(&w));
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_cols_names_a_column_the_table_lacks() {
    Runtime::scope(|_rt| {
        let err = q::decode_response_cols(&msg(&wide_table()), &["seq", "nope"]).unwrap_err();
        assert!(
            err.to_string().contains("`nope`"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_cols_refuses_bad_names() {
    Runtime::scope(|_rt| {
        let full = msg(&wide_table());
        let err = q::decode_response_cols(&full, &["seq", "px", "seq"]).unwrap_err();
        assert!(
            err.to_string().contains("`seq` named twice"),
            "unexpected error: {err}"
        );
        let err = q::decode_response_cols(&full, &["s\0q"]).unwrap_err();
        assert!(err.to_string().contains("NUL"), "unexpected error: {err}");
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_cols_without_a_table_or_names_is_decode_response() {
    Runtime::scope(|_rt| {
        // Not a table: the names are ignored.
        let longs = msg(&long_vec(&[1, 2, 3]));
        let v = q::decode_response_cols(&longs, &["seq"]).unwrap();
        assert_eq!(v.as_slice::<i64>().unwrap(), &[1, 2, 3]);

        // No names: every column, as `decode_response` would.
        let narrow = msg(&table(
            &["seq", "sym"],
            &[long_vec(&[1, 2]), sym_vec(&["AAPL", "MSFT"])],
        ));
        let none: &[&str] = &[];
        let t = Table::from_value(q::decode_response_cols(&narrow, none).unwrap()).unwrap();
        assert_eq!(t.column_names(), ["seq", "sym"]);

        // A server error is an error either way.
        let mut body = vec![(-128i8) as u8];
        body.extend_from_slice(b"type\0");
        let err = q::decode_response_cols(&msg(&body), &["seq"]).unwrap_err();
        assert!(err.to_string().contains("type"), "unexpected error: {err}");
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_cols_projects_a_keyed_tables_value_side() {
    Runtime::scope(|_rt| {
        let kt = msg(&keyed(
            table(&["k"], &[sym_vec(&["x", "y"])]),
            table(
                &["a", "span", "c"],
                &[
                    long_vec(&[1, 2]),
                    timespan_vec(&[1, 2]),
                    float_vec(&[0.5, 1.5]),
                ],
            ),
        ));
        // A keyed table decodes as (key table; value table). Naming the key
        // column is allowed and changes nothing.
        let v = q::decode_response_cols(&kt, &["c", "k", "a"]).unwrap();
        assert!(v.is_list() && v.len() == 2, "type {}", v.type_code());
        let key = Table::from_value(v.get(0).unwrap()).unwrap();
        let val = Table::from_value(v.get(1).unwrap()).unwrap();
        assert_eq!(key.column_names(), ["k"]);
        assert_eq!(
            key.column("k").unwrap().get(1).unwrap().as_sym().unwrap(),
            "y"
        );
        assert_eq!(val.column_names(), ["c", "a"]);
        assert_eq!(
            val.column("c").unwrap().as_slice::<f64>().unwrap(),
            &[0.5, 1.5]
        );

        let err = q::decode_response_cols(&kt, &["k", "b"]).unwrap_err();
        assert!(err.to_string().contains("`b`"), "unexpected error: {err}");
        Ok(())
    })
    .unwrap();
}

#[test]
fn decode_response_cols_checks_the_wire_header() {
    Runtime::scope(|_rt| {
        let mut request = msg(&wide_table());
        request[1] = 1; // sync request, not response
        let err = q::decode_response_cols(&request, &["seq"]).unwrap_err();
        assert!(
            err.to_string().contains("expected response message type 2"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}

/// Bind a mock that accepts one connection and never answers, not even the
/// handshake; returns the port. The socket stays open until the test ends.
fn spawn_silent() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((sock, _)) = listener.accept() {
            thread::sleep(Duration::from_secs(60));
            drop(sock);
        }
    });
    port
}

#[test]
fn connect_opts_connects_with_every_keepalive() {
    Runtime::scope(|_rt| {
        // A sub-millisecond budget is valid: it rounds up, as a timeout does.
        for keepalive in [
            Keepalive::Auto,
            Keepalive::Off,
            Keepalive::Budget(Duration::from_secs(25)),
            Keepalive::Budget(Duration::from_micros(1)),
        ] {
            let port = spawn_mock(msg(&long_vec(&[7])));
            let opts = ConnectOpts {
                keepalive,
                ..ConnectOpts::default()
            };
            let conn = QConnection::connect_opts("127.0.0.1", port, &opts)
                .unwrap_or_else(|e| panic!("{keepalive:?}: {e}"));
            let v = conn.execute("7").unwrap();
            assert_eq!(v.as_slice::<i64>().unwrap(), &[7], "{keepalive:?}");
        }
        Ok(())
    })
    .unwrap();
}

/// The timeout and the keepalive are both `int` milliseconds at the C call, so
/// swapped they would still compile. Swapped, this connect would wait out the
/// 30 s budget as its timeout instead of failing at 200 ms.
#[test]
fn connect_opts_times_out_on_the_timeout_not_the_keepalive() {
    Runtime::scope(|_rt| {
        let port = spawn_silent();
        let opts = ConnectOpts {
            timeout: Some(Duration::from_millis(200)),
            keepalive: Keepalive::Budget(Duration::from_secs(30)),
            ..ConnectOpts::default()
        };
        let started = Instant::now();
        let err = QConnection::connect_opts("127.0.0.1", port, &opts)
            .err()
            .expect("a silent peer cannot complete the handshake");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "took {:?}: {err}",
            started.elapsed()
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn connect_opts_refuses_a_zero_keepalive_budget() {
    Runtime::scope(|_rt| {
        // `Off` already says off; a zero budget that meant it too would turn
        // keepalive off without anyone asking for that.
        let opts = ConnectOpts {
            keepalive: Keepalive::Budget(Duration::ZERO),
            ..ConnectOpts::default()
        };
        let err = QConnection::connect_opts("127.0.0.1", 1, &opts)
            .err()
            .expect("a zero budget is refused before connecting");
        assert!(
            err.to_string().contains("keepalive"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn connect_opts_refuses_a_keepalive_budget_past_the_c_int() {
    Runtime::scope(|_rt| {
        let ms = u64::try_from(i32::MAX).unwrap() + 1;
        let opts = ConnectOpts {
            keepalive: Keepalive::Budget(Duration::from_millis(ms)),
            ..ConnectOpts::default()
        };
        let err = QConnection::connect_opts("127.0.0.1", 1, &opts)
            .err()
            .expect("a budget past i32::MAX ms is refused before connecting");
        assert!(
            err.to_string().contains("keepalive"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn connect_opts_refuses_a_zero_timeout() {
    Runtime::scope(|_rt| {
        // The C side reads 0 as "block forever", the opposite of what a zero
        // timeout asks for. `None` is how to say no timeout.
        let opts = ConnectOpts {
            timeout: Some(Duration::ZERO),
            ..ConnectOpts::default()
        };
        let err = QConnection::connect_opts("127.0.0.1", 1, &opts)
            .err()
            .expect("a zero timeout is refused before connecting");
        assert!(
            err.to_string().contains("timeout"),
            "unexpected error: {err}"
        );
        Ok(())
    })
    .unwrap();
}
