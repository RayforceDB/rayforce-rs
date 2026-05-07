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

//! Query builder for Rayforce 2.x.
//!
//! Renders a Rayfall source string from a fluent builder and dispatches
//! it through [`Rayforce::eval`].  This is the option-B implementation
//! from the migration plan: rather than constructing an AST and calling
//! `ray_select` / `ray_update` / etc. directly (those entry points
//! expect a *special-form* dict whose values stay unevaluated until the
//! evaluator gets to them — extremely awkward to build by hand from the
//! outside), we synthesize the textual form and let the engine parse,
//! compile, and execute it.
//!
//! ```rust,no_run
//! use rayforce::{Rayforce, SelectQuery, Column};
//!
//! # fn main() -> rayforce::Result<()> {
//! let rf = Rayforce::new()?;
//! rf.eval("(set t (table [sym price] (list [AAPL GOOG] [101.0 99.5])))")?;
//!
//! let result = SelectQuery::from("t")
//!     .column("sym", Column::new("sym"))
//!     .filter(Column::new("price").gt(100))
//!     .execute(&rf)?;
//! println!("{result}");
//! # Ok(()) }
//! ```

use crate::error::Result;
use crate::ffi::RayObj;
use crate::Rayforce;

/// A Rayfall expression fragment.
///
/// Internally just a piece of Rayfall source text.  Build leaf nodes
/// with the `lit_*` constructors or [`Expression::raw`], and combine
/// them with [`Operation`] / the `Column` arithmetic helpers.
#[derive(Debug, Clone)]
pub struct Expression(String);

impl Expression {
    /// Wrap a hand-written Rayfall fragment verbatim (no quoting).
    pub fn raw(src: impl Into<String>) -> Self {
        Self(src.into())
    }

    /// Integer literal.
    pub fn lit_i64(v: i64) -> Self {
        Self(v.to_string())
    }

    /// Float literal — always rendered with a decimal point so the
    /// parser keeps it as F64 (`100` would be parsed as I64).
    pub fn lit_f64(v: f64) -> Self {
        let s = format!("{v}");
        if s.contains('.') || s.contains('e') || s.contains('E') || s.contains("nan") || s.contains("inf") {
            Self(s)
        } else {
            Self(format!("{s}.0"))
        }
    }

    /// Boolean literal (`1b` / `0b`).
    pub fn lit_bool(v: bool) -> Self {
        Self(if v { "1b".into() } else { "0b".into() })
    }

    /// String literal, with `\` and `"` escaped.
    pub fn lit_str(v: &str) -> Self {
        let mut out = String::with_capacity(v.len() + 2);
        out.push('"');
        for c in v.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                _ => out.push(c),
            }
        }
        out.push('"');
        Self(out)
    }

    /// Symbol literal (`'name`).
    pub fn lit_sym(v: &str) -> Self {
        Self(format!("'{v}"))
    }

    /// Compose a function-call form: `(name a b c ...)`.
    pub fn call(name: &str, args: &[Expression]) -> Self {
        let mut out = String::new();
        out.push('(');
        out.push_str(name);
        for a in args {
            out.push(' ');
            out.push_str(&a.0);
        }
        out.push(')');
        Self(out)
    }

    /// Render to Rayfall source.
    pub fn to_source(&self) -> &str {
        &self.0
    }

    /// `(and self other)` — chainable boolean conjunction.
    pub fn and<E: Into<Expression>>(self, other: E) -> Expression {
        Expression::call("and", &[self, other.into()])
    }

    /// `(or self other)` — chainable boolean disjunction.
    pub fn or<E: Into<Expression>>(self, other: E) -> Expression {
        Expression::call("or", &[self, other.into()])
    }

    /// `(not self)` — chainable negation.
    pub fn not(self) -> Expression {
        Expression::call("not", &[self])
    }
}

impl From<&str> for Expression {
    fn from(v: &str) -> Self {
        Expression::lit_str(v)
    }
}

impl From<String> for Expression {
    fn from(v: String) -> Self {
        Expression::lit_str(&v)
    }
}

impl From<i64> for Expression {
    fn from(v: i64) -> Self {
        Expression::lit_i64(v)
    }
}

impl From<i32> for Expression {
    fn from(v: i32) -> Self {
        Expression::lit_i64(v as i64)
    }
}

impl From<f64> for Expression {
    fn from(v: f64) -> Self {
        Expression::lit_f64(v)
    }
}

impl From<bool> for Expression {
    fn from(v: bool) -> Self {
        Expression::lit_bool(v)
    }
}

/// A reference to a table column by name.
///
/// In Rayfall a bare symbol inside a query body resolves to the column
/// by that name — which is exactly what we render.
#[derive(Debug, Clone)]
pub struct Column(String);

