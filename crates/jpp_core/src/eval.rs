//! Evaluator for JSONPath queries

use crate::ast::{CompOp, Expr, JsonPath, Literal, LogicalOp, Segment, Selector};
use regex::Regex;
use serde_json::Value;

/// Result of evaluating an expression
#[derive(Debug, Clone)]
enum ExprResult {
    /// A single JSON value
    Value(Value),
    /// Multiple values from a path query
    NodeList(Vec<Value>),
    /// No result (missing property, failed comparison, etc.)
    Nothing,
}

impl ExprResult {
    /// Check if the result is truthy per RFC 9535 rules
    fn is_truthy(&self) -> bool {
        match self {
            ExprResult::NodeList(list) => !list.is_empty(),
            ExprResult::Value(v) => value_is_truthy(v),
            ExprResult::Nothing => false,
        }
    }

    /// Check if the result is singular (at most one value)
    /// RFC 9535: comparisons require singular queries on both sides
    fn is_singular(&self) -> bool {
        match self {
            ExprResult::Value(_) => true,
            ExprResult::NodeList(list) => list.len() <= 1,
            ExprResult::Nothing => true,
        }
    }

    /// Convert to a single value for comparison (takes first if NodeList)
    fn to_value(&self) -> Option<&Value> {
        match self {
            ExprResult::Value(v) => Some(v),
            ExprResult::NodeList(list) => list.first(),
            ExprResult::Nothing => None,
        }
    }
}

/// Check if a JSON value is truthy
fn value_is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
    }
}

/// Evaluate a JSONPath query against a JSON value
pub fn evaluate<'a>(path: &JsonPath, root: &'a Value) -> Vec<&'a Value> {
    let mut current = vec![root];

    for segment in &path.segments {
        current = evaluate_segment(segment, &current, root);
    }

    current
}

fn evaluate_segment<'a>(segment: &Segment, nodes: &[&'a Value], root: &'a Value) -> Vec<&'a Value> {
    match segment {
        Segment::Child(selectors) => {
            let mut results = Vec::new();
            for node in nodes {
                for selector in selectors {
                    results.extend(evaluate_selector(selector, node, root));
                }
            }
            results
        }
        Segment::Descendant(selectors) => {
            let mut results = Vec::new();
            for node in nodes {
                let descendants = collect_descendants(node);
                for desc in &descendants {
                    for selector in selectors {
                        results.extend(evaluate_selector(selector, desc, root));
                    }
                }
            }
            results
        }
    }
}

fn evaluate_selector<'a>(selector: &Selector, node: &'a Value, root: &'a Value) -> Vec<&'a Value> {
    match selector {
        Selector::Name(name) => {
            if let Value::Object(map) = node {
                map.get(name).into_iter().collect()
            } else {
                vec![]
            }
        }
        Selector::Index(idx) => {
            if let Value::Array(arr) = node {
                let index = normalize_index(*idx, arr.len());
                index.and_then(|i| arr.get(i)).into_iter().collect()
            } else {
                vec![]
            }
        }
        Selector::Wildcard => match node {
            Value::Array(arr) => arr.iter().collect(),
            Value::Object(map) => map.values().collect(),
            _ => vec![],
        },
        Selector::Slice { start, end, step } => {
            if let Value::Array(arr) = node {
                evaluate_slice(arr, *start, *end, *step)
            } else {
                vec![]
            }
        }
        Selector::Filter(expr) => evaluate_filter(expr, node, root),
    }
}

/// Evaluate a filter expression against a node
fn evaluate_filter<'a>(expr: &Expr, node: &'a Value, root: &'a Value) -> Vec<&'a Value> {
    match node {
        Value::Array(arr) => arr
            .iter()
            .filter(|elem| {
                let result = evaluate_expr(expr, elem, root);
                result.is_truthy()
            })
            .collect(),
        Value::Object(map) => map
            .values()
            .filter(|elem| {
                let result = evaluate_expr(expr, elem, root);
                result.is_truthy()
            })
            .collect(),
        _ => vec![],
    }
}

