//! Evaluator for JSONPath queries

use crate::ast::{JsonPath, Segment, Selector};
use serde_json::Value;

/// Evaluate a JSONPath query against a JSON value
pub fn evaluate<'a>(path: &JsonPath, root: &'a Value) -> Vec<&'a Value> {
    let mut current = vec![root];

    for segment in &path.segments {
        current = evaluate_segment(segment, &current, root);
    }

    current
}

fn evaluate_segment<'a>(
    segment: &Segment,
    nodes: &[&'a Value],
    _root: &'a Value,
) -> Vec<&'a Value> {
    match segment {
        Segment::Child(selectors) => {
            let mut results = Vec::new();
            for node in nodes {
                for selector in selectors {
                    results.extend(evaluate_selector(selector, node));
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
                        results.extend(evaluate_selector(selector, desc));
                    }
                }
            }
            results
        }
    }
}

fn evaluate_selector<'a>(selector: &Selector, node: &'a Value) -> Vec<&'a Value> {
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
}
