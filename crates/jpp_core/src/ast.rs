//! AST definitions for JSONPath queries (RFC 9535)

use serde_json::Value;

/// A complete JSONPath query
#[derive(Debug, Clone, PartialEq)]
pub struct JsonPath {
    pub segments: Vec<Segment>,
}

/// A segment in a JSONPath query
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    /// Child segment (single dot: `.key` or `[selector]`)
    Child(Vec<Selector>),
    /// Descendant segment (double dot: `..key` or `..[selector]`)
    Descendant(Vec<Selector>),
}

/// A selector within a segment
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// Name selector: `.key` or `['key']`
    Name(String),
    /// Index selector: `[0]` or `[-1]`
    Index(i64),
    /// Wildcard selector: `*` or `[*]`
    Wildcard,
    /// Array slice selector: `[start:end:step]`
    Slice {
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    },
    /// Filter selector: `[?expr]`
    Filter(Box<Expr>),
}

/// An expression in a filter
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Current node reference: `@`
    CurrentNode,
    /// Root node reference: `$` (for absolute paths in filters)
    RootNode,
    /// Path expression relative to current/root node: `@.foo.bar` or `$.foo`
    Path {
        /// Starting point (CurrentNode or RootNode)
        start: Box<Expr>,
        /// Path segments to traverse
        segments: Vec<Segment>,
    },
    /// Literal value (with pre-cached JSON Value)
    Literal(CachedLiteral),
    /// Comparison expression: `@.price < 10`
    Comparison {
        left: Box<Expr>,
        op: CompOp,
        right: Box<Expr>,
    },
    /// Logical AND/OR expression: `@.a && @.b`
    Logical {
        left: Box<Expr>,
        op: LogicalOp,
        right: Box<Expr>,
    },
    /// Logical NOT expression: `!@.archived`
    Not(Box<Expr>),
    /// Function call: `length(@.items)`
    FunctionCall { name: String, args: Vec<Expr> },
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompOp {
    /// Equal: `==`
    Eq,
    /// Not equal: `!=`
    Ne,
    /// Less than: `<`
    Lt,
    /// Greater than: `>`
    Gt,
    /// Less than or equal: `<=`
    Le,
    /// Greater than or equal: `>=`
    Ge,
}

/// Logical operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    /// Logical AND: `&&`
    And,
    /// Logical OR: `||`
    Or,
}

/// Literal values in expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Number (integer or floating-point)
    Number(f64),
    /// String value
    String(String),
}

/// Literal with pre-computed JSON Value for efficient evaluation.
/// The cached_value is computed once at parse time, avoiding repeated
/// conversions during filter evaluation.
#[derive(Debug, Clone)]
pub struct CachedLiteral {
    /// The original literal value
    pub literal: Literal,
    /// Pre-computed serde_json::Value for fast evaluation
    pub cached_value: Value,
}

impl CachedLiteral {
    /// Create a new CachedLiteral with pre-computed Value
    #[inline]
    pub fn new(literal: Literal) -> Self {
        let cached_value = match &literal {
            Literal::Null => Value::Null,
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Number(n) => serde_json::Number::from_f64(*n)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Literal::String(s) => Value::String(s.clone()),
        };
        Self {
            literal,
            cached_value,
        }
    }
}

// PartialEq compares only the literal, ignoring cached_value
// (cached_value is deterministically derived from literal)
impl PartialEq for CachedLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.literal == other.literal
    }
}

impl JsonPath {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self { segments }
    }
}
