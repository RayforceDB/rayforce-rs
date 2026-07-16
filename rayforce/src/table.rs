//! Tables (`RAY_TABLE`) — a schema (column-name symbols) paired with a list of
//! equal-length column vectors.
//!
//! [`Table`] is a thin newtype over [`Value`] carrying table-specific
//! construction, accessors, and on-disk / CSV I/O. Richer query operations
//! (select / where / by / joins) build on the expression layer (later phases).

use crate::error::{check, RayError, Result};
use crate::raw;
use crate::value::Value;
use rayforce_sys as sys;

/// A RayforceDB table.
#[derive(Clone)]
pub struct Table {
    value: Value,
}

impl Table {
    // ---- construction ----

    /// Build a table from column names and their column vectors.
    ///
    /// Each column is retained by the table; the caller's `Value`s remain valid.
    /// Errors if the counts differ or a name/column is invalid.
    pub fn new<S: AsRef<str>>(names: &[S], columns: &[Value]) -> Result<Table> {
        if names.len() != columns.len() {
            return Err(RayError::binding(format!(
                "table: {} names but {} columns",
                names.len(),
                columns.len()
            )));
        }
        // All columns must share a row count.
        if let Some(first) = columns.first() {
            let nrows = first.len();
            if let Some((i, c)) = columns.iter().enumerate().find(|(_, c)| c.len() != nrows) {
                return Err(RayError::binding(format!(
                    "table: column {i} has {} rows, expected {nrows}",
                    c.len()
                )));
            }
        }
        unsafe {
            let mut tbl = check(sys::ray_table_new(names.len() as i64))?;
            for (name, col) in names.iter().zip(columns.iter()) {
                let s = name.as_ref();
                let id = sys::ray_sym_intern(s.as_ptr() as *const _, s.len());
                let res = sys::ray_table_add_col(tbl, id, col.as_ptr());
                if raw::is_err(res) {
                    // add_col released `tbl` on its error path; the error owns
                    // nothing of ours.
                    return Err(RayError::from_obj(res));
                }
                tbl = res;
            }
            Ok(Table {
                value: Value::from_owned(tbl),
            })
        }
    }

    /// Wrap a [`Value`] that is a table (errors otherwise).
    pub fn from_value(value: Value) -> Result<Table> {
        if value.type_code() != sys::RAY_TABLE as i8 {
            return Err(RayError::binding(format!(
                "expected a table, got type tag {}",
                value.type_code()
            )));
        }
        Ok(Table { value })
    }

    /// Borrow the underlying [`Value`].
    pub fn as_value(&self) -> &Value {
        &self.value
    }

    /// Consume into the underlying [`Value`].
    pub fn into_value(self) -> Value {
        self.value
    }

    // ---- accessors ----

    /// Number of columns.
    pub fn ncols(&self) -> usize {
        unsafe { sys::ray_table_ncols(self.value.as_ptr()).max(0) as usize }
    }

    /// Number of rows.
    pub fn nrows(&self) -> usize {
        unsafe { sys::ray_table_nrows(self.value.as_ptr()).max(0) as usize }
    }

    /// `(rows, cols)`.
    pub fn shape(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }

    /// Column names in order.
    pub fn column_names(&self) -> Vec<String> {
        let n = self.ncols();
        let mut out = Vec::with_capacity(n);
        unsafe {
            for i in 0..n {
                let id = sys::ray_table_col_name(self.value.as_ptr(), i as i64);
                out.push(sym_id_to_string(id));
            }
        }
        out
    }

    /// Borrowed view of a column by name.
    pub fn column(&self, name: &str) -> Result<Value> {
        unsafe {
            let id = sys::ray_sym_intern(name.as_ptr() as *const _, name.len());
            let col = sys::ray_table_get_col(self.value.as_ptr(), id);
            if col.is_null() || raw::is_err(col) {
                return Err(RayError::binding(format!("column not found: {name}")));
            }
            Ok(Value::from_borrowed(col))
        }
    }

    /// Borrowed view of a column by position.
    pub fn column_at(&self, idx: usize) -> Result<Value> {
        if idx >= self.ncols() {
            return Err(RayError::binding(format!(
                "column index {idx} out of range ({} columns)",
                self.ncols()
            )));
        }
        unsafe {
            let col = sys::ray_table_get_col_idx(self.value.as_ptr(), idx as i64);
            if col.is_null() || raw::is_err(col) {
                return Err(RayError::binding(format!("column at {idx} is unavailable")));
            }
            Ok(Value::from_borrowed(col))
        }
    }

    /// All column vectors in order.
    pub fn columns(&self) -> Result<Vec<Value>> {
        (0..self.ncols()).map(|i| self.column_at(i)).collect()
    }

