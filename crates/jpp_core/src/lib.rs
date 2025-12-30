//! jpp_core - JSONPath processor core library (RFC 9535)
//!
//! This library provides JSONPath query parsing and evaluation.

pub mod ast;
pub mod eval;
pub mod lexer;
pub mod parser;

use ast::JsonPath;
use serde_json::Value;

/// A pre-compiled JSONPath query for efficient repeated evaluation
///
/// Use [`compile`] to create a `CompiledPath`, then call [`query`](CompiledPath::query)
/// or [`query_ref`](CompiledPath::query_ref) to execute it against JSON values.
///
/// # Example
/// ```
/// use serde_json::json;
/// use jpp_core::compile;
///
/// let path = compile("$.foo").unwrap();
/// let json = json!({"foo": "bar"});
/// let results = path.query(&json);
/// assert_eq!(results, vec![json!("bar")]);
/// ```
pub struct CompiledPath {
    path: JsonPath,
}

impl CompiledPath {
    /// Execute the query and return owned values (cloned)
    pub fn query(&self, json: &Value) -> Vec<Value> {
        eval::evaluate(&self.path, json)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Execute the query and return references (zero-copy)
    pub fn query_ref<'a>(&self, json: &'a Value) -> Vec<&'a Value> {
        eval::evaluate(&self.path, json)
    }
}

/// Compile a JSONPath query for repeated use
///
/// This function parses the JSONPath query once, returning a [`CompiledPath`]
/// that can be efficiently executed multiple times against different JSON values.
///
/// # Arguments
/// * `jsonpath` - A JSONPath query string (e.g., "$.store.book[*].author")
///
/// # Returns
/// A compiled path ready for execution, or an error if the query is invalid
///
/// # Example
/// ```
/// use serde_json::json;
/// use jpp_core::compile;
///
/// let path = compile("$.store.book[*].price").unwrap();
///
/// let json1 = json!({"store": {"book": [{"price": 10}, {"price": 20}]}});
/// let json2 = json!({"store": {"book": [{"price": 30}]}});
///
/// assert_eq!(path.query(&json1), vec![json!(10), json!(20)]);
/// assert_eq!(path.query(&json2), vec![json!(30)]);
/// ```
pub fn compile(jsonpath: &str) -> Result<CompiledPath, Error> {
    let path = parser::Parser::parse(jsonpath)?;
    Ok(CompiledPath { path })
}

/// Error type for JSONPath operations
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<parser::ParseError> for Error {
    fn from(e: parser::ParseError) -> Self {
        Self {
            message: format!("parse error: {e}"),
        }
    }
}

/// Execute a JSONPath query against a JSON value
///
/// # Arguments
/// * `jsonpath` - A JSONPath query string (e.g., "$.store.book[*].author")
/// * `json` - The JSON value to query
///
/// # Returns
/// A vector of matching JSON values, or an error if the query is invalid
///
/// # Example
/// ```
/// use serde_json::json;
/// use jpp_core::query;
///
/// let json = json!({"foo": "bar"});
/// let results = query("$.foo", &json).unwrap();
/// assert_eq!(results, vec![json!("bar")]);
/// ```
pub fn query(jsonpath: &str, json: &Value) -> Result<Vec<Value>, Error> {
    let path = parser::Parser::parse(jsonpath)?;
    let results = eval::evaluate(&path, json);
    Ok(results.into_iter().cloned().collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_query_simple() {
        let json = json!({"foo": "bar"});
        let results = query("$.foo", &json).unwrap();
        assert_eq!(results, vec![json!("bar")]);
    }

    #[test]
    fn test_query_array() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[0]", &json).unwrap();
        assert_eq!(results, vec![json!(1)]);
    }

    #[test]
    fn test_query_wildcard() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[*]", &json).unwrap();
        assert_eq!(results, vec![json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn test_query_invalid() {
        let json = json!({"foo": "bar"});
        let result = query("invalid", &json);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_and_query() {
        let path = compile("$.foo").unwrap();
        let json = json!({"foo": "bar"});
        let results = path.query(&json);
        assert_eq!(results, vec![json!("bar")]);
    }

    #[test]
    fn test_compile_and_query_ref() {
        let path = compile("$.foo").unwrap();
        let json = json!({"foo": "bar"});
        let results = path.query_ref(&json);
        assert_eq!(results, vec![&json!("bar")]);
    }

    #[test]
    fn test_compile_reuse() {
        let path = compile("$.value").unwrap();
        let json1 = json!({"value": 1});
        let json2 = json!({"value": 2});
        assert_eq!(path.query(&json1), vec![json!(1)]);
        assert_eq!(path.query(&json2), vec![json!(2)]);
    }

    #[test]
    fn test_compile_invalid() {
        let result = compile("invalid");
        assert!(result.is_err());
    }
}
