//! KDB+ IPC client — connect to a kdb+ server, send a query, get the response
//! decoded into a rayforce [`Value`].
//!
//! The wire protocol (handshake, serialize/deserialize, decompression) is a C
//! client vendored into `rayforce-sys` (`csrc/kdb_ipc.c`, originally from
//! `rayforce-py`) — staged there until it moves into the engine proper. Sync
//! request/reply only (no async/subscribe), which is all the RevoLT-style
//! pull-by-sequence pattern needs. Requires a live [`crate::Runtime`] (decoded
//! responses are engine-allocated objects).
//!
//! ```no_run
//! use rayforce::{Runtime, kdb::KdbConnection};
//! let _rt = Runtime::new().unwrap();
//! let conn = KdbConnection::connect("localhost", 5010).unwrap();
//! let fills = conn.execute("select from fixmsgs where i > 0").unwrap();
//! ```

use std::ffi::CString;

use rayforce_sys as sys;

use crate::error::{check, RayError, Result};
use crate::value::Value;

/// An open connection to a kdb+ server. Closed on drop.
pub struct KdbConnection {
    slot: i64,
}

impl KdbConnection {
    /// Open a TCP connection and perform the KDB+ handshake.
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let host_c = CString::new(host).map_err(|_| RayError::binding("kdb host contains NUL"))?;
        let slot = unsafe { sys::rkdb_connect(host_c.as_ptr(), i32::from(port)) };
        if slot < 0 {
            return Err(RayError::binding(format!(
                "kdb: connect to {host}:{port} failed"
            )));
        }
        Ok(KdbConnection { slot })
    }

    /// Send a query string for remote evaluation; return the response decoded
    /// into a [`Value`] (atoms, vectors, tables, …). A kdb-side error surfaces
    /// as `Err`.
    pub fn execute(&self, query: &str) -> Result<Value> {
        let q = Value::string(query);
        // rkdb_send borrows `q` (serializes it), returns an owned result or a
        // RAY_ERROR object (never null); `check` converts the error object.
        let res = unsafe { sys::rkdb_send(self.slot, q.as_ptr()) };
        let ok = unsafe { check(res)? };
        Ok(unsafe { Value::from_owned(ok) })
    }
}

impl Drop for KdbConnection {
    fn drop(&mut self) {
        unsafe { sys::rkdb_close(self.slot) };
    }
}
