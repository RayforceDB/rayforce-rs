//! Table type and operations for Rayforce.

use crate::error::{RayforceError, Result};
use crate::ffi::{self, RayObj};
use crate::types::{RayDict, RayList, RayType, RaySymbol, RayVector};
use crate::*;
use std::collections::HashMap;
use std::fmt;

/// A Rayforce table.
#[derive(Clone)]
pub struct RayTable {
    ptr: RayObj,
    is_reference: bool,
    is_parted: bool,
}

impl RayTable {
    /// Create a table from column names (symbols) and column data (list of vectors).
    pub fn new(columns: RayVector<RaySymbol>, data: RayList) -> Result<Self> {
        let ptr = ffi::new_table(columns.ptr().clone(), data.ptr().clone())?;
        Ok(Self {
            ptr,
            is_reference: false,
            is_parted: false,
        })
    }

    /// Create a table from a dictionary of column name -> vector.
    pub fn from_dict<I, K, V>(columns: I) -> Result<Self>
    where
        K: AsRef<str>,
        V: Into<RayObj>,
        I: IntoIterator<Item = (K, V)>,
    {
        let items: Vec<_> = columns.into_iter().collect();
        let keys = RayVector::<RaySymbol>::from_iter(items.iter().map(|(k, _)| k.as_ref()));
        let mut values = RayList::new();
        for (_, v) in items {
            values.push(v);
        }
        
        let ptr = ffi::new_table(keys.ptr().clone(), values.ptr().clone())?;
        Ok(Self {
            ptr,
            is_reference: false,
            is_parted: false,
        })
    }

    /// Create a table reference by name (lazy loading).
    pub fn from_name(name: &str) -> Self {
        Self {
            ptr: ffi::new_symbol(name),
            is_reference: true,
            is_parted: false,
        }
    }

    /// Create from a RayObj pointer.
    pub fn from_ptr(ptr: RayObj) -> Result<Self> {
        if ptr.type_code() != TYPE_TABLE as i8 {
            return Err(RayforceError::TypeMismatch {
                expected: "RayTable".into(),
                actual: format!("type code {}", ptr.type_code()),
            });
        }
        Ok(Self {
            ptr,
            is_reference: false,
            is_parted: false,
        })
    }

    /// Check if this is a reference to a named table.
    pub fn is_reference(&self) -> bool {
        self.is_reference
    }

    /// Check if this is a parted table.
    pub fn is_parted(&self) -> bool {
        self.is_parted
    }

    /// Get the column names.
    pub fn columns(&self) -> Result<Vec<String>> {
        unsafe {
            let ptr = if self.is_reference {
                // Evaluate the reference to get the actual table
                let evaled = eval_obj(clone_obj(self.ptr.as_ptr()));
                if evaled.is_null() {
                    return Err(RayforceError::EvalFailed("Failed to evaluate table reference".into()));
                }
                evaled
            } else {
                self.ptr.as_ptr()
            };

            // Get the keys (column names) from the table
            // Table structure: [keys, values]
            let keys = at_idx(ptr, 0);
            if keys.is_null() {
                return Err(RayforceError::NullPointer);
            }

            let keys_obj = RayObj::from_raw(clone_obj(keys));
            let len = ffi::get_obj_len(&keys_obj) as usize;
            let mut result = Vec::with_capacity(len);
            
            let raw = ffi::get_obj_raw_ptr(&keys_obj) as *const i64;
            
            for i in 0..len {
                let id = *raw.add(i);
                let cstr = str_from_symbol(id);
                if !cstr.is_null() {
                    result.push(std::ffi::CStr::from_ptr(cstr).to_string_lossy().into_owned());
                }
            }

            if self.is_reference {
                drop_obj(ptr);
            }

            Ok(result)
        }
    }