/// Evaluate an expression in filter context
fn evaluate_expr(expr: &Expr, current: &Value, root: &Value) -> ExprResult {
    match expr {
        Expr::CurrentNode => ExprResult::Value(current.clone()),
        Expr::RootNode => ExprResult::Value(root.clone()),
        Expr::Path { start, segments } => {
            let start_value = match start.as_ref() {
                Expr::CurrentNode => current,
                Expr::RootNode => root,
                _ => return ExprResult::Nothing,
            };
            let results = evaluate_path_segments(segments, start_value, root);
            if results.is_empty() {
                ExprResult::Nothing
            } else {
                // RFC 9535: Always return NodeList for paths.
                // For existence tests, truthiness is based on whether any nodes exist,
                // not on the value itself. This ensures [?@.a] matches {"a": null}
                // because the path selects a node (even if its value is null).
                ExprResult::NodeList(results.into_iter().cloned().collect())
            }
        }
        Expr::Literal(lit) => ExprResult::Value(literal_to_value(lit)),
        Expr::Comparison { left, op, right } => {
            let left_result = evaluate_expr(left, current, root);
            let right_result = evaluate_expr(right, current, root);
            let result = compare_values(&left_result, *op, &right_result);
            ExprResult::Value(Value::Bool(result))
        }
        Expr::Logical { left, op, right } => {
            let left_result = evaluate_expr(left, current, root);
            match op {
                LogicalOp::And => {
                    if !left_result.is_truthy() {
                        ExprResult::Value(Value::Bool(false))
                    } else {
                        let right_result = evaluate_expr(right, current, root);
                        ExprResult::Value(Value::Bool(right_result.is_truthy()))
                    }
                }
                LogicalOp::Or => {
                    if left_result.is_truthy() {
                        ExprResult::Value(Value::Bool(true))
                    } else {
                        let right_result = evaluate_expr(right, current, root);
                        ExprResult::Value(Value::Bool(right_result.is_truthy()))
                    }
                }
            }
        }
        Expr::Not(inner) => {
            let inner_result = evaluate_expr(inner, current, root);
            ExprResult::Value(Value::Bool(!inner_result.is_truthy()))
        }
        Expr::FunctionCall { name, args } => evaluate_function(name, args, current, root),
    }
}

/// Evaluate path segments starting from a value
fn evaluate_path_segments<'a>(
    segments: &[Segment],
    start: &'a Value,
    root: &'a Value,
) -> Vec<&'a Value> {
    let mut current = vec![start];
    for segment in segments {
        current = evaluate_segment_for_expr(segment, &current, root);
    }
    current
}

/// Evaluate a segment for expression path traversal
fn evaluate_segment_for_expr<'a>(
    segment: &Segment,
    nodes: &[&'a Value],
    root: &'a Value,
) -> Vec<&'a Value> {
    match segment {
        Segment::Child(selectors) => {
            let mut results = Vec::new();
            for node in nodes {
                for selector in selectors {
                    results.extend(evaluate_selector_in_path(selector, node, root));
                }
            }
            results
        }
        Segment::Descendant(selectors) => {
            let mut results = Vec::new();
            for node in nodes {
                let descendants = collect_descendants(node);
                for desc in &descendants {
                    for selector in selectors {
                        results.extend(evaluate_selector_in_path(selector, desc, root));
                    }
                }
            }
            results
        }
    }
}

/// Evaluate a selector within a path expression (supports nested filters)
fn evaluate_selector_in_path<'a>(
    selector: &Selector,
    node: &'a Value,
    root: &'a Value,
) -> Vec<&'a Value> {
    match selector {
        Selector::Name(name) => {
            if let Value::Object(map) = node {
                map.get(name).into_iter().collect()
            } else {
                vec![]
            }
        }
        Selector::Index(idx) => {
            if let Value::Array(arr) = node {
                let index = normalize_index(*idx, arr.len());
                index.and_then(|i| arr.get(i)).into_iter().collect()
            } else {
                vec![]
            }
        }
        Selector::Wildcard => match node {
            Value::Array(arr) => arr.iter().collect(),
            Value::Object(map) => map.values().collect(),
            _ => vec![],
        },
        Selector::Slice { start, end, step } => {
            if let Value::Array(arr) = node {
                evaluate_slice(arr, *start, *end, *step)
            } else {
                vec![]
            }
        }
        Selector::Filter(expr) => {
            // Nested filter: evaluate the filter expression against node's children
            match node {
                Value::Array(arr) => arr
                    .iter()
                    .filter(|elem| {
                        let result = evaluate_expr(expr, elem, root);
                        result.is_truthy()
                    })
                    .collect(),
                Value::Object(map) => map
                    .values()
                    .filter(|elem| {
                        let result = evaluate_expr(expr, elem, root);
                        result.is_truthy()
                    })
                    .collect(),
                _ => vec![],
            }
        }
    }
}

