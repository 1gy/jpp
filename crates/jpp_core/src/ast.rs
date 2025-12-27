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
}

impl JsonPath {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self { segments }
    }
}