    /// Get the number of rows.
    pub fn len(&self) -> Result<usize> {
        unsafe {
            let ptr = if self.is_reference {
                let evaled = eval_obj(clone_obj(self.ptr.as_ptr()));
                if evaled.is_null() {
                    return Err(RayforceError::EvalFailed("Failed to evaluate table reference".into()));
                }
                evaled
            } else {
                self.ptr.as_ptr()
            };

            // Get the values (columns) from the table
            let values = at_idx(ptr, 1);
            if values.is_null() {
                if self.is_reference {
                    drop_obj(ptr);
                }
                return Ok(0);
            }

            // Get the first column to determine row count
            let first_col = at_idx(values, 0);
            let len = if first_col.is_null() {
                0
            } else {
                let first_col_obj = RayObj::from_raw(clone_obj(first_col));
                ffi::get_obj_len(&first_col_obj) as usize
            };

            if self.is_reference {
                drop_obj(ptr);
            }

            Ok(len)
        }
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Get a column by name.
    pub fn get_column(&self, name: &str) -> Result<RayObj> {
        let key = ffi::new_symbol(name);
        unsafe {
            let ptr = if self.is_reference {
                let evaled = eval_obj(clone_obj(self.ptr.as_ptr()));
                if evaled.is_null() {
                    return Err(RayforceError::EvalFailed("Failed to evaluate table reference".into()));
                }
                evaled
            } else {
                self.ptr.as_ptr()
            };

            let col = at_obj(ptr, key.as_ptr());
            if col.is_null() {
                if self.is_reference {
                    drop_obj(ptr);
                }
                return Err(RayforceError::KeyNotFound(name.to_string()));
            }

            let result = RayObj::from_raw(clone_obj(col));

            if self.is_reference {
                drop_obj(ptr);
            }

            Ok(result)
        }
    }

    /// Save the table to the environment with a name.
    pub fn save(&self, name: &str) -> Result<()> {
        ffi::set_global(name, &self.ptr)?;
        Ok(())
    }

    /// Get the underlying RayObj.
    pub fn as_ray_obj(&self) -> &RayObj {
        &self.ptr
    }

    /// Create a select query builder.
    pub fn select(&self) -> RaySelectQuery {
        RaySelectQuery::new(self.clone())
    }

    /// Create an update query builder.
    pub fn update(&self) -> RayUpdateQuery {
        RayUpdateQuery::new(self.clone())
    }

    /// Create an insert query builder.
    pub fn insert(&self) -> RayInsertQuery {
        RayInsertQuery::new(self.clone())
    }

    /// Create an upsert query builder.
    pub fn upsert(&self, match_by_first: usize) -> RayUpsertQuery {
        RayUpsertQuery::new(self.clone(), match_by_first)
    }

    /// Sort ascending by columns.
    pub fn xasc(&self, columns: &[&str]) -> Result<RayTable> {
        let col_syms = RayVector::<RaySymbol>::from_iter(columns.iter().copied());
        let mut args = RayList::new();
        args.push(ffi::get_internal_function("xasc").ok_or_else(|| {
            RayforceError::CApiError("xasc not found".into())
        })?);
        
        let ptr = if self.is_reference {
            unsafe { eval_obj(clone_obj(self.ptr.as_ptr())) }
        } else {
            unsafe { clone_obj(self.ptr.as_ptr()) }
        };
        args.push(unsafe { RayObj::from_raw(ptr) });
        args.push(col_syms.ptr().clone());

        unsafe {
            let result = eval_obj(args.ptr().as_ptr());
            if result.is_null() {
                return Err(RayforceError::EvalFailed("xasc failed".into()));
            }
            std::mem::forget(args);
            RayTable::from_ptr(RayObj::from_raw(result))
        }
    }

    /// Sort descending by columns.
    pub fn xdesc(&self, columns: &[&str]) -> Result<RayTable> {
        let col_syms = RayVector::<RaySymbol>::from_iter(columns.iter().copied());
        let mut args = RayList::new();
        args.push(ffi::get_internal_function("xdesc").ok_or_else(|| {
            RayforceError::CApiError("xdesc not found".into())
        })?);
        
        let ptr = if self.is_reference {
            unsafe { eval_obj(clone_obj(self.ptr.as_ptr())) }
        } else {
            unsafe { clone_obj(self.ptr.as_ptr()) }
        };
        args.push(unsafe { RayObj::from_raw(ptr) });
        args.push(col_syms.ptr().clone());

        unsafe {
            let result = eval_obj(args.ptr().as_ptr());
            if result.is_null() {
                return Err(RayforceError::EvalFailed("xdesc failed".into()));
            }
            std::mem::forget(args);
            RayTable::from_ptr(RayObj::from_raw(result))
        }
    }

    /// Inner join with another table.
    pub fn inner_join(&self, other: &RayTable, on: &[&str]) -> Result<RayTable> {
        self.join_impl(other, on, "inner-join")
    }

    /// Left join with another table.
    pub fn left_join(&self, other: &RayTable, on: &[&str]) -> Result<RayTable> {
        self.join_impl(other, on, "left-join")
    }

    fn join_impl(&self, other: &RayTable, on: &[&str], join_type: &str) -> Result<RayTable> {
        let on_syms = RayVector::<RaySymbol>::from_iter(on.iter().copied());
        let mut args = RayList::new();
        args.push(ffi::get_internal_function(join_type).ok_or_else(|| {
            RayforceError::CApiError(format!("{} not found", join_type))
        })?);
        args.push(on_syms.ptr().clone());
        args.push(self.ptr.clone());
        args.push(other.ptr.clone());

        unsafe {
            let result = eval_obj(clone_obj(args.ptr().as_ptr()));
            if result.is_null() {
                return Err(RayforceError::EvalFailed(format!("{} failed", join_type)));
            }
            RayTable::from_ptr(RayObj::from_raw(result))
        }
    }

    /// Concatenate tables.
    pub fn concat(&self, other: &RayTable) -> Result<RayTable> {
        let mut args = RayList::new();
        args.push(ffi::get_internal_function("concat").ok_or_else(|| {
            RayforceError::CApiError("concat not found".into())
        })?);
        args.push(self.ptr.clone());
        args.push(other.ptr.clone());

        unsafe {
            let result = eval_obj(clone_obj(args.ptr().as_ptr()));
            if result.is_null() {
                return Err(RayforceError::EvalFailed("concat failed".into()));
            }
            RayTable::from_ptr(RayObj::from_raw(result))
        }
    }
}

impl RayType for RayTable {
    const TYPE_CODE: i8 = TYPE_TABLE as i8;
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
        if self.is_reference {
            write!(f, "RayTableRef")
        } else {
            write!(f, "RayTable")
        }
    }
}

