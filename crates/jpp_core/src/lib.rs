//! jpp_core - JSONPath processor core library (RFC 9535)
//!
//! This library provides JSONPath query parsing and evaluation.

pub mod ast;
pub mod eval;
pub mod lexer;
pub mod parser;

use serde_json::Value;

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
}