/// Convert a Literal to a JSON Value
fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Null => Value::Null,
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Number(n) => Value::Number((*n).into()),
        Literal::String(s) => Value::String(s.clone()),
    }
}

/// Evaluate a built-in function call
fn evaluate_function(name: &str, args: &[Expr], current: &Value, root: &Value) -> ExprResult {
    match name {
        "length" => fn_length(args, current, root),
        "count" => fn_count(args, current, root),
        "value" => fn_value(args, current, root),
        "match" => fn_match(args, current, root),
        "search" => fn_search(args, current, root),
        _ => ExprResult::Nothing, // Unknown function
    }
}

/// RFC 9535 length() function: returns length of string, array, or object
fn fn_length(args: &[Expr], current: &Value, root: &Value) -> ExprResult {
    if args.len() != 1 {
        return ExprResult::Nothing;
    }

    let arg = evaluate_expr(&args[0], current, root);
    match arg.to_value() {
        Some(Value::String(s)) => ExprResult::Value(Value::Number(s.len().into())),
        Some(Value::Array(arr)) => ExprResult::Value(Value::Number(arr.len().into())),
        Some(Value::Object(obj)) => ExprResult::Value(Value::Number(obj.len().into())),
        _ => ExprResult::Nothing,
    }
}

/// RFC 9535 count() function: returns count of nodes in a nodelist
fn fn_count(args: &[Expr], current: &Value, root: &Value) -> ExprResult {
    if args.len() != 1 {
        return ExprResult::Nothing;
    }

    let arg = evaluate_expr(&args[0], current, root);
    let count = match &arg {
        ExprResult::NodeList(list) => list.len(),
        ExprResult::Value(_) => 1,
        ExprResult::Nothing => 0,
    };
    ExprResult::Value(Value::Number(count.into()))
}

/// RFC 9535 value() function: returns the value if exactly one node, Nothing otherwise
fn fn_value(args: &[Expr], current: &Value, root: &Value) -> ExprResult {
    if args.len() != 1 {
        return ExprResult::Nothing;
    }

    let arg = evaluate_expr(&args[0], current, root);
    match arg {
        ExprResult::Value(v) => ExprResult::Value(v),
        ExprResult::NodeList(list) if list.len() == 1 => ExprResult::Value(list[0].clone()),
        _ => ExprResult::Nothing,
    }
}

/// RFC 9535 match() function: returns true if string matches regex (full match)
fn fn_match(args: &[Expr], current: &Value, root: &Value) -> ExprResult {
    if args.len() != 2 {
        return ExprResult::Nothing;
    }

    let string_arg = evaluate_expr(&args[0], current, root);
    let pattern_arg = evaluate_expr(&args[1], current, root);

    let string = match string_arg.to_value() {
        Some(Value::String(s)) => s.clone(),
        _ => return ExprResult::Value(Value::Bool(false)),
    };

    let pattern = match pattern_arg.to_value() {
        Some(Value::String(p)) => p.clone(),
        _ => return ExprResult::Value(Value::Bool(false)),
    };

    // Create anchored regex for full match
    let anchored_pattern = format!("^(?:{})$", pattern);
    match Regex::new(&anchored_pattern) {
        Ok(re) => ExprResult::Value(Value::Bool(re.is_match(&string))),
        Err(_) => ExprResult::Value(Value::Bool(false)),
    }
}

/// RFC 9535 search() function: returns true if regex pattern found anywhere in string
fn fn_search(args: &[Expr], current: &Value, root: &Value) -> ExprResult {
    if args.len() != 2 {
        return ExprResult::Nothing;
    }

    let string_arg = evaluate_expr(&args[0], current, root);
    let pattern_arg = evaluate_expr(&args[1], current, root);

    let string = match string_arg.to_value() {
        Some(Value::String(s)) => s.clone(),
        _ => return ExprResult::Value(Value::Bool(false)),
    };

    let pattern = match pattern_arg.to_value() {
        Some(Value::String(p)) => p.clone(),
        _ => return ExprResult::Value(Value::Bool(false)),
    };

    match Regex::new(&pattern) {
        Ok(re) => ExprResult::Value(Value::Bool(re.is_match(&string))),
        Err(_) => ExprResult::Value(Value::Bool(false)),
    }
}