impl fmt::Display for RayTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let formatted = obj_fmt(self.ptr.as_ptr(), 0);
            if formatted.is_null() {
                write!(f, "<table>")
            } else {
                let formatted_obj = RayObj::from_raw(formatted);
                let len = ffi::get_obj_len(&formatted_obj) as usize;
                let raw = ffi::get_obj_raw_ptr(&formatted_obj);
                let bytes = std::slice::from_raw_parts(raw, len);
                let s = String::from_utf8_lossy(bytes);
                write!(f, "{}", s)
            }
        }
    }
}

/// A table column reference for use in expressions.
#[derive(Clone)]
pub struct RayColumn {
    name: String,
}

impl RayColumn {
    /// Create a new column reference.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Get the column name.
    pub fn name(&self) -> &str {
        &self.name
    }

    // Comparison operations that return RayExpression

    /// Equal to.
    pub fn eq<T: Into<RayObj>>(&self, value: T) -> RayExpression {
        RayExpression::binary(Operation::Equals, self.clone(), value.into())
    }

    /// Not equal to.
    pub fn ne<T: Into<RayObj>>(&self, value: T) -> RayExpression {
        RayExpression::binary(Operation::NotEquals, self.clone(), value.into())
    }

    /// Greater than.
    pub fn gt<T: Into<RayObj>>(&self, value: T) -> RayExpression {
        RayExpression::binary(Operation::GreaterThan, self.clone(), value.into())
    }

    /// Greater than or equal.
    pub fn ge<T: Into<RayObj>>(&self, value: T) -> RayExpression {
        RayExpression::binary(Operation::GreaterEqual, self.clone(), value.into())
    }

    /// Less than.
    pub fn lt<T: Into<RayObj>>(&self, value: T) -> RayExpression {
        RayExpression::binary(Operation::LessThan, self.clone(), value.into())
    }

    /// Less than or equal.
    pub fn le<T: Into<RayObj>>(&self, value: T) -> RayExpression {
        RayExpression::binary(Operation::LessEqual, self.clone(), value.into())
    }

    /// Check if value is in a list.
    pub fn is_in<T: Into<RayObj>>(&self, values: T) -> RayExpression {
        RayExpression::binary(Operation::In, self.clone(), values.into())
    }

