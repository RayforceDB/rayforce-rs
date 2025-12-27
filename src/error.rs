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

    /// Invalid operation on parted table.
    #[error("Cannot perform destructive operation on parted table: {0}")]
    PartedTableError(String),

    /// Connection error.
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(String),

    /// Query error.
    #[error("Query error: {0}")]
    QueryError(String),

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

