//! jpp_core - JSONPath processor core library (RFC 9535)
//!
//! This library provides JSONPath query parsing and evaluation.
//!
//! # Example
//! ```
//! use serde_json::json;
//! use jpp_core::JsonPath;
//!
//! let path = JsonPath::parse("$.store.book[*].price").unwrap();
//! let json = json!({"store": {"book": [{"price": 10}, {"price": 20}]}});
//!
//! // Query returns references (zero-copy)
//! let results = path.query(&json);
//! assert_eq!(results, vec![&json!(10), &json!(20)]);
//!
//! // Clone if you need owned values
//! let owned: Vec<_> = results.into_iter().cloned().collect();
//! ```

pub mod ast;
pub mod eval;
pub mod lexer;
pub mod parser;

pub use ast::JsonPath;
use serde_json::Value;

impl JsonPath {
    /// Parse a JSONPath query string
    ///
    /// # Arguments
    /// * `jsonpath` - A JSONPath query string (e.g., "$.store.book[*].author")
    ///
    /// # Returns
    /// A parsed JsonPath ready for execution, or an error if the query is invalid
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use jpp_core::JsonPath;
    ///
    /// let path = JsonPath::parse("$.foo").unwrap();
    /// let json = json!({"foo": "bar"});
    /// let results = path.query(&json);
    /// assert_eq!(results, vec![&json!("bar")]);
    /// ```
    pub fn parse(jsonpath: &str) -> Result<Self, Error> {
        parser::Parser::parse(jsonpath).map_err(Error::from)
    }

    /// Execute the query and return references to matching values
    ///
    /// Returns references to the matched values within the input JSON.
    /// This is a zero-copy operation for maximum performance.
    ///
    /// # Example
    /// ```
    /// use serde_json::json;
    /// use jpp_core::JsonPath;
    ///
    /// let path = JsonPath::parse("$.items[*]").unwrap();
    /// let json = json!({"items": [1, 2, 3]});
    /// let results = path.query(&json);
    /// assert_eq!(results, vec![&json!(1), &json!(2), &json!(3)]);
    ///
    /// // Clone if you need owned values
    /// let owned: Vec<_> = results.into_iter().cloned().collect();
    /// ```
    pub fn query<'a>(&self, json: &'a Value) -> Vec<&'a Value> {
        eval::evaluate(self, json)
    }
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
/// This is a convenience function that parses and executes in one step.
/// For repeated queries, use [`JsonPath::parse`] to parse once and reuse.
///
/// # Arguments
/// * `jsonpath` - A JSONPath query string (e.g., "$.store.book[*].author")
/// * `json` - The JSON value to query
///
/// # Returns
/// A vector of references to matching JSON values, or an error if the query is invalid
///
/// # Example
/// ```
/// use serde_json::json;
/// use jpp_core::query;
///
/// let json = json!({"foo": "bar"});
/// let results = query("$.foo", &json).unwrap();
/// assert_eq!(results, vec![&json!("bar")]);
/// ```
pub fn query<'a>(jsonpath: &str, json: &'a Value) -> Result<Vec<&'a Value>, Error> {
    let path = parser::Parser::parse(jsonpath)?;
    Ok(eval::evaluate(&path, json))
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
        assert_eq!(results, vec![&json!("bar")]);
    }

    #[test]
    fn test_query_array() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[0]", &json).unwrap();
        assert_eq!(results, vec![&json!(1)]);
    }

    #[test]
    fn test_query_wildcard() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[*]", &json).unwrap();
        assert_eq!(results, vec![&json!(1), &json!(2), &json!(3)]);
    }

    #[test]
    fn test_query_invalid() {
        let json = json!({"foo": "bar"});
        let result = query("invalid", &json);
        assert!(result.is_err());
    }

    #[test]
    fn test_jsonpath_parse_and_query() {
        let path = JsonPath::parse("$.foo").unwrap();
        let json = json!({"foo": "bar"});
        let results = path.query(&json);
        assert_eq!(results, vec![&json!("bar")]);
    }

    #[test]
    fn test_jsonpath_reuse() {
        let path = JsonPath::parse("$.value").unwrap();
        let json1 = json!({"value": 1});
        let json2 = json!({"value": 2});
        assert_eq!(path.query(&json1), vec![&json!(1)]);
        assert_eq!(path.query(&json2), vec![&json!(2)]);
    }

    #[test]
    fn test_jsonpath_parse_invalid() {
        let result = JsonPath::parse("invalid");
        assert!(result.is_err());
    }
}
