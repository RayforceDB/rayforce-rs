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

//! Table type for Rayforce 2.0.
//!
//! The high-level query builder (`SelectQuery`/`UpdateQuery`/...) from
//! the 1.0 bindings is intentionally absent: the C symbols backing it
//! (`ray_select`, `ray_update`, `ray_insert`, `ray_upsert`) are no
//! longer in the 2.0 public API. Run queries via `Rayforce::eval` with
//! a Rayfall source string for now; a Rust-side builder layer that
//! synthesises Rayfall strings is a planned follow-up.

use crate::error::{RayforceError, Result};
use crate::ffi::{self, RayObj};
use crate::types::{RayList, RaySymbol, RayType, RayVector};
use crate::*;
use std::fmt;

/// A Rayforce table (`RAY_TABLE`).
#[derive(Clone)]
pub struct RayTable {
    ptr: RayObj,
}

impl RayTable {
    /// Build a table from a parallel `(names, columns)` pair.
    pub fn new(columns: RayVector<RaySymbol>, data: RayList) -> Result<Self> {
        // Resolve column names back to strings, pair with the matching
        // RayObj column from the list, and hand off to the FFI helper.
        let n = columns.len();
        if n != data.len() {
            return Err(RayforceError::CApiError(format!(
                "column count mismatch: {n} names vs {} columns",
                data.len()
            )));
        }
        let mut pairs: Vec<(String, RayObj)> = Vec::with_capacity(n);
        for i in 0..n {
            let name = columns.get(i).ok_or_else(|| RayforceError::NullPointer)?;
            let col = data.get(i).ok_or_else(|| RayforceError::NullPointer)?;
            pairs.push((name, col));
        }
        let ptr = ffi::new_table_from_pairs(pairs)?;
        Ok(Self { ptr })
    }

    /// Build a table from `(name, column)` pairs.
    pub fn from_dict<I, K, V>(columns: I) -> Result<Self>
    where
        K: AsRef<str>,
        V: Into<RayObj>,
        I: IntoIterator<Item = (K, V)>,
    {
        let pairs: Vec<(String, RayObj)> = columns
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.into()))
            .collect();
        let ptr = ffi::new_table_from_pairs(pairs)?;
        Ok(Self { ptr })
    }

    /// Resolve a named global binding to a table.
    ///
    /// The 1.0 bindings used to keep an unevaluated symbol reference and
    /// re-evaluate on every access. 2.0 has no `eval_obj`; we resolve
    /// once via `ray_env_get` and wrap the resulting table.
    pub fn from_name(name: &str) -> Result<Self> {
        unsafe {
            let id = ray_sym_intern(name.as_ptr() as *const i8, name.len());
            let val = ray_env_get(id);
            if val.is_null() {
                return Err(RayforceError::KeyNotFound(name.to_string()));
            }
            ray_retain(val);
            let obj = RayObj::from_raw(val);
            if obj.type_code() != RAY_TABLE as i8 {
                return Err(RayforceError::TypeMismatch {
                    expected: "RayTable".into(),
                    actual: format!("type code {}", obj.type_code()),
                });
            }
            Ok(Self { ptr: obj })
        }
    }

    pub fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != RAY_TABLE as i8 {
            return Err(RayforceError::TypeMismatch {
                expected: "RayTable".into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self { ptr })
    }

    /// Names of all columns, in declaration order.
    pub fn columns(&self) -> Result<Vec<String>> {
        unsafe {
            let n = ray_table_ncols(self.ptr.as_ptr());
            let mut out = Vec::with_capacity(n as usize);
            for i in 0..n {
                let id = ray_table_col_name(self.ptr.as_ptr(), i);
                let s_obj = ray_sym_str(id);
                if s_obj.is_null() {
                    out.push(String::new());
                    continue;
                }
                let p = ray_str_ptr(s_obj);
                let len = ray_str_len(s_obj);
                if p.is_null() {
                    out.push(String::new());
                } else {
                    let bytes = std::slice::from_raw_parts(p as *const u8, len);
                    out.push(String::from_utf8_lossy(bytes).into_owned());
                }
            }
            Ok(out)
        }
    }

    /// Number of rows.
    pub fn len(&self) -> Result<usize> {
        unsafe { Ok(ray_table_nrows(self.ptr.as_ptr()) as usize) }
    }

    /// Number of columns.
    pub fn ncols(&self) -> usize {
        unsafe { ray_table_ncols(self.ptr.as_ptr()) as usize }
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Look up a column by name. Returns a freshly retained `RayObj`.
    pub fn get_column(&self, name: &str) -> Result<RayObj> {
        unsafe {
            let id = ray_sym_intern(name.as_ptr() as *const i8, name.len());
            let col = ray_table_get_col(self.ptr.as_ptr(), id);
            if col.is_null() {
                return Err(RayforceError::KeyNotFound(name.to_string()));
            }
            ray_retain(col);
            Ok(RayObj::from_raw(col))
        }
    }

    /// Look up a column by ordinal index. Returns a freshly retained `RayObj`.
    pub fn get_column_idx(&self, idx: usize) -> Result<RayObj> {
        unsafe {
            let col = ray_table_get_col_idx(self.ptr.as_ptr(), idx as i64);
            if col.is_null() {
                return Err(RayforceError::IndexOutOfBounds {
                    index: idx as i64,
                    length: self.ncols() as i64,
                });
            }
            ray_retain(col);
            Ok(RayObj::from_raw(col))
        }
    }

    /// Bind this table under `name` in the global environment.
    pub fn save(&self, name: &str) -> Result<()> {
        ffi::set_global(name, &self.ptr)
    }

    pub fn as_ray_obj(&self) -> &RayObj {
        &self.ptr
    }
}

impl RayType for RayTable {
    const TYPE_CODE: i8 = RAY_TABLE as i8;
    const RAY_NAME: &'static str = "RayTable";

    fn from_ptr(ptr: RayObj) -> Result<Self> {
        RayTable::from_ptr(ptr)
    }

    fn ptr(&self) -> &RayObj {
        &self.ptr
    }
}

impl fmt::Debug for RayTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cols = self.columns().unwrap_or_default();
        let rows = self.len().unwrap_or(0);
        write!(f, "RayTable(ncols={}, nrows={}, cols={:?})", cols.len(), rows, cols)
    }
}

impl fmt::Display for RayTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 2.0 doesn't expose a public `obj_fmt`; fall back to a minimal
        // schema-only rendering.  Pretty-printing is a follow-up.
        let cols = self.columns().unwrap_or_default();
        let rows = self.len().unwrap_or(0);
        write!(f, "Table[{} rows × {} cols] {:?}", rows, cols.len(), cols)
    }
}

pub type Table = RayTable;
