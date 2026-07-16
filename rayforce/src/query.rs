//! Fluent query builders: select / update / insert / upsert / joins.
//!
//! `select` compiles to a dict AST `{projections…, from, where, by}` wrapped in
//! `(select dict)` and evaluated; `update`/`insert`/`upsert` call the core
//! query builtins directly. Mirrors `rayforce-py/types/table.py`.

use crate::error::{check, materialize, Result};
use crate::expr::Expr;
use crate::ops::Operation;
use crate::runtime::eval_value;
use crate::table::Table;
use crate::value::Value;
use rayforce_sys as sys;

/// Aggregation operations that collapse a column to a single value.
fn is_aggregation(op: Operation) -> bool {
    matches!(
        op,
        Operation::Sum
            | Operation::Avg
            | Operation::Count
            | Operation::Min
            | Operation::Max
            | Operation::First
            | Operation::Last
            | Operation::Median
    )
}

/// Rewrite `(sum (filter c m))` → `(sum (if m c 0))` for the select/DAG path,
/// where `filter` isn't DAG-lowerable but `if` lowers to an element-wise mask.
/// Recurses through the tree.
fn dag_rewrite(e: &Expr) -> Expr {
    match e {
        Expr::Op(Operation::Sum, ops) if ops.len() == 1 => {
            if let Expr::Op(Operation::Filter, f) = &ops[0] {
                if f.len() == 2 {
                    return Expr::Op(
                        Operation::Sum,
                        vec![Expr::Op(
                            Operation::If,
                            vec![
                                dag_rewrite(&f[1]),
                                dag_rewrite(&f[0]),
                                Expr::Lit(Value::i64(0)),
                            ],
                        )],
                    );
                }
            }
            Expr::Op(Operation::Sum, vec![dag_rewrite(&ops[0])])
        }
        Expr::Op(op, ops) => Expr::Op(*op, ops.iter().map(dag_rewrite).collect()),
        Expr::Call(name, ops) => Expr::Call(name.clone(), ops.iter().map(dag_rewrite).collect()),
        other => other.clone(),
    }
}

fn compile_query_expr(e: &Expr) -> Value {
    dag_rewrite(e).compile()
}

/// A `select` query builder.
pub struct Select {
    from: Value,
    proj: Vec<(String, Expr)>,
    wheres: Vec<Expr>,
    by: Vec<(String, Expr)>,
    order: Option<(Vec<String>, bool)>,
}

impl Select {
    pub(crate) fn new(from: Value) -> Select {
        Select {
            from,
            proj: Vec::new(),
            wheres: Vec::new(),
            by: Vec::new(),
            order: None,
        }
    }

    /// Project a column through unchanged.
    pub fn col(mut self, name: impl Into<String>) -> Select {
        let n = name.into();
        self.proj.push((n.clone(), Expr::Col(n)));
        self
    }

    /// Project several columns through unchanged.
    pub fn cols<I, S>(mut self, names: I) -> Select
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for n in names {
            let n = n.into();
            self.proj.push((n.clone(), Expr::Col(n)));
        }
        self
    }

    /// Add a named computed / aggregation column.
    pub fn agg(mut self, name: impl Into<String>, expr: Expr) -> Select {
        self.proj.push((name.into(), expr));
        self
    }

    /// Add a `where` predicate (multiple predicates are AND-combined).
    pub fn filter(mut self, predicate: Expr) -> Select {
        self.wheres.push(predicate);
        self
    }

    /// Group by a column.
    pub fn by(mut self, name: impl Into<String>) -> Select {
        let n = name.into();
        self.by.push((n.clone(), Expr::Col(n)));
        self
    }

    /// Group by a named computed expression.
    pub fn by_expr(mut self, name: impl Into<String>, expr: Expr) -> Select {
        self.by.push((name.into(), expr));
        self
    }

    /// Order the result by columns (ascending unless `desc`).
    pub fn order_by<I, S>(mut self, cols: I, desc: bool) -> Select
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.order = Some((cols.into_iter().map(Into::into).collect(), desc));
        self
    }

    /// Build the select dict AST.
    fn compile_dict(&self) -> Value {
        let mut keys: Vec<String> = Vec::new();
        let mut vals: Vec<Value> = Vec::new();

        for (name, expr) in &self.proj {
            keys.push(name.clone());
            vals.push(compile_query_expr(expr));
        }

        if !self.by.is_empty() {
            let bk: Vec<String> = self.by.iter().map(|(n, _)| n.clone()).collect();
            let bv: Vec<Value> = self.by.iter().map(|(_, e)| compile_query_expr(e)).collect();
            keys.push("by".to_string());
            vals.push(Value::dict(Value::sym_vec(&bk), Value::list(&bv)));
        }

        keys.push("from".to_string());
        vals.push(self.from.clone());

        if !self.wheres.is_empty() {
            let mut combined = self.wheres[0].clone();
            for w in &self.wheres[1..] {
                combined = combined.and(w.clone());
            }
            keys.push("where".to_string());
            vals.push(compile_query_expr(&combined));
        }

        Value::dict(Value::sym_vec(&keys), Value::list(&vals))
    }

    fn is_aggregation_only(&self) -> bool {
        if !self.by.is_empty() || self.proj.is_empty() {
            return false;
        }
        self.proj.iter().all(|(_, e)| match e {
            Expr::Op(op, _) => is_aggregation(*op),
            _ => false,
        })
    }

    /// Compile, evaluate, and return the resulting table.
    pub fn execute(&self) -> Result<Table> {
        let dict = self.compile_dict();
        let select_ast = Value::list(&[Value::name_ref("select"), dict]);

        let result = match &self.order {
            Some((cols, desc)) => {
                let op = if *desc { "xdesc" } else { "xasc" };
                let ast = Value::list(&[Value::name_ref(op), select_ast, Value::sym_vec(cols)]);
                eval_value(&ast)?
            }
            None => eval_value(&select_ast)?,
        };

        let result = if self.is_aggregation_only() {
            take_rows(&result, 1)?
        } else {
            result
        };
        Table::from_value(result)
    }
}