    // Aggregation operations

    /// Count aggregation.
    pub fn count(&self) -> RayExpression {
        RayExpression::unary(Operation::Count, self.clone())
    }

    /// Sum aggregation.
    pub fn sum(&self) -> RayExpression {
        RayExpression::unary(Operation::Sum, self.clone())
    }

    /// Average aggregation.
    pub fn avg(&self) -> RayExpression {
        RayExpression::unary(Operation::Avg, self.clone())
    }

    /// Min aggregation.
    pub fn min(&self) -> RayExpression {
        RayExpression::unary(Operation::Min, self.clone())
    }

    /// Max aggregation.
    pub fn max(&self) -> RayExpression {
        RayExpression::unary(Operation::Max, self.clone())
    }

    /// First aggregation.
    pub fn first(&self) -> RayExpression {
        RayExpression::unary(Operation::First, self.clone())
    }

    /// Last aggregation.
    pub fn last(&self) -> RayExpression {
        RayExpression::unary(Operation::Last, self.clone())
    }

    /// Distinct aggregation.
    pub fn distinct(&self) -> RayExpression {
        RayExpression::unary(Operation::Distinct, self.clone())
    }
}

impl From<&str> for RayColumn {
    fn from(name: &str) -> Self {
        RayColumn::new(name)
    }
}

impl From<RayColumn> for RayObj {
    fn from(col: RayColumn) -> Self {
        ffi::new_symbol(&col.name)
    }
}

/// Type alias for backward compatibility.
pub type Column = RayColumn;

use crate::types::Operation;

/// An expression for use in queries.
#[derive(Clone)]
pub struct RayExpression {
    operation: Operation,
    operands: Vec<ExprOperand>,
}

#[derive(Clone)]
enum ExprOperand {
    Column(RayColumn),
    Value(RayObj),
    Expr(Box<RayExpression>),
}

impl RayExpression {
    fn unary(op: Operation, col: RayColumn) -> Self {
        Self {
            operation: op,
            operands: vec![ExprOperand::Column(col)],
        }
    }

    fn binary(op: Operation, col: RayColumn, value: RayObj) -> Self {
        Self {
            operation: op,
            operands: vec![ExprOperand::Column(col), ExprOperand::Value(value)],
        }
    }

    /// Combine expressions with AND.
    pub fn and(self, other: RayExpression) -> RayExpression {
        RayExpression {
            operation: Operation::And,
            operands: vec![
                ExprOperand::Expr(Box::new(self)),
                ExprOperand::Expr(Box::new(other)),
            ],
        }
    }

    /// Combine expressions with OR.
    pub fn or(self, other: RayExpression) -> RayExpression {
        RayExpression {
            operation: Operation::Or,
            operands: vec![
                ExprOperand::Expr(Box::new(self)),
                ExprOperand::Expr(Box::new(other)),
            ],
        }
    }

    /// Compile the expression to a RayObj.
    pub fn compile(&self) -> RayObj {
        let mut list = RayList::new();
        
        if let Some(op) = self.operation.to_ray_obj() {
            list.push(op);
        } else {
            list.push(ffi::new_symbol(self.operation.name()));
        }

        for operand in &self.operands {
            match operand {
                ExprOperand::Column(col) => {
                    list.push(ffi::new_symbol(&col.name));
                }
                ExprOperand::Value(val) => {
                    list.push(val.clone());
                }
                ExprOperand::Expr(expr) => {
                    list.push(expr.compile());
                }
            }
        }

        list.ptr().clone()
    }
}

/// Type alias for backward compatibility.
pub type Expression = RayExpression;

/// Select query builder.
pub struct RaySelectQuery {
    table: RayTable,
    columns: Vec<String>,
    computed: HashMap<String, RayExpression>,
    where_conditions: Vec<RayExpression>,
    group_by: Vec<String>,
}

impl RaySelectQuery {
    fn new(table: RayTable) -> Self {
        Self {
            table,
            columns: Vec::new(),
            computed: HashMap::new(),
            where_conditions: Vec::new(),
            group_by: Vec::new(),
        }
    }