/// Compare two expression results with the given operator
/// Per RFC 9535: comparisons require singular queries on both sides
fn compare_values(left: &ExprResult, op: CompOp, right: &ExprResult) -> bool {
    // RFC 9535: Non-singular queries in comparisons always return false
    if !left.is_singular() || !right.is_singular() {
        return false;
    }

    let left_val = left.to_value();
    let right_val = right.to_value();

    match (left_val, right_val) {
        (Some(l), Some(r)) => compare_json_values(l, op, r),
        // If either side is Nothing (missing), comparison always returns false
        // This matches RFC 9535 behavior where missing values don't match anything
        _ => false,
    }
}

/// Compare two JSON values
fn compare_json_values(left: &Value, op: CompOp, right: &Value) -> bool {
    match op {
        CompOp::Eq => values_equal(left, right),
        CompOp::Ne => !values_equal(left, right),
        CompOp::Lt => values_less_than(left, right),
        CompOp::Gt => values_less_than(right, left),
        CompOp::Le => values_equal(left, right) || values_less_than(left, right),
        CompOp::Ge => values_equal(left, right) || values_less_than(right, left),
    }
}

/// Check if two JSON values are equal
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Number(l), Value::Number(r)) => {
            // Compare as f64 for consistency
            l.as_f64() == r.as_f64()
        }
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Array(l), Value::Array(r)) => l == r,
        (Value::Object(l), Value::Object(r)) => l == r,
        _ => false, // Different types are never equal
    }
}

/// Check if left < right (only for comparable types)
fn values_less_than(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => match (l.as_f64(), r.as_f64()) {
            (Some(lf), Some(rf)) => lf < rf,
            _ => false,
        },
        (Value::String(l), Value::String(r)) => l < r,
        _ => false, // Non-comparable types
    }
}

fn normalize_index(idx: i64, len: usize) -> Option<usize> {
    let len_i64 = len as i64;
    if idx >= 0 {
        let i = idx as usize;
        if i < len { Some(i) } else { None }
    } else {
        let normalized = len_i64 + idx;
        if normalized >= 0 {
            Some(normalized as usize)
        } else {
            None
        }
    }
}

fn evaluate_slice(
    arr: &[Value],
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>,
) -> Vec<&Value> {
    let len = arr.len() as i64;
    let step = step.unwrap_or(1);

    if step == 0 {
        return vec![];
    }

    let (start, end) = if step > 0 {
        let start = start.map(|s| normalize_slice_bound(s, len)).unwrap_or(0);
        let end = end.map(|e| normalize_slice_bound(e, len)).unwrap_or(len);
        (start.max(0), end.min(len))
    } else {
        let start = start
            .map(|s| normalize_slice_bound(s, len))
            .unwrap_or(len - 1);
        let end = end.map(|e| normalize_slice_bound(e, len)).unwrap_or(-1);
        (start.min(len - 1), end.max(-1))
    };

    let mut results = Vec::new();

    if step > 0 {
        let mut i = start;
        while i < end {
            if i >= 0 && (i as usize) < arr.len() {
                results.push(&arr[i as usize]);
            }
            i += step;
        }
    } else {
        let mut i = start;
        while i > end {
            if i >= 0 && (i as usize) < arr.len() {
                results.push(&arr[i as usize]);
            }
            i += step;
        }
    }

    results
}

fn normalize_slice_bound(bound: i64, len: i64) -> i64 {
    if bound >= 0 {
        bound
    } else {
        (len + bound).max(0)
    }
}