/// An `update` query builder.
pub struct Update {
    from: Value,
    sets: Vec<(String, Expr)>,
    wheres: Vec<Expr>,
}

impl Update {
    pub(crate) fn new(from: Value) -> Update {
        Update {
            from,
            sets: Vec::new(),
            wheres: Vec::new(),
        }
    }

    /// Set / add a column to the given expression.
    pub fn set(mut self, name: impl Into<String>, expr: Expr) -> Update {
        self.sets.push((name.into(), expr));
        self
    }

    /// Restrict the update to matching rows.
    pub fn filter(mut self, predicate: Expr) -> Update {
        self.wheres.push(predicate);
        self
    }

    fn compile_dict(&self) -> Value {
        let mut keys: Vec<String> = Vec::new();
        let mut vals: Vec<Value> = Vec::new();
        for (name, expr) in &self.sets {
            keys.push(name.clone());
            vals.push(compile_query_expr(expr));
        }
        keys.push("from".to_string());
        vals.push(self.from.clone());
        if !self.wheres.is_empty() {
            let mut combined = self.wheres[0].clone();
            for w in &self.wheres[1..] {
                combined = combined.and(w.clone());
            }
            keys.push("where".to_string());
            vals.push(compile_query_expr(&combined));
        }
        Value::dict(Value::sym_vec(&keys), Value::list(&vals))
    }

    /// Apply the update and return the new table.
    pub fn execute(&self) -> Result<Table> {
        let dict = self.compile_dict();
        unsafe {
            let mut args = [dict.as_ptr()];
            let r = materialize(check(sys::ray_update_fn(args.as_mut_ptr(), 1))?)?;
            Table::from_value(Value::from_owned(r))
        }
    }
}

impl Table {
    /// Begin a `select` over this table.
    pub fn select(&self) -> Select {
        Select::new(self.as_value().clone())
    }

    /// Begin an `update` over this table.
    pub fn update(&self) -> Update {
        Update::new(self.as_value().clone())
    }

    /// Append a single row of scalar values.
    pub fn insert_row(&self, values: &[Value]) -> Result<Table> {
        let data = Value::list(values);
        self.insert_data(&data)
    }

    /// Append rows column-wise (each `Value` is a column vector).
    pub fn insert_columns(&self, columns: &[Value]) -> Result<Table> {
        let data = Value::list(columns);
        self.insert_data(&data)
    }

    fn insert_data(&self, data: &Value) -> Result<Table> {
        unsafe {
            let mut args = [self.as_value().as_ptr(), data.as_ptr()];
            let r = materialize(check(sys::ray_insert_fn(args.as_mut_ptr(), 2))?)?;
            Table::from_value(Value::from_owned(r))
        }
    }

    /// Upsert rows column-wise, treating the first `key_columns` columns as the
    /// match key.
    pub fn upsert_columns(&self, key_columns: i64, columns: &[Value]) -> Result<Table> {
        let keys = Value::i64(key_columns);
        let data = Value::list(columns);
        unsafe {
            let mut args = [self.as_value().as_ptr(), keys.as_ptr(), data.as_ptr()];
            let r = materialize(check(sys::ray_upsert_fn(args.as_mut_ptr(), 3))?)?;
            Table::from_value(Value::from_owned(r))
        }
    }

    /// First `n` rows.
    pub fn head(&self, n: i64) -> Result<Table> {
        Table::from_value(take_rows(self.as_value(), n)?)
    }

    /// Last `n` rows.
    pub fn tail(&self, n: i64) -> Result<Table> {
        Table::from_value(take_rows(self.as_value(), -n)?)
    }

    /// Take `n` rows (negative counts from the end).
    pub fn take(&self, n: i64) -> Result<Table> {
        Table::from_value(take_rows(self.as_value(), n)?)
    }

    /// Inner join with `other` on the given key columns.
    pub fn inner_join<S: AsRef<str>>(&self, other: &Table, on: &[S]) -> Result<Table> {
        self.join(Operation::InnerJoin, other, on)
    }

    /// Left join with `other` on the given key columns.
    pub fn left_join<S: AsRef<str>>(&self, other: &Table, on: &[S]) -> Result<Table> {
        self.join(Operation::LeftJoin, other, on)
    }

    /// As-of join with `other` on the given key columns.
    pub fn asof_join<S: AsRef<str>>(&self, other: &Table, on: &[S]) -> Result<Table> {
        self.join(Operation::AsofJoin, other, on)
    }

    fn join<S: AsRef<str>>(&self, op: Operation, other: &Table, on: &[S]) -> Result<Table> {
        let on_vec = Value::sym_vec(on);
        let ast = Value::list(&[
            Value::name_ref(op.as_str()),
            on_vec,
            self.as_value().clone(),
            other.as_value().clone(),
        ]);
        Table::from_value(eval_value(&ast)?)
    }
}

/// Evaluate `(take value n)` and return the result value.
fn take_rows(table: &Value, n: i64) -> Result<Value> {
    let ast = Value::list(&[Value::name_ref("take"), table.clone(), Value::i64(n)]);
    eval_value(&ast)
}
