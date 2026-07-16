//! Expression AST for the query DSL.
//!
//! An [`Expr`] is a column reference, a literal, or an operation applied to
//! sub-expressions. [`Expr::compile`] lowers it to a `Value` AST (a list whose
//! head is a name reference to a builtin); [`Expr::execute`] evaluates it.
//!
//! Comparisons are methods (`col("p").gt(100)`) because Rust's `==`/`<` must
//! return `bool`; arithmetic uses real operator overloads (`col("p") + 1`).

use crate::convert::ToValue;
use crate::ops::Operation;
use crate::runtime::eval_value;
use crate::value::Value;
use crate::Result;

/// A node in the query expression tree.
#[derive(Clone)]
pub enum Expr {
    /// A column / global-env name reference.
    Col(String),
    /// A literal value.
    Lit(Value),
    /// An operation applied to operands.
    Op(Operation, Vec<Expr>),
    /// A call to a named, env-bound function (e.g. a user [`crate::Fn`] lambda)
    /// applied to operands. Compiles just like [`Expr::Op`] but with an
    /// arbitrary function name in head position.
    Call(String, Vec<Expr>),
}

/// Anything convertible into an [`Expr`] operand: another `Expr`, a literal
/// scalar, or a [`Value`].
pub trait IntoExpr {
    fn into_expr(self) -> Expr;
}

impl IntoExpr for Expr {
    fn into_expr(self) -> Expr {
        self
    }
}
impl IntoExpr for &Expr {
    fn into_expr(self) -> Expr {
        self.clone()
    }
}
impl IntoExpr for Value {
    fn into_expr(self) -> Expr {
        Expr::Lit(self)
    }
}

macro_rules! into_expr_lit {
    ($($t:ty),*) => {$(
        impl IntoExpr for $t {
            fn into_expr(self) -> Expr { Expr::Lit(self.to_value()) }
        }
    )*};
}
into_expr_lit!(bool, u8, i16, i32, i64, f32, f64);

// In expressions, bare string literals become STRING atoms (matching the
// Python query compiler): the select/DAG path compares symbol columns against
// string scalars, whereas a symbol scalar raises a schema error. (This differs
// from the general `ToValue` path, where `&str` is a symbol.)
impl IntoExpr for &str {
    fn into_expr(self) -> Expr {
        Expr::Lit(Value::string(self))
    }
}
impl IntoExpr for String {
    fn into_expr(self) -> Expr {
        Expr::Lit(Value::string(&self))
    }
}
impl<S: AsRef<str>> IntoExpr for crate::Str<S> {
    fn into_expr(self) -> Expr {
        Expr::Lit(self.to_value())
    }
}

/// A column / global name reference.
pub fn col(name: impl Into<String>) -> Expr {
    Expr::Col(name.into())
}

/// A literal value from any [`ToValue`].
pub fn lit(v: impl ToValue) -> Expr {
    Expr::Lit(v.to_value())
}

impl Expr {
    /// Build an operation node.
    pub fn op(op: Operation, operands: Vec<Expr>) -> Expr {
        Expr::Op(op, operands)
    }

    fn binary(self, op: Operation, rhs: impl IntoExpr) -> Expr {
        Expr::Op(op, vec![self, rhs.into_expr()])
    }
    fn unary(self, op: Operation) -> Expr {
        Expr::Op(op, vec![self])
    }

