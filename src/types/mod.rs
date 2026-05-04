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

//! Rust wrappers for Rayforce 2.0 values.

mod scalars;
mod containers;
pub mod table;

pub use scalars::*;
pub use containers::*;
pub use table::*;

use crate::error::{RayforceError, Result};
use crate::ffi::RayObj;
use crate::*;

/// Trait implemented by every typed wrapper around a `RayObj`.
pub trait RayType: Sized {
    /// The 2.0 type tag for this wrapper (negative for atoms,
    /// non-negative for vectors / compounds).
    const TYPE_CODE: i8;

    /// Human-readable type name for error messages.
    const RAY_NAME: &'static str;

    /// Wrap an existing `RayObj`, validating the type tag.
    fn from_ptr(ptr: RayObj) -> Result<Self>;

    /// Borrow the underlying `RayObj`.
    fn ptr(&self) -> &RayObj;

    fn type_code(&self) -> i8 {
        self.ptr().type_code()
    }
}

/// Convert a Rust value to a `RayObj`.
pub fn to_ray<T: Into<RayObj>>(value: T) -> RayObj {
    value.into()
}

/// Try to convert a `RayObj` to a Rust type.
pub fn from_ray<T: TryFrom<RayObj, Error = RayforceError>>(obj: RayObj) -> Result<T> {
    T::try_from(obj)
}

/// Map a 2.0 type tag to the Rust wrapper name.
pub fn type_name_for_code(code: i8) -> &'static str {
    let abs = code.unsigned_abs() as u32;
    match abs {
        RAY_LIST => "RayList",
        RAY_BOOL => "RayBool",
        RAY_U8 => "RayU8",
        RAY_I16 => "RayI16",
        RAY_I32 => "RayI32",
        RAY_I64 => "RayI64",
        RAY_F32 => "RayF32",
        RAY_F64 => "RayF64",
        RAY_DATE => "RayDate",
        RAY_TIME => "RayTime",
        RAY_TIMESTAMP => "RayTimestamp",
        RAY_GUID => "RayGuid",
        RAY_SYM => "RaySymbol",
        RAY_STR => "RayString",
        RAY_INDEX => "RayIndex",
        RAY_TABLE => "RayTable",
        RAY_DICT => "RayDict",
        RAY_LAMBDA => "RayLambda",
        RAY_UNARY => "RayUnary",
        RAY_BINARY => "RayBinary",
        RAY_VARY => "RayVariadic",
        RAY_NULL => "RayNull",
        RAY_ERROR => "RayError",
        _ => "Unknown",
    }
}