impl Column {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn into_expr(self) -> Expression {
        Expression(self.0)
    }

    pub fn as_expr(&self) -> Expression {
        Expression(self.0.clone())
    }

    pub fn gt<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call(">", &[self.into_expr(), rhs.into()])
    }
    pub fn ge<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call(">=", &[self.into_expr(), rhs.into()])
    }
    pub fn lt<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("<", &[self.into_expr(), rhs.into()])
    }
    pub fn le<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("<=", &[self.into_expr(), rhs.into()])
    }
    pub fn eq<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("==", &[self.into_expr(), rhs.into()])
    }
    pub fn ne<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("<>", &[self.into_expr(), rhs.into()])
    }
    pub fn add<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("+", &[self.into_expr(), rhs.into()])
    }
    pub fn sub<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("-", &[self.into_expr(), rhs.into()])
    }
    pub fn mul<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("*", &[self.into_expr(), rhs.into()])
    }
    pub fn div<E: Into<Expression>>(self, rhs: E) -> Expression {
        Expression::call("%", &[self.into_expr(), rhs.into()])
    }

    /// `(in col values)`.  `values` is rendered via `Into<Expression>`
    /// — typical use is `col.is_in(Expression::raw("[1 2 3]"))` or
    /// any other vector literal expression.
    pub fn is_in<E: Into<Expression>>(self, values: E) -> Expression {
        Expression::call("in", &[self.into_expr(), values.into()])
    }

    // -- Aggregate shortcuts (1.0-compat ergonomics).
    pub fn sum(self) -> Expression {
        Expression::call("sum", &[self.into_expr()])
    }
    pub fn avg(self) -> Expression {
        Expression::call("avg", &[self.into_expr()])
    }
    pub fn min(self) -> Expression {
        Expression::call("min", &[self.into_expr()])
    }
    pub fn max(self) -> Expression {
        Expression::call("max", &[self.into_expr()])
    }
    pub fn count(self) -> Expression {
        Expression::call("count", &[self.into_expr()])
    }
    pub fn first(self) -> Expression {
        Expression::call("first", &[self.into_expr()])
    }
    pub fn last(self) -> Expression {
        Expression::call("last", &[self.into_expr()])
    }
    pub fn distinct(self) -> Expression {
        Expression::call("distinct", &[self.into_expr()])
    }
}

impl From<Column> for Expression {
    fn from(c: Column) -> Self {
        c.into_expr()
    }
}

/// Common aggregate / unary operations that compile to a Rayfall call.
///
/// The variants cover the operators most frequently bolted on top of
/// `select` group-bys; for anything else, build the `Expression`
/// directly with [`Expression::call`] or [`Expression::raw`].
#[derive(Debug, Clone)]
pub enum Operation {
    Sum(Expression),
    Avg(Expression),
    Min(Expression),
    Max(Expression),
    Count(Expression),
    First(Expression),
    Last(Expression),
    Abs(Expression),
    Neg(Expression),
    Not(Expression),
    /// Boolean `and` over two predicates.
    And(Expression, Expression),
    /// Boolean `or` over two predicates.
    Or(Expression, Expression),
    /// Escape hatch: `(name args ...)`.
    Custom(String, Vec<Expression>),
}

impl From<Operation> for Expression {
    fn from(op: Operation) -> Self {
        match op {
            Operation::Sum(e) => Expression::call("sum", &[e]),
            Operation::Avg(e) => Expression::call("avg", &[e]),
            Operation::Min(e) => Expression::call("min", &[e]),
            Operation::Max(e) => Expression::call("max", &[e]),
            Operation::Count(e) => Expression::call("count", &[e]),
            Operation::First(e) => Expression::call("first", &[e]),
            Operation::Last(e) => Expression::call("last", &[e]),
            Operation::Abs(e) => Expression::call("abs", &[e]),
            Operation::Neg(e) => Expression::call("neg", &[e]),
            Operation::Not(e) => Expression::call("not", &[e]),
            Operation::And(a, b) => Expression::call("and", &[a, b]),
            Operation::Or(a, b) => Expression::call("or", &[a, b]),
            Operation::Custom(name, args) => Expression::call(&name, &args),
        }
    }
}

// ===== SelectQuery =====================================================

/// `(select {col: expr ... from: tbl [where: pred] [by: key] [asc: c]
///           [desc: c] [take: n]})`
#[derive(Debug, Clone)]
pub struct SelectQuery {
    table: String,
    columns: Vec<(String, Expression)>,
    filter: Option<Expression>,
    by: Vec<Expression>,
    asc: Vec<String>,
    desc: Vec<String>,
    take: Option<i64>,
}