    // ---- comparisons (methods: `==`/`<` can't return Expr in Rust) ----
    pub fn eq(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::Equals, rhs)
    }
    pub fn ne(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::NotEquals, rhs)
    }
    pub fn lt(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::LessThan, rhs)
    }
    pub fn le(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::LessEqual, rhs)
    }
    pub fn gt(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::GreaterThan, rhs)
    }
    pub fn ge(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::GreaterEqual, rhs)
    }
    pub fn like(self, pattern: impl IntoExpr) -> Expr {
        self.binary(Operation::Like, pattern)
    }
    pub fn is_in(self, set: impl IntoExpr) -> Expr {
        self.binary(Operation::In, set)
    }
    pub fn within(self, range: impl IntoExpr) -> Expr {
        self.binary(Operation::Within, range)
    }

    // ---- logical ----
    pub fn and(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::And, rhs)
    }
    pub fn or(self, rhs: impl IntoExpr) -> Expr {
        self.binary(Operation::Or, rhs)
    }

    // ---- aggregations / reductions ----
    pub fn sum(self) -> Expr {
        self.unary(Operation::Sum)
    }
    pub fn avg(self) -> Expr {
        self.unary(Operation::Avg)
    }
    pub fn count(self) -> Expr {
        self.unary(Operation::Count)
    }
    pub fn min(self) -> Expr {
        self.unary(Operation::Min)
    }
    pub fn max(self) -> Expr {
        self.unary(Operation::Max)
    }
    pub fn first(self) -> Expr {
        self.unary(Operation::First)
    }
    pub fn last(self) -> Expr {
        self.unary(Operation::Last)
    }
    pub fn median(self) -> Expr {
        self.unary(Operation::Median)
    }
    pub fn deviation(self) -> Expr {
        self.unary(Operation::Deviation)
    }
    pub fn std(self) -> Expr {
        self.unary(Operation::Stddev)
    }
    pub fn distinct(self) -> Expr {
        self.unary(Operation::Distinct)
    }

    // ---- math ----
    pub fn ceil(self) -> Expr {
        self.unary(Operation::Ceil)
    }
    pub fn floor(self) -> Expr {
        self.unary(Operation::Floor)
    }
    pub fn round(self) -> Expr {
        self.unary(Operation::Round)
    }

    // ---- collection ----
    /// Keep elements where `mask` is true.
    pub fn filter(self, mask: impl IntoExpr) -> Expr {
        self.binary(Operation::Filter, mask)
    }

    /// Lower this expression to a `Value` AST.
    pub fn compile(&self) -> Value {
        match self {
            Expr::Col(name) => Value::name_ref(name),
            Expr::Lit(v) => v.clone(),
            Expr::Op(op, operands) => {
                let mut items = Vec::with_capacity(operands.len() + 1);
                // Head is a name reference to the builtin (as the parser emits);
                // `ray_eval` resolves and applies it. Aggregations return a lazy
                // DAG result that the eval entry points materialize.
                items.push(Value::name_ref(op.as_str()));
                for o in operands {
                    items.push(o.compile());
                }
                Value::list(&items)
            }
            Expr::Call(name, operands) => {
                // Same shape as `Op`, but the head is a reference to a named
                // env binding (a user lambda bound by `Fn::apply`).
                let mut items = Vec::with_capacity(operands.len() + 1);
                items.push(Value::name_ref(name));
                for o in operands {
                    items.push(o.compile());
                }
                Value::list(&items)
            }
        }
    }

    /// Compile and evaluate this expression.
    pub fn execute(&self) -> Result<Value> {
        eval_value(&self.compile())
    }
}

// ---- operator overloads (arithmetic + logical) ----

macro_rules! bin_op {
    ($trait:ident, $method:ident, $op:expr) => {
        impl<R: IntoExpr> std::ops::$trait<R> for Expr {
            type Output = Expr;
            fn $method(self, rhs: R) -> Expr {
                Expr::Op($op, vec![self, rhs.into_expr()])
            }
        }
    };
}
bin_op!(Add, add, Operation::Add);
bin_op!(Sub, sub, Operation::Subtract);
bin_op!(Mul, mul, Operation::Multiply);
bin_op!(Div, div, Operation::Divide);
bin_op!(Rem, rem, Operation::Modulo);
bin_op!(BitAnd, bitand, Operation::And);
bin_op!(BitOr, bitor, Operation::Or);

impl std::ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        self.unary(Operation::Negate)
    }
}
impl std::ops::Not for Expr {
    type Output = Expr;
    fn not(self) -> Expr {
        self.unary(Operation::Not)
    }
}

// ---- free-function aggregations: `sum(col("x"))` ----

macro_rules! free_agg {
    ($name:ident, $op:expr) => {
        pub fn $name(e: impl IntoExpr) -> Expr {
            Expr::Op($op, vec![e.into_expr()])
        }
    };
}
free_agg!(sum, Operation::Sum);
free_agg!(avg, Operation::Avg);
free_agg!(count, Operation::Count);
free_agg!(min, Operation::Min);
free_agg!(max, Operation::Max);
free_agg!(first, Operation::First);
free_agg!(last, Operation::Last);
free_agg!(median, Operation::Median);
free_agg!(distinct, Operation::Distinct);