fn collect_descendants(node: &Value) -> Vec<&Value> {
    let mut results = Vec::new();
    let mut stack = vec![node];

    while let Some(current) = stack.pop() {
        results.push(current);
        match current {
            Value::Array(arr) => {
                // Push in reverse order to maintain traversal order
                stack.extend(arr.iter().rev());
            }
            Value::Object(map) => {
                // Push in reverse order to maintain traversal order
                stack.extend(map.values().rev());
            }
            _ => {}
        }
    }
    results
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use serde_json::json;

    fn query(path: &str, json: &Value) -> Vec<Value> {
        let parsed = Parser::parse(path).unwrap();
        evaluate(&parsed, json).into_iter().cloned().collect()
    }

    #[test]
    fn test_root_only() {
        let json = json!({"foo": "bar"});
        let results = query("$", &json);
        assert_eq!(results, vec![json!({"foo": "bar"})]);
    }

    #[test]
    fn test_simple_name() {
        let json = json!({"foo": "bar"});
        let results = query("$.foo", &json);
        assert_eq!(results, vec![json!("bar")]);
    }

    #[test]
    fn test_nested_name() {
        let json = json!({"foo": {"bar": "baz"}});
        let results = query("$.foo.bar", &json);
        assert_eq!(results, vec![json!("baz")]);
    }

    #[test]
    fn test_array_index() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[0]", &json);
        assert_eq!(results, vec![json!(1)]);
    }

    #[test]
    fn test_negative_index() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[-1]", &json);
        assert_eq!(results, vec![json!(3)]);
    }

    #[test]
    fn test_wildcard_array() {
        let json = json!({"arr": [1, 2, 3]});
        let results = query("$.arr[*]", &json);
        assert_eq!(results, vec![json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn test_wildcard_object() {
        let json = json!({"a": 1, "b": 2});
        let results = query("$.*", &json);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_descendant() {
        let json = json!({
            "store": {
                "book": [
                    {"author": "Author1"},
                    {"author": "Author2"}
                ]
            }
        });
        let results = query("$..author", &json);
        assert_eq!(results, vec![json!("Author1"), json!("Author2")]);
    }

    #[test]
    fn test_slice() {
        let json = json!({"arr": [0, 1, 2, 3, 4]});
        let results = query("$.arr[1:3]", &json);
        assert_eq!(results, vec![json!(1), json!(2)]);
    }

    #[test]
    fn test_complex_path() {
        let json = json!({
            "store": {
                "book": [
                    {"title": "Book1", "price": 10},
                    {"title": "Book2", "price": 20}
                ]
            }
        });
        let results = query("$.store.book[0].title", &json);
        assert_eq!(results, vec![json!("Book1")]);
    }

    // ========== Filter Expression Tests ==========

    #[test]
    fn test_filter_existence() {
        let json = json!({
            "items": [
                {"name": "apple", "price": 5},
                {"name": "banana"},
                {"name": "cherry", "price": 15}
            ]
        });
        let results = query("$.items[?@.price]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple");
        assert_eq!(results[1]["name"], "cherry");
    }

    #[test]
    fn test_filter_comparison_less_than() {
        let json = json!({
            "items": [
                {"name": "apple", "price": 5},
                {"name": "banana", "price": 10},
                {"name": "cherry", "price": 15}
            ]
        });
        let results = query("$.items[?@.price < 10]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["name"], "apple");
    }

    #[test]
    fn test_filter_comparison_equal() {
        let json = json!({
            "items": [
                {"name": "apple", "price": 5},
                {"name": "banana", "price": 10},
                {"name": "cherry", "price": 15}
            ]
        });
        let results = query("$.items[?@.price == 10]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["name"], "banana");
    }

    #[test]
    fn test_filter_comparison_string() {
        let json = json!({
            "items": [
                {"name": "apple"},
                {"name": "banana"},
                {"name": "cherry"}
            ]
        });
        let results = query("$.items[?@.name == \"banana\"]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["name"], "banana");
    }

    #[test]
    fn test_filter_logical_and() {
        let json = json!({
            "items": [
                {"name": "apple", "price": 5, "available": true},
                {"name": "banana", "price": 10, "available": false},
                {"name": "cherry", "price": 8, "available": true}
            ]
        });
        let results = query("$.items[?@.price < 10 && @.available == true]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple");
        assert_eq!(results[1]["name"], "cherry");
    }

    #[test]
    fn test_filter_logical_or() {
        let json = json!({
            "items": [
                {"name": "apple", "price": 5},
                {"name": "banana", "price": 10},
                {"name": "cherry", "price": 15}
            ]
        });
        let results = query("$.items[?@.price < 6 || @.price > 14]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple");
        assert_eq!(results[1]["name"], "cherry");
    }

    #[test]
    fn test_filter_not() {
        // RFC 9535: [?!@.archived] matches items where 'archived' does NOT exist
        // (negates existence test, not the value)
        let json = json!({
            "items": [
                {"name": "apple", "archived": false},
                {"name": "banana", "archived": true},
                {"name": "cherry"}
            ]
        });
        let results = query("$.items[?!@.archived]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["name"], "cherry");

        // To filter by value being false, use comparison:
        let results_false = query("$.items[?@.archived == false]", &json);
        assert_eq!(results_false.len(), 1);
        assert_eq!(results_false[0]["name"], "apple");
    }

    #[test]
    fn test_filter_null_comparison() {
        let json = json!({
            "items": [
                {"name": "apple", "discount": null},
                {"name": "banana", "discount": 5},
                {"name": "cherry"}
            ]
        });
        let results = query("$.items[?@.discount != null]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["name"], "banana");
    }

    #[test]
    fn test_filter_greater_equal() {
        let json = json!({
            "items": [
                {"name": "apple", "price": 5},
                {"name": "banana", "price": 10},
                {"name": "cherry", "price": 15}
            ]
        });
        let results = query("$.items[?@.price >= 10]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "banana");
        assert_eq!(results[1]["name"], "cherry");
    }

    #[test]
    fn test_filter_nested_path() {
        let json = json!({
            "items": [
                {"name": "apple", "info": {"category": "fruit"}},
                {"name": "carrot", "info": {"category": "vegetable"}},
                {"name": "banana", "info": {"category": "fruit"}}
            ]
        });
        let results = query("$.items[?@.info.category == \"fruit\"]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple");
        assert_eq!(results[1]["name"], "banana");
    }

    // ========== Built-in Function Tests ==========

    #[test]
    fn test_function_length_string() {
        let json = json!({
            "items": [
                {"name": "apple"},
                {"name": "banana"},
                {"name": "fig"}
            ]
        });
        // Filter items where name length > 4
        let results = query("$.items[?length(@.name) > 4]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple");
        assert_eq!(results[1]["name"], "banana");
    }

    #[test]
    fn test_function_length_array() {
        let json = json!({
            "items": [
                {"name": "a", "tags": [1, 2, 3]},
                {"name": "b", "tags": [1]},
                {"name": "c", "tags": [1, 2, 3, 4, 5]}
            ]
        });
        // Filter items where tags array length >= 3
        let results = query("$.items[?length(@.tags) >= 3]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "a");
        assert_eq!(results[1]["name"], "c");
    }

    #[test]
    fn test_function_length_object() {
        let json = json!({
            "items": [
                {"name": "a", "props": {"x": 1}},
                {"name": "b", "props": {"x": 1, "y": 2, "z": 3}},
                {"name": "c", "props": {}}
            ]
        });
        // Filter items where props object has > 1 key
        let results = query("$.items[?length(@.props) > 1]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["name"], "b");
    }

    #[test]
    fn test_function_match() {
        let json = json!({
            "items": [
                {"name": "apple"},
                {"name": "apricot"},
                {"name": "banana"}
            ]
        });
        // Filter items where name matches pattern "ap.*"
        let results = query("$.items[?match(@.name, \"ap.*\")]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple");
        assert_eq!(results[1]["name"], "apricot");
    }

    #[test]
    fn test_function_search() {
        let json = json!({
            "items": [
                {"name": "apple pie"},
                {"name": "banana"},
                {"name": "pineapple"}
            ]
        });
        // Filter items where name contains "apple"
        let results = query("$.items[?search(@.name, \"apple\")]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "apple pie");
        assert_eq!(results[1]["name"], "pineapple");
    }

    #[test]
    fn test_function_match_vs_search() {
        let json = json!({
            "items": [
                {"name": "test"},
                {"name": "testing"},
                {"name": "a test here"}
            ]
        });
        // match() requires full match
        let match_results = query("$.items[?match(@.name, \"test\")]", &json);
        assert_eq!(match_results.len(), 1);
        assert_eq!(match_results[0]["name"], "test");

        // search() finds substring
        let search_results = query("$.items[?search(@.name, \"test\")]", &json);
        assert_eq!(search_results.len(), 3);
    }

    // ========== Null Existence Semantics Tests ==========

    #[test]
    fn test_existence_with_null_value() {
        // RFC 9535: [?@.a] should match if 'a' exists, even if its value is null
        let json = json!({
            "items": [
                {"a": null},
                {"a": 1},
                {"b": 2}
            ]
        });
        let results = query("$.items[?@.a]", &json);
        // Both {"a": null} and {"a": 1} should match (a exists in both)
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], json!({"a": null}));
        assert_eq!(results[1], json!({"a": 1}));
    }

    #[test]
    fn test_null_comparison_equal() {
        let json = json!({
            "items": [
                {"a": null},
                {"a": 1},
                {"b": 2}
            ]
        });
        // [?@.a == null] should only match {"a": null}
        let results = query("$.items[?@.a == null]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!({"a": null}));
    }

    #[test]
    fn test_null_comparison_not_equal() {
        let json = json!({
            "items": [
                {"a": null},
                {"a": 1},
                {"b": 2}
            ]
        });
        // [?@.a != null] should only match {"a": 1}
        // (missing 'a' returns Nothing, which doesn't match in comparisons)
        let results = query("$.items[?@.a != null]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!({"a": 1}));
    }

    // ========== Nested Filter Tests ==========

    #[test]
    fn test_nested_filter_basic() {
        // $[?@[?@.a]] - select elements that have children with property 'a'
        let json = json!([
            [{"a": 1}, {"b": 2}],
            [{"b": 3}],
            [{"a": 4}, {"a": 5}]
        ]);
        let results = query("$[?@[?@.a]]", &json);
        // First and third arrays have children with property 'a'
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], json!([{"a": 1}, {"b": 2}]));
        assert_eq!(results[1], json!([{"a": 4}, {"a": 5}]));
    }

    #[test]
    fn test_nested_filter_with_comparison() {
        let json = json!([
            [{"x": 1}, {"x": 10}],
            [{"x": 5}],
            [{"x": 20}, {"x": 30}]
        ]);
        // Select arrays that have at least one element with x > 15
        let results = query("$[?@[?@.x > 15]]", &json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!([{"x": 20}, {"x": 30}]));
    }

    #[test]
    fn test_nested_filter_in_path() {
        let json = json!({
            "data": [
                {"items": [{"valid": true}, {"valid": false}]},
                {"items": [{"valid": false}]},
                {"items": [{"valid": true}]}
            ]
        });
        // Select data items that have at least one valid item
        let results = query("$.data[?@.items[?@.valid == true]]", &json);
        assert_eq!(results.len(), 2);
    }

    // ========== Non-Singular Query Comparison Tests ==========

    #[test]
    fn test_non_singular_wildcard_comparison_rejected() {
        // RFC 9535: @[*] returning multiple values is non-singular, comparison returns false
        let json = json!([[1, 2, 3], [1, 1], [5, 6]]);
        // $[?@[*] == 1] should not match arrays with multiple elements
        // because @[*] returns multiple values, making it non-singular
        let results = query("$[?@[*] == 1]", &json);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_singular_single_element_array_works() {
        // Single-element array with wildcard returns 1 value (singular at runtime)
        let json = json!([[1], [2], [1]]);
        // @[*] on single-element arrays returns 1 value, so comparison works
        let results = query("$[?@[*] == 1]", &json);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_singular_index_comparison_works() {
        // @[0] is singular, comparison should work
        let json = json!([[1, 2, 3], [5, 6], [1, 7]]);
        // $[?@[0] == 1] should match arrays whose first element is 1
        let results = query("$[?@[0] == 1]", &json);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], json!([1, 2, 3]));
        assert_eq!(results[1], json!([1, 7]));
    }

    #[test]
    fn test_singular_property_comparison_works() {
        // @.a is singular, comparison should work
        let json = json!([
            {"a": 1, "b": 2},
            {"a": 5},
            {"a": 1}
        ]);
        let results = query("$[?@.a == 1]", &json);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_non_singular_on_right_side_rejected() {
        // Non-singular on right side should also be rejected
        let json = json!({
            "items": [
                {"val": 1, "arr": [1, 2]},
                {"val": 2, "arr": [2, 3]}
            ]
        });
        // Comparison with non-singular on right side
        let results = query("$.items[?@.val == @.arr[*]]", &json);
        assert_eq!(results.len(), 0);
    }
}