    // ---- CSV / on-disk I/O ----

    /// Read a CSV file with an explicit column schema (type names like
    /// `"I64"`, `"F64"`, `"SYMBOL"`, `"STR"`, `"DATE"`, `"TIME"`,
    /// `"TIMESTAMP"`, `"B8"`, `"GUID"`, `"I32"`, `"I16"`, `"U8"`).
    pub fn read_csv<S: AsRef<str>>(column_types: &[S], path: &str) -> Result<Table> {
        let upper: Vec<String> = column_types
            .iter()
            .map(|t| normalize_type_token(t.as_ref()))
            .collect();
        let schema = Value::sym_vec(&upper);
        let path_v = Value::string(path);
        unsafe {
            let mut args = [schema.as_ptr(), path_v.as_ptr()];
            let r = check(sys::ray_read_csv_fn(args.as_mut_ptr(), 2))?;
            Table::from_value(Value::from_owned(r))
        }
    }

    /// Write this table to a CSV file.
    pub fn write_csv(&self, path: &str) -> Result<()> {
        let path_v = Value::string(path);
        unsafe {
            let mut args = [self.value.as_ptr(), path_v.as_ptr()];
            check(sys::ray_write_csv_fn(args.as_mut_ptr(), 2))?;
        }
        Ok(())
    }

    /// Save as a splayed (column-per-file) table under `dir`. An optional
    /// symbol-file path may be supplied for enumerated symbol columns.
    pub fn save_splayed(&self, dir: &str, sym_path: Option<&str>) -> Result<()> {
        let dir_v = Value::string(dir);
        let sym_v = sym_path.map(Value::string);
        unsafe {
            match &sym_v {
                Some(s) => {
                    let mut args = [dir_v.as_ptr(), self.value.as_ptr(), s.as_ptr()];
                    check(sys::ray_set_splayed_fn(args.as_mut_ptr(), 3))?;
                }
                None => {
                    let mut args = [dir_v.as_ptr(), self.value.as_ptr()];
                    check(sys::ray_set_splayed_fn(args.as_mut_ptr(), 2))?;
                }
            }
        }
        Ok(())
    }

    /// Load a splayed table from `dir`.
    pub fn load_splayed(dir: &str, sym_path: Option<&str>) -> Result<Table> {
        let dir_v = Value::string(dir);
        let sym_v = sym_path.map(Value::string);
        unsafe {
            let r = match &sym_v {
                Some(s) => {
                    let mut args = [dir_v.as_ptr(), s.as_ptr()];
                    check(sys::ray_get_splayed_fn(args.as_mut_ptr(), 2))?
                }
                None => {
                    let mut args = [dir_v.as_ptr()];
                    check(sys::ray_get_splayed_fn(args.as_mut_ptr(), 1))?
                }
            };
            Table::from_value(Value::from_owned(r))
        }
    }

    /// Load a partitioned table named `name` rooted at `root`.
    pub fn load_parted(root: &str, name: &str) -> Result<Table> {
        let root_v = Value::string(root);
        let name_v = Value::sym(name);
        unsafe {
            let mut args = [root_v.as_ptr(), name_v.as_ptr()];
            let r = check(sys::ray_get_parted_fn(args.as_mut_ptr(), 2))?;
            Table::from_value(Value::from_owned(r))
        }
    }
}

impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value.format())
    }
}

impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Table{:?}\n{}", self.column_names(), self.value.format())
    }
}

impl Value {
    /// True if this value is a table.
    pub fn is_table(&self) -> bool {
        self.type_code() == sys::RAY_TABLE as i8
    }

    /// Interpret this value as a [`Table`] (cloning the handle).
    pub fn as_table(&self) -> Result<Table> {
        Table::from_value(self.clone())
    }
}

/// Normalize a user-supplied CSV column-type token to the engine's canonical
/// uppercase name, accepting common aliases.
fn normalize_type_token(t: &str) -> String {
    let u = t.to_uppercase();
    match u.as_str() {
        "SYM" => "SYMBOL".to_string(),
        "STRING" => "STR".to_string(),
        "BOOL" | "BOOLEAN" => "B8".to_string(),
        _ => u,
    }
}

/// Resolve a symbol id to an owned `String` via the global intern table.
unsafe fn sym_id_to_string(id: i64) -> String {
    let s = sys::ray_sym_str(id);
    if s.is_null() {
        return String::new();
    }
    let p = sys::ray_str_ptr(s).cast::<u8>();
    let n = sys::ray_str_len(s);
    if p.is_null() || n == 0 {
        String::new()
    } else {
        String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned()
    }
}
