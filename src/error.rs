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

