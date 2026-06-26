//! End-to-end check of the vendored KDB+ IPC client against a *real* `q`
//! server (not the mock in `kdb.rs`). Gated on `RAYFORCE_KDB_ADDR=host:port`
//! so it's a no-op when no kdb endpoint is available (CI, no `q` binary).
//!
//! Start a server first, e.g.:
//!   QHOME=~/lynx/q/m64 ~/lynx/q/m64/m64/q fixsrv.q -q
//! then:
//!   RAYFORCE_KDB_ADDR=127.0.0.1:5010 cargo test -p rayforce --test kdb_real -- --nocapture

use rayforce::{kdb::KdbConnection, Runtime, Table};

fn addr() -> Option<(String, u16)> {
    let a = std::env::var("RAYFORCE_KDB_ADDR").ok()?;
    let (h, p) = a.rsplit_once(':')?;
    Some((h.to_string(), p.parse().ok()?))
}

#[test]
fn real_q_roundtrips_atoms_vectors_and_tables() {
    let Some((host, port)) = addr() else {
        eprintln!("RAYFORCE_KDB_ADDR unset — skipping real-q e2e");
        return;
    };
    let _rt = Runtime::new().unwrap();
    let conn = KdbConnection::connect(&host, port).unwrap();

    // scalar
    let v = conn.execute("ping 41").unwrap();
    assert_eq!(v.format(), "42");

    // vector
    let v = conn.execute("exec seq from fixmsgs").unwrap();
    assert_eq!(v.as_slice::<i64>().unwrap(), &[1, 2, 3, 4, 5]);

    // full table
    let t = Table::from_value(conn.execute("select from fixmsgs").unwrap()).unwrap();
    assert_eq!(t.shape(), (5, 4));
    assert_eq!(
        t.column("seq").unwrap().as_slice::<i64>().unwrap(),
        &[1, 2, 3, 4, 5]
    );
    let sym = t.column("sym").unwrap();
    assert_eq!(sym.get(0).unwrap().as_sym().unwrap(), "AAPL");
    assert_eq!(sym.get(4).unwrap().as_sym().unwrap(), "TSLA");

    // RevoLT-style pull-by-sequence: only rows past a cursor
    let t = Table::from_value(conn.execute("select from fixmsgs where seq > 3").unwrap()).unwrap();
    assert_eq!(t.shape(), (2, 4));
    assert_eq!(
        t.column("seq").unwrap().as_slice::<i64>().unwrap(),
        &[4, 5]
    );

    // kdb-side error surfaces as Err
    assert!(conn.execute("1+`a").is_err());
}