impl SelectQuery {
    /// Start building a query against the named global table.
    pub fn from(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            filter: None,
            by: Vec::new(),
            asc: Vec::new(),
            desc: Vec::new(),
            take: None,
        }
    }

    /// Add a projected column: `name: expr`.
    pub fn column(mut self, name: impl Into<String>, expr: impl Into<Expression>) -> Self {
        self.columns.push((name.into(), expr.into()));
        self
    }

    /// Bulk-add identity-projected columns: each name renders as
    /// `name: name`.  Equivalent to calling [`column`] for every entry
    /// with `expr` set to `Column::new(name)`.
    pub fn columns<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for n in names {
            let name: String = n.into();
            self.columns.push((name.clone(), Column::new(name).into_expr()));
        }
        self
    }

    /// `where: pred`.
    pub fn filter(mut self, pred: impl Into<Expression>) -> Self {
        self.filter = Some(pred.into());
        self
    }

    /// `by: <expr>` — append one grouping key.
    pub fn group_by(mut self, key: impl Into<Expression>) -> Self {
        self.by.push(key.into());
        self
    }

    pub fn asc(mut self, col: impl Into<String>) -> Self {
        self.asc.push(col.into());
        self
    }

    pub fn desc(mut self, col: impl Into<String>) -> Self {
        self.desc.push(col.into());
        self
    }

    pub fn take(mut self, n: i64) -> Self {
        self.take = Some(n);
        self
    }

    /// Render to Rayfall source.  Public so callers can inspect or
    /// embed the query inside a larger script.
    pub fn to_rayfall(&self) -> String {
        let mut out = String::new();
        out.push_str("(select {");
        for (name, expr) in &self.columns {
            out.push_str(name);
            out.push_str(": ");
            out.push_str(expr.to_source());
            out.push(' ');
        }
        out.push_str("from: ");
        out.push_str(&self.table);
        if let Some(pred) = &self.filter {
            out.push_str(" where: ");
            out.push_str(pred.to_source());
        }
        if !self.by.is_empty() {
            out.push_str(" by: ");
            push_keys_or_vec(&mut out, &self.by);
        }
        for c in &self.asc {
            out.push_str(" asc: ");
            out.push_str(c);
        }
        for c in &self.desc {
            out.push_str(" desc: ");
            out.push_str(c);
        }
        if let Some(n) = self.take {
            out.push_str(" take: ");
            out.push_str(&n.to_string());
        }
        out.push_str("})");
        out
    }

    /// Render and dispatch through [`Rayforce::eval`].
    pub fn execute(&self, rf: &Rayforce) -> Result<RayObj> {
        rf.eval(&self.to_rayfall())
    }
}

// ===== UpdateQuery =====================================================

/// `(update {col: expr ... from: 'tbl [where: pred] [by: key]})`.
///
/// The `from:` value is rendered as a quoted symbol so the engine
/// updates the named global table in place.  Non-mutating "produce a
/// new table" updates can be expressed by binding the result into a
/// fresh global with [`Rayforce::eval`] (`(set t2 ...)`).
#[derive(Debug, Clone)]
pub struct UpdateQuery {
    table: String,
    sets: Vec<(String, Expression)>,
    filter: Option<Expression>,
    by: Vec<Expression>,
}

impl UpdateQuery {
    pub fn from(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            sets: Vec::new(),
            filter: None,
            by: Vec::new(),
        }
    }

    pub fn set(mut self, name: impl Into<String>, expr: impl Into<Expression>) -> Self {
        self.sets.push((name.into(), expr.into()));
        self
    }

    pub fn filter(mut self, pred: impl Into<Expression>) -> Self {
        self.filter = Some(pred.into());
        self
    }

    pub fn group_by(mut self, key: impl Into<Expression>) -> Self {
        self.by.push(key.into());
        self
    }

    pub fn to_rayfall(&self) -> String {
        let mut out = String::new();
        out.push_str("(update {");
        for (name, expr) in &self.sets {
            out.push_str(name);
            out.push_str(": ");
            out.push_str(expr.to_source());
            out.push(' ');
        }
        out.push_str("from: '");
        out.push_str(&self.table);
        if let Some(pred) = &self.filter {
            out.push_str(" where: ");
            out.push_str(pred.to_source());
        }
        if !self.by.is_empty() {
            out.push_str(" by: ");
            push_keys_or_vec(&mut out, &self.by);
        }
        out.push_str("})");
        out
    }

    pub fn execute(&self, rf: &Rayforce) -> Result<RayObj> {
        rf.eval(&self.to_rayfall())
    }
}

// ===== InsertQuery =====================================================

/// `(insert t <rows>)` — append rows to a table and return the new table.
#[derive(Debug, Clone)]
pub struct InsertQuery {
    table: String,
    rows: Expression,
}

