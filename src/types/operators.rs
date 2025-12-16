//! Rayforce built-in operations.

use crate::ffi::{self, RayObj};
use crate::*;

/// Built-in Rayforce operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Negate,

    // Comparison
    Equals,
    NotEquals,
    GreaterThan,
    GreaterEqual,
    LessThan,
    LessEqual,

    // Logical
    And,
    Or,
    Not,

    // Aggregation
    Sum,
    Avg,
    Count,
    Min,
    Max,
    First,
    Last,
    Median,
    Deviation,

    // Statistical
    XBar,

    // Math
    Ceil,
    Floor,
    Round,

    // Collection
    In,
    Distinct,

    // Query
    Select,
    Insert,
    Where,

    // Join
    InnerJoin,
    LeftJoin,
    WindowJoin,
    WindowJoin1,

    // Sort
    Asc,
    Desc,
    XAsc,
    XDesc,
    IAsc,
    IDesc,

    // Accessor
    At,

    // Functional
    Map,
    MapLeft,

    // Composition
    Til,

    // Type
    ListOp,

    // Other
    Eval,
    Quote,
    Concat,
    ReadCsv,
    Set,
    SetSplayed,
    GetSplayed,
    GetParted,
}

impl Operation {
    /// Get the Rayforce operator name.
    pub fn name(&self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "-",
            Operation::Multiply => "*",
            Operation::Divide => "/",
            Operation::Modulo => "%",
            Operation::Negate => "neg",
            Operation::Equals => "==",
            Operation::NotEquals => "!=",
            Operation::GreaterThan => ">",
            Operation::GreaterEqual => ">=",
            Operation::LessThan => "<",
            Operation::LessEqual => "<=",
            Operation::And => "and",
            Operation::Or => "or",
            Operation::Not => "not",
            Operation::Sum => "sum",
            Operation::Avg => "avg",
            Operation::Count => "count",
            Operation::Min => "min",
            Operation::Max => "max",
            Operation::First => "first",
            Operation::Last => "last",
            Operation::Median => "med",
            Operation::Deviation => "dev",
            Operation::XBar => "xbar",
            Operation::Ceil => "ceil",
            Operation::Floor => "floor",
            Operation::Round => "round",
            Operation::In => "in",
            Operation::Distinct => "distinct",
            Operation::Select => "select",
            Operation::Insert => "insert",
            Operation::Where => "where",
            Operation::InnerJoin => "inner-join",
            Operation::LeftJoin => "left-join",
            Operation::WindowJoin => "window-join",
            Operation::WindowJoin1 => "window-join1",
            Operation::Asc => "asc",
            Operation::Desc => "desc",
            Operation::XAsc => "xasc",
            Operation::XDesc => "xdesc",
            Operation::IAsc => "iasc",
            Operation::IDesc => "idesc",
            Operation::At => "at",
            Operation::Map => "map",
            Operation::MapLeft => "map-left",
            Operation::Til => "til",
            Operation::ListOp => "list",
            Operation::Eval => "eval",
            Operation::Quote => "quote",
            Operation::Concat => "concat",
            Operation::ReadCsv => "read-csv",
            Operation::Set => "set",
            Operation::SetSplayed => "set-splayed",
            Operation::GetSplayed => "get-splayed",
            Operation::GetParted => "get-parted",
        }
    }

    /// Get the operation as a RayObj (internal function reference).
    pub fn to_ray_obj(&self) -> Option<RayObj> {
        ffi::get_internal_function(self.name())
    }

    /// Check if this is a binary operation.
    pub fn is_binary(&self) -> bool {
        if let Some(obj) = self.to_ray_obj() {
            obj.type_code() == TYPE_BINARY as i8
        } else {
            false
        }
    }

    /// Check if this is a unary operation.
    pub fn is_unary(&self) -> bool {
        if let Some(obj) = self.to_ray_obj() {
            obj.type_code() == TYPE_UNARY as i8
        } else {
            false
        }
    }

    /// Check if this is a variadic operation.
    pub fn is_variadic(&self) -> bool {
        if let Some(obj) = self.to_ray_obj() {
            obj.type_code() == TYPE_VARY as i8
        } else {
            false
        }
    }
}

impl From<Operation> for RayObj {
    fn from(op: Operation) -> Self {
        op.to_ray_obj().unwrap_or_else(|| ffi::new_symbol(op.name()))
    }
}

