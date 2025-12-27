/*
*   Copyright (c) 2025 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

//! IPC (Inter-Process Communication) support for connecting to remote Rayforce servers.

use crate::error::{RayforceError, Result};
use crate::ffi::{self, RayObj};
use crate::types::RayString;
use crate::*;

/// A connection to a remote RayforceDB server.
pub struct Connection {
    handle: RayObj,
    closed: bool,
}

impl Connection {
    /// Create a new connection from a handle.
    fn new(handle: RayObj) -> Self {
        Self {
            handle,
            closed: false,
        }
    }

    /// Execute a query string on the remote server.
    pub fn execute(&self, query: &str) -> Result<RayObj> {
        if self.closed {
            return Err(RayforceError::ConnectionError("Connection is closed".into()));
        }

        let query_str = RayString::new(query);
        unsafe {
            let result = ray_write(self.handle.as_ptr(), query_str.ptr().as_ptr());
            if result.is_null() {
                return Err(RayforceError::IoError("Write failed".into()));
            }
            if (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::IoError(msg));
            }
            Ok(RayObj::from_raw(result))
        }
    }

    /// Execute a RayObj query on the remote server.
    pub fn execute_obj(&self, obj: &RayObj) -> Result<RayObj> {
        if self.closed {
            return Err(RayforceError::ConnectionError("Connection is closed".into()));
        }

        unsafe {
            let result = ray_write(self.handle.as_ptr(), obj.as_ptr());
            if result.is_null() {
                return Err(RayforceError::IoError("Write failed".into()));
            }
            if (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::IoError(msg));
            }
            Ok(RayObj::from_raw(result))
        }
    }

    /// Close the connection.
    pub fn close(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }

        unsafe {
            let result = ray_hclose(self.handle.as_ptr());
            if !result.is_null() && (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::IoError(msg));
            }
            if !result.is_null() {
                drop_obj(result);
            }
        }

        self.closed = true;
        Ok(())
    }

    /// Check if the connection is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.close();
        }
    }
}

/// Open a connection to a remote RayforceDB server.
///
/// # Arguments
/// * `host` - The hostname or IP address
/// * `port` - The port number
///
/// # Example
/// ```rust,no_run
/// use rayforce::ipc::hopen;
///
/// let conn = hopen("localhost", 5000).unwrap();
/// let result = conn.execute("1+1").unwrap();
/// ```
pub fn hopen(host: &str, port: u16) -> Result<Connection> {
    let path = format!("{}:{}", host, port);
    let path_str = RayString::new(&path);
    
    unsafe {
        let args = [path_str.ptr().as_ptr()];
        let handle = ray_hopen(args.as_ptr() as *mut *mut obj_t, 1);
        
        if handle.is_null() {
            return Err(RayforceError::ConnectionError(
                format!("Failed to connect to {}:{}", host, port)
            ));
        }
        
        if (*handle).type_ == TYPE_ERR as i8 {
            let msg = ffi::get_error_message(handle);
            drop_obj(handle);
            return Err(RayforceError::ConnectionError(msg));
        }
        
        Ok(Connection::new(RayObj::from_raw(handle)))
    }
}

/// Open a connection with a timeout.
///
/// # Arguments
/// * `host` - The hostname or IP address
/// * `port` - The port number
/// * `timeout_ms` - Connection timeout in milliseconds
pub fn hopen_timeout(host: &str, port: u16, timeout_ms: i64) -> Result<Connection> {
    let path = format!("{}:{}", host, port);
    let path_str = RayString::new(&path);
    let timeout = RayObj::from(timeout_ms);
    
    unsafe {
        let args = [path_str.ptr().as_ptr(), timeout.as_ptr()];
        let handle = ray_hopen(args.as_ptr() as *mut *mut obj_t, 2);
        
        if handle.is_null() {
            return Err(RayforceError::ConnectionError(
                format!("Failed to connect to {}:{}", host, port)
            ));
        }
        
        if (*handle).type_ == TYPE_ERR as i8 {
            let msg = ffi::get_error_message(handle);
            drop_obj(handle);
            return Err(RayforceError::ConnectionError(msg));
        }
        
        Ok(Connection::new(RayObj::from_raw(handle)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Rayforce server
    #[test]
    #[ignore]
    fn test_connection() {
        let _rf = crate::Rayforce::new().unwrap();
        let conn = hopen("localhost", 5000).unwrap();
        assert!(!conn.is_closed());
    }
}