    /// Select specific columns.
    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.columns = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a computed column.
    pub fn column_expr(mut self, name: &str, expr: RayExpression) -> Self {
        self.computed.insert(name.to_string(), expr);
        self
    }

    /// Add a WHERE condition.
    pub fn where_cond(mut self, expr: RayExpression) -> Self {
        self.where_conditions.push(expr);
        self
    }

    /// Add GROUP BY columns.
    pub fn group_by(mut self, cols: &[&str]) -> Self {
        self.group_by = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Execute the query.
    pub fn execute(self) -> Result<RayTable> {
        let query_dict = self.build_query_dict()?;
        
        unsafe {
            let result = ray_select(query_dict.ptr().as_ptr());
            if result.is_null() {
                return Err(RayforceError::QueryError("Select query failed".into()));
            }
            if (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::QueryError(msg));
            }
            RayTable::from_ptr(RayObj::from_raw(result))
        }
    }

    fn build_query_dict(&self) -> Result<RayDict> {
        let mut pairs: Vec<(&str, RayObj)> = Vec::new();

        // Add 'from'
        if self.table.is_reference {
            pairs.push(("from", ffi::quote(&self.table.ptr)));
        } else {
            pairs.push(("from", self.table.ptr.clone()));
        }

        // Add selected columns
        for col in &self.columns {
            pairs.push((col, ffi::new_symbol(col)));
        }

        // Add computed columns
        for (name, expr) in &self.computed {
            pairs.push((name, expr.compile()));
        }

        // Add WHERE
        if !self.where_conditions.is_empty() {
            let mut combined = self.where_conditions[0].clone();
            for cond in &self.where_conditions[1..] {
                combined = combined.and(cond.clone());
            }
            pairs.push(("where", combined.compile()));
        }

        // Add GROUP BY
        if !self.group_by.is_empty() {
            let mut by_dict: Vec<(&str, RayObj)> = Vec::new();
            for col in &self.group_by {
                by_dict.push((col, ffi::new_symbol(col)));
            }
            let by = RayDict::from_pairs(by_dict)?;
            pairs.push(("by", by.ptr().clone()));
        }

        RayDict::from_pairs(pairs)
    }
}

/// Type alias for backward compatibility.
pub type SelectQuery = RaySelectQuery;

/// Update query builder.
pub struct RayUpdateQuery {
    table: RayTable,
    updates: HashMap<String, RayExpression>,
    where_conditions: Vec<RayExpression>,
}

impl RayUpdateQuery {
    fn new(table: RayTable) -> Self {
        Self {
            table,
            updates: HashMap::new(),
            where_conditions: Vec::new(),
        }
    }

    /// Set a column to an expression.
    pub fn set(mut self, column: &str, expr: RayExpression) -> Self {
        self.updates.insert(column.to_string(), expr);
        self
    }

    /// Set a column to a value.
    pub fn set_value<T: Into<RayObj>>(mut self, column: &str, value: T) -> Self {
        // For simple assignment, we use a trivial expression
        self.updates.insert(column.to_string(), RayExpression {
            operation: Operation::Eval,
            operands: vec![ExprOperand::Value(value.into())],
        });
        self
    }

    /// Add a WHERE condition.
    pub fn where_cond(mut self, expr: RayExpression) -> Self {
        self.where_conditions.push(expr);
        self
    }

    /// Execute the update.
    pub fn execute(self) -> Result<RayTable> {
        let query_dict = self.build_query_dict()?;
        
        unsafe {
            let result = ray_update(query_dict.ptr().as_ptr());
            if result.is_null() {
                return Err(RayforceError::QueryError("Update query failed".into()));
            }
            if (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::QueryError(msg));
            }
            
            if self.table.is_reference {
                // Return a reference to the updated table
                let sym = RaySymbol::from_ptr(RayObj::from_raw(result))?;
                Ok(RayTable::from_name(&sym.value()))
            } else {
                RayTable::from_ptr(RayObj::from_raw(result))
            }
        }
    }

    fn build_query_dict(&self) -> Result<RayDict> {
        let mut pairs: Vec<(&str, RayObj)> = Vec::new();

        // Add 'from'
        if self.table.is_reference {
            pairs.push(("from", ffi::quote(&self.table.ptr)));
        } else {
            pairs.push(("from", self.table.ptr.clone()));
        }

        // Add updates
        for (col, expr) in &self.updates {
            pairs.push((col, expr.compile()));
        }

        // Add WHERE
        if !self.where_conditions.is_empty() {
            let mut combined = self.where_conditions[0].clone();
            for cond in &self.where_conditions[1..] {
                combined = combined.and(cond.clone());
            }
            pairs.push(("where", combined.compile()));
        }

        RayDict::from_pairs(pairs)
    }
}

