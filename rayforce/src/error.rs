//! Error types for the safe API.
//!
//! Core operations return a `*mut ray_t` that may be a `RAY_ERROR` object. We
//! detect it ([`raw::is_err`]), extract the short ASCII code and the per-thread
//! message, free the error, and surface a typed [`RayError`].

use crate::raw::{self, Raw};
use rayforce_sys as sys;
use std::ffi::CStr;
use std::fmt;

/// A categorized RayforceDB error code (mirrors the core `ray_err_t` enum and
/// the short codes carried by `RAY_ERROR` objects).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Oom,
    Type,
    Range,
    Length,
    Rank,
    Domain,
    Nyi,
    Io,
    Schema,
    Corrupt,
    Cancel,
    Parse,
    Name,
    Limit,
    /// A code the core produced that doesn't map to a known category.
    Other,
}

impl ErrorCode {
    /// Map a core `ray_err_t` numeric value.
    fn from_err_t(e: sys::ray_err_t) -> Self {
        match e {
            sys::ray_err_t_RAY_ERR_OOM => ErrorCode::Oom,
            sys::ray_err_t_RAY_ERR_TYPE => ErrorCode::Type,
            sys::ray_err_t_RAY_ERR_RANGE => ErrorCode::Range,
            sys::ray_err_t_RAY_ERR_LENGTH => ErrorCode::Length,
            sys::ray_err_t_RAY_ERR_RANK => ErrorCode::Rank,
            sys::ray_err_t_RAY_ERR_DOMAIN => ErrorCode::Domain,
            sys::ray_err_t_RAY_ERR_NYI => ErrorCode::Nyi,
            sys::ray_err_t_RAY_ERR_IO => ErrorCode::Io,
            sys::ray_err_t_RAY_ERR_SCHEMA => ErrorCode::Schema,
            sys::ray_err_t_RAY_ERR_CORRUPT => ErrorCode::Corrupt,
            sys::ray_err_t_RAY_ERR_CANCEL => ErrorCode::Cancel,
            sys::ray_err_t_RAY_ERR_PARSE => ErrorCode::Parse,
            sys::ray_err_t_RAY_ERR_NAME => ErrorCode::Name,
            sys::ray_err_t_RAY_ERR_LIMIT => ErrorCode::Limit,
            _ => ErrorCode::Other,
        }
    }
}

/// An error returned by a RayforceDB operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RayError {
    /// Categorized error code.
    pub code: ErrorCode,
    /// The raw short code string as produced by the core (e.g. `"type"`).
    pub code_str: String,
    /// Human-readable detail, when the core provided one.
    pub message: String,
}

impl RayError {
    /// Construct a binding-side error (not originating from a core object).
    pub(crate) fn binding(message: impl Into<String>) -> Self {
        RayError {
            code: ErrorCode::Other,
            code_str: String::new(),
            message: message.into(),
        }
    }

    /// Build a [`RayError`] from a core `RAY_ERROR` object and release it.
    ///
    /// # Safety
    /// `err` must be a valid `RAY_ERROR` pointer (see [`raw::is_err`]).
    pub(crate) unsafe fn from_obj(err: Raw) -> Self {
        let code = ErrorCode::from_err_t(sys::ray_err_from_obj(err));
        let code_str = cstr_to_string(sys::ray_err_code(err));
        let message = cstr_to_string(sys::ray_error_msg());
        // ray_release is a no-op for error objects; free explicitly.
        sys::ray_error_free(err);
        RayError {
            code,
            code_str,
            message,
        }
    }
}

impl fmt::Display for RayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = if self.code_str.is_empty() {
            "rayforce error"
        } else {
            &self.code_str
        };
        if self.message.is_empty() {
            write!(f, "{label}")
        } else {
            write!(f, "{label}: {}", self.message)
        }
    }
}

impl std::error::Error for RayError {}

/// Convenience result alias for the crate.
pub type Result<T> = std::result::Result<T, RayError>;

unsafe fn cstr_to_string(p: *const std::os::raw::c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

/// Inspect a freshly-returned, owned core pointer: convert `RAY_ERROR` into
/// `Err` (freeing it), pass everything else through unchanged.
///
/// # Safety
/// `v` must be an owned pointer just returned by a core call (or null).
pub(crate) unsafe fn check(v: Raw) -> Result<Raw> {
    if raw::is_err(v) {
        Err(RayError::from_obj(v))
    } else {
        Ok(v)
    }
}

/// `RAY_LAZY` — the deferred-DAG result type returned by `ray_eval` and
/// graph-aware builtins.
const RAY_LAZY: i8 = 104;

/// Materialize a lazy DAG result into a concrete value. A no-op for non-lazy
/// inputs; consumes the lazy reference and returns the owned materialized one.
///
/// # Safety
/// `v` must be an owned, non-error pointer (post-[`check`]).
pub(crate) unsafe fn materialize(v: Raw) -> Result<Raw> {
    if v.is_null() || raw::type_code(v) != RAY_LAZY {
        return Ok(v);
    }
    let m = sys::ray_lazy_materialize(v);
    if m.is_null() {
        return Err(RayError::binding("lazy materialization failed"));
    }
    if raw::is_err(m) {
        return Err(RayError::from_obj(m));
    }
    Ok(m)
}