impl InsertQuery {
    /// Insert into the named global table.
    pub fn into_table(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            rows: Expression::raw("(list)"),
        }
    }

    /// Provide a row payload.  Common shapes are:
    /// - `Expression::raw("(list 4 'David 40.0)")` — single row as a list of column values
    /// - `Expression::raw("(list [4 5] ['David 'Eve] [40.0 50.0])")` — multiple rows
    /// - `Expression::raw("(dict [Value Name ID] (list 120.0 'Leo 12))")` — keyword form
    /// - `Expression::raw("(table [...] (list ...))")` — table-from-table append
    pub fn rows(mut self, rows: impl Into<Expression>) -> Self {
        self.rows = rows.into();
        self
    }

    /// Render Rayfall source.  Uses the in-place form `(insert 'tab rows)`
    /// so the named global table is mutated and callers don't need to
    /// rebind the result.
    pub fn to_rayfall(&self) -> String {
        format!("(insert '{} {})", self.table, self.rows.to_source())
    }

    pub fn execute(&self, rf: &Rayforce) -> Result<RayObj> {
        rf.eval(&self.to_rayfall())
    }
}

// ===== UpsertQuery =====================================================

/// `(upsert t key_idx <rows>)` — update if `key_idx` matches an
/// existing row, otherwise insert.
#[derive(Debug, Clone)]
pub struct UpsertQuery {
    table: String,
    key_idx: i64,
    rows: Expression,
}

impl UpsertQuery {
    pub fn into_table(table: impl Into<String>, key_idx: i64) -> Self {
        Self {
            table: table.into(),
            key_idx,
            rows: Expression::raw("(list)"),
        }
    }

    pub fn rows(mut self, rows: impl Into<Expression>) -> Self {
        self.rows = rows.into();
        self
    }

    /// Render Rayfall source.  Uses the in-place form `(upsert 'tab idx rows)`
    /// to mutate the named global table.
    pub fn to_rayfall(&self) -> String {
        format!(
            "(upsert '{} {} {})",
            self.table,
            self.key_idx,
            self.rows.to_source()
        )
    }

    pub fn execute(&self, rf: &Rayforce) -> Result<RayObj> {
        rf.eval(&self.to_rayfall())
    }
}

// ===== Helpers =========================================================

/// Render either a single expression (one key) or a `[a b c ...]`
/// vector literal (multi-column key) for `by:` / `asc:` clauses.
fn push_keys_or_vec(out: &mut String, exprs: &[Expression]) {
    if exprs.len() == 1 {
        out.push_str(exprs[0].to_source());
    } else {
        out.push('[');
        for (i, e) in exprs.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(e.to_source());
        }
        out.push(']');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_basic_render() {
        let q = SelectQuery::from("t");
        assert_eq!(q.to_rayfall(), "(select {from: t})");
    }

    #[test]
    fn select_filter_render() {
        let q = SelectQuery::from("t")
            .column("sym", Column::new("sym"))
            .filter(Column::new("price").gt(100));
        assert_eq!(
            q.to_rayfall(),
            "(select {sym: sym from: t where: (> price 100)})"
        );
    }

    #[test]
    fn select_groupby_render() {
        let q = SelectQuery::from("trades")
            .column("avg_p", Operation::Avg(Column::new("price").into_expr()))
            .group_by(Column::new("sym"));
        assert_eq!(
            q.to_rayfall(),
            "(select {avg_p: (avg price) from: trades by: sym})"
        );
    }

    #[test]
    fn update_render() {
        let q = UpdateQuery::from("tab")
            .set("price", Column::new("price").add(Expression::lit_i64(1)))
            .filter(Column::new("volume").gt(400));
        assert_eq!(
            q.to_rayfall(),
            "(update {price: (+ price 1) from: 'tab where: (> volume 400)})"
        );
    }

    #[test]
    fn insert_render() {
        let q = InsertQuery::into_table("t").rows(Expression::raw("(list 4 'David 40.0)"));
        assert_eq!(q.to_rayfall(), "(insert 't (list 4 'David 40.0))");
    }

    #[test]
    fn upsert_render() {
        let q = UpsertQuery::into_table("t", 1).rows(Expression::raw("(list 2 'Bobby 25.0)"));
        assert_eq!(q.to_rayfall(), "(upsert 't 1 (list 2 'Bobby 25.0))");
    }

    #[test]
    fn lit_sym_rendered() {
        assert_eq!(Expression::lit_sym("AAPL").to_source(), "'AAPL");
    }

    #[test]
    fn lit_f64_keeps_decimal() {
        assert_eq!(Expression::lit_f64(1.0).to_source(), "1.0");
        assert_eq!(Expression::lit_f64(1.5).to_source(), "1.5");
    }

    #[test]
    fn lit_str_escapes() {
        assert_eq!(Expression::lit_str("a\"b").to_source(), "\"a\\\"b\"");
    }
}