/// Type alias for backward compatibility.
pub type UpdateQuery = RayUpdateQuery;

/// Insert query builder.
pub struct RayInsertQuery {
    table: RayTable,
    data: Option<RayObj>,
}

impl RayInsertQuery {
    fn new(table: RayTable) -> Self {
        Self {
            table,
            data: None,
        }
    }

    /// Insert data as a dictionary of column -> values.
    pub fn values<I, K, V>(mut self, data: I) -> Self
    where
        K: AsRef<str>,
        V: Into<RayObj>,
        I: IntoIterator<Item = (K, V)>,
    {
        if let Ok(dict) = RayDict::from_pairs(data) {
            self.data = Some(dict.ptr().clone());
        }
        self
    }

    /// Insert data from a RayList of row values.
    pub fn rows(mut self, data: RayList) -> Self {
        self.data = Some(data.ptr().clone());
        self
    }

    /// Execute the insert.
    pub fn execute(self) -> Result<RayTable> {
        let data = self.data.ok_or_else(|| {
            RayforceError::QueryError("No data provided for insert".into())
        })?;

        let table_ptr = ffi::quote(&self.table.ptr);
        
        unsafe {
            let args = [table_ptr.as_ptr(), data.as_ptr()];
            let result = ray_insert(args.as_ptr() as *mut *mut obj_t, 2);
            
            if result.is_null() {
                return Err(RayforceError::QueryError("Insert query failed".into()));
            }
            if (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::QueryError(msg));
            }
            
            if self.table.is_reference {
                let sym = RaySymbol::from_ptr(RayObj::from_raw(result))?;
                Ok(RayTable::from_name(&sym.value()))
            } else {
                RayTable::from_ptr(RayObj::from_raw(result))
            }
        }
    }
}

/// Type alias for backward compatibility.
pub type InsertQuery = RayInsertQuery;

/// Upsert query builder.
pub struct RayUpsertQuery {
    table: RayTable,
    match_by_first: usize,
    data: Option<RayObj>,
}

impl RayUpsertQuery {
    fn new(table: RayTable, match_by_first: usize) -> Self {
        Self {
            table,
            match_by_first,
            data: None,
        }
    }

    /// Upsert data as a dictionary of column -> values.
    pub fn values<I, K, V>(mut self, data: I) -> Self
    where
        K: AsRef<str>,
        V: Into<RayObj>,
        I: IntoIterator<Item = (K, V)>,
    {
        if let Ok(dict) = RayDict::from_pairs(data) {
            self.data = Some(dict.ptr().clone());
        }
        self
    }

    /// Execute the upsert.
    pub fn execute(self) -> Result<RayTable> {
        let data = self.data.ok_or_else(|| {
            RayforceError::QueryError("No data provided for upsert".into())
        })?;

        let table_ptr = ffi::quote(&self.table.ptr);
        let keys = RayObj::from(self.match_by_first as i64);
        
        unsafe {
            let args = [table_ptr.as_ptr(), keys.as_ptr(), data.as_ptr()];
            let result = ray_upsert(args.as_ptr() as *mut *mut obj_t, 3);
            
            if result.is_null() {
                return Err(RayforceError::QueryError("Upsert query failed".into()));
            }
            if (*result).type_ == TYPE_ERR as i8 {
                let msg = ffi::get_error_message(result);
                drop_obj(result);
                return Err(RayforceError::QueryError(msg));
            }
            
            if self.table.is_reference {
                let sym = RaySymbol::from_ptr(RayObj::from_raw(result))?;
                Ok(RayTable::from_name(&sym.value()))
            } else {
                RayTable::from_ptr(RayObj::from_raw(result))
            }
        }
    }
}

/// Type alias for backward compatibility.
pub type UpsertQuery = RayUpsertQuery;

/// Type alias for backward compatibility.
pub type Table = RayTable;

