//! AST definitions for JSONPath queries (RFC 9535)

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
    /// Literal value
    Literal(Literal),
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
    /// Integer number
    Number(i64),
    /// String value
    String(String),
}

impl JsonPath {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self { segments }
    }
}
