/*
*   Copyright (c) 2025-2026 Anton Kundenko <singaraiona@gmail.com>
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

//! Error types for the Rayforce Rust bindings.

use thiserror::Error;

/// Result type for Rayforce operations.
pub type Result<T> = std::result::Result<T, RayforceError>;

/// Error types that can occur in Rayforce operations.
#[derive(Error, Debug)]
pub enum RayforceError {
    /// Failed to create the Rayforce runtime.
    #[error("Failed to create runtime")]
    RuntimeCreationFailed,

    /// Failed to evaluate an expression.
    #[error("Evaluation failed: {0}")]
    EvalFailed(String),

    /// Type mismatch error.
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: String,
        actual: String,
    },

    /// Index out of bounds.
    #[error("Index out of bounds: {index} (length: {length})")]
    IndexOutOfBounds {
        index: i64,
        length: i64,
    },

    /// Null pointer error.
    #[error("Null pointer encountered")]
    NullPointer,

    /// Invalid string encoding.
    #[error("Invalid string encoding")]
    InvalidString,

    /// Key not found in dictionary or table.
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(String),

    /// Conversion error.
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// Memory allocation error.
    #[error("Memory allocation failed")]
    AllocationFailed,

    /// Invalid GUID format.
    #[error("Invalid GUID format: {0}")]
    InvalidGuid(String),

    /// Generic C API error.
    #[error("C API error: {0}")]
    CApiError(String),

    /// Error returned from the Rayforce engine. `code` is the
    /// short tag from `ray_err_code` ("oom", "type", "range", ...);
    /// `kind` is the matching `ray_err_t` enum value when known.
    #[error("Rayforce error [{code}]: {message}")]
    Ray {
        code: String,
        message: String,
        kind: Option<crate::ray_err_t>,
    },
}

impl From<crate::ray_err_t> for RayforceError {
    fn from(err: crate::ray_err_t) -> Self {
        let code = ray_err_t_code(err);
        RayforceError::Ray {
            code: code.to_string(),
            message: code.to_string(),
            kind: Some(err),
        }
    }
}

impl From<std::ffi::NulError> for RayforceError {
    fn from(_: std::ffi::NulError) -> Self {
        RayforceError::InvalidString
    }
}

impl From<std::str::Utf8Error> for RayforceError {
    fn from(_: std::str::Utf8Error) -> Self {
        RayforceError::InvalidString
    }
}

/// Map a `ray_err_t` to its short code string.
///
/// Mirrors `ray_err_code_str` from the C API but doesn't require a
/// runtime call (and avoids hitting the C side from generic code paths).
fn ray_err_t_code(err: crate::ray_err_t) -> &'static str {
    match err {
        crate::ray_err_t_RAY_OK => "ok",
        crate::ray_err_t_RAY_ERR_OOM => "oom",
        crate::ray_err_t_RAY_ERR_TYPE => "type",
        crate::ray_err_t_RAY_ERR_RANGE => "range",
        crate::ray_err_t_RAY_ERR_LENGTH => "length",
        crate::ray_err_t_RAY_ERR_RANK => "rank",
        crate::ray_err_t_RAY_ERR_DOMAIN => "domain",
        crate::ray_err_t_RAY_ERR_NYI => "nyi",
        crate::ray_err_t_RAY_ERR_IO => "io",
        crate::ray_err_t_RAY_ERR_SCHEMA => "schema",
        crate::ray_err_t_RAY_ERR_CORRUPT => "corrupt",
        crate::ray_err_t_RAY_ERR_CANCEL => "cancel",
        crate::ray_err_t_RAY_ERR_PARSE => "parse",
        crate::ray_err_t_RAY_ERR_NAME => "name",
        crate::ray_err_t_RAY_ERR_LIMIT => "limit",
        crate::ray_err_t_RAY_ERR_RESERVED => "reserved",
        _ => "unknown",
    }
}
