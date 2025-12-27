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

//! Rayforce type system.
//!
//! This module provides Rust types that wrap Rayforce objects,
//! similar to the Python bindings.

mod scalars;
mod containers;
pub mod table;
mod operators;

pub use scalars::*;
pub use containers::*;
pub use table::*;
pub use operators::*;

use crate::error::{RayforceError, Result};
use crate::ffi::RayObj;
use crate::*;

/// Trait for types that can be converted to/from RayObj.
pub trait RayType: Sized {
    /// The Rayforce type code for this type.
    const TYPE_CODE: i8;
    
    /// The name of this type in Rayforce.
    const RAY_NAME: &'static str;

    /// Create from a RayObj pointer.
    fn from_ptr(ptr: RayObj) -> Result<Self>;

    /// Get the underlying RayObj.
    fn ptr(&self) -> &RayObj;

    /// Get the type code of the underlying object.
    fn type_code(&self) -> i8 {
        self.ptr().type_code()
    }
}

/// Convert a Rust value to a RayObj.
pub fn to_ray<T: Into<RayObj>>(value: T) -> RayObj {
    value.into()
}

/// Try to convert a RayObj to a Rust type.
pub fn from_ray<T: TryFrom<RayObj, Error = RayforceError>>(obj: RayObj) -> Result<T> {
    T::try_from(obj)
}

/// Get the type name for a type code.
pub fn type_name_for_code(code: i8) -> &'static str {
    match code.abs() as u32 {
        TYPE_LIST => "RayList",
        TYPE_B8 => "RayBool",
        TYPE_U8 => "RayU8",
        TYPE_I16 => "RayI16",
        TYPE_I32 => "RayI32",
        TYPE_I64 => "RayI64",
        TYPE_SYMBOL => "RaySymbol",
        TYPE_DATE => "RayDate",
        TYPE_TIME => "RayTime",
        TYPE_TIMESTAMP => "RayTimestamp",
        TYPE_F64 => "RayF64",
        TYPE_GUID => "RayGuid",
        TYPE_C8 => "RayString",
        TYPE_ENUM => "RayEnum",
        TYPE_TABLE => "RayTable",
        TYPE_DICT => "RayDict",
        TYPE_LAMBDA => "RayLambda",
        TYPE_UNARY => "RayUnary",
        TYPE_BINARY => "RayBinary",
        TYPE_VARY => "RayVariadic",
        TYPE_NULL => "RayNull",
        TYPE_ERR => "RayError",
        _ => "Unknown",
    }
}

