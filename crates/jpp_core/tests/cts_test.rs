//! JSONPath Compliance Test Suite (CTS) integration tests
//!
//! This module runs the official JSONPath CTS tests to validate
//! RFC 9535 compliance.

#![allow(clippy::expect_used)]

use jpp_core::query;
use serde::Deserialize;
use serde_json::Value;
use std::fs;

/// A single CTS test case
#[derive(Debug, Deserialize)]
struct CtsTest {
    name: String,
    selector: String,
    #[serde(default)]
    document: Value,
    #[serde(default)]
    result: Option<Vec<Value>>,
    #[serde(default)]
    results: Option<Vec<Vec<Value>>>,
    #[serde(default)]
    invalid_selector: bool,
    #[serde(default)]
    #[allow(dead_code)]
    tags: Vec<String>,
}

/// The CTS file structure
#[derive(Debug, Deserialize)]
struct CtsFile {
    tests: Vec<CtsTest>,
}

/// Run a single CTS test and return (passed, skip_reason)
fn run_cts_test(test: &CtsTest) -> (bool, Option<String>) {
    // If test expects invalid selector
    if test.invalid_selector {
        match query(&test.selector, &test.document) {
            Ok(_) => (
                false,
                Some("Expected parse error but query succeeded".to_string()),
            ),
            Err(_) => (true, None), // Correctly rejected invalid selector
        }
    } else {
        // Valid selector test
        match query(&test.selector, &test.document) {
            Ok(results) => {
                // Get expected results (handle both "result" and "results" fields)
                let expected = if let Some(ref result) = test.result {
                    result.clone()
                } else if let Some(ref results) = test.results {
                    // "results" contains multiple valid result sets, use first one
                    results.first().cloned().unwrap_or_default()
                } else {
                    vec![]
                };

                // Compare results (results is Vec<&Value>, expected is Vec<Value>)
                let expected_refs: Vec<&Value> = expected.iter().collect();
                if results == expected_refs {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!(
                            "Result mismatch:\n  got:      {:?}\n  expected: {:?}",
                            results, expected
                        )),
                    )
                }
            }
            Err(e) => (false, Some(format!("Unexpected parse error: {}", e))),
        }
    }
}

#[test]
fn run_cts_tests() {
    // Load CTS file
    let cts_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/cts/cts.json");
    let cts_content =
        fs::read_to_string(cts_path).expect("Failed to read CTS file. Run from project root.");

    let cts: CtsFile = serde_json::from_str(&cts_content).expect("Failed to parse CTS JSON");

    let total = cts.tests.len();
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_tests: Vec<(String, String)> = vec![];

    for test in &cts.tests {
        let (success, reason) = run_cts_test(test);
        if success {
            passed += 1;
        } else {
            failed += 1;
            if let Some(r) = reason {
                failed_tests.push((test.name.clone(), r));
            }
        }
    }

    // Print summary
    println!("\n========================================");
    println!("CTS Test Results");
    println!("========================================");
    println!("Total:  {}", total);
    println!(
        "Passed: {} ({:.1}%)",
        passed,
        (passed as f64 / total as f64) * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        failed,
        (failed as f64 / total as f64) * 100.0
    );
    println!("========================================\n");

    // Print first 20 failures for debugging
    if !failed_tests.is_empty() {
        println!("First {} failed tests:", failed_tests.len().min(20));
        for (name, reason) in failed_tests.iter().take(20) {
            println!("\n[FAIL] {}", name);
            println!("  {}", reason);
        }
        if failed_tests.len() > 20 {
            println!("\n... and {} more failures", failed_tests.len() - 20);
        }
    }

    // Enforce 100% CTS compliance - fail if any tests fail
    assert_eq!(
        failed, 0,
        "CTS tests failed: {} out of {} tests failed. RFC 9535 compliance must be maintained at 100%.",
        failed, total
    );
}

/// Test that CTS file loads correctly
#[test]
fn test_cts_file_loads() {
    let cts_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/cts/cts.json");
    let cts_content = fs::read_to_string(cts_path).expect("Failed to read CTS file");

    let cts: CtsFile = serde_json::from_str(&cts_content).expect("Failed to parse CTS JSON");

    assert!(!cts.tests.is_empty(), "CTS should have tests");
    println!("CTS contains {} tests", cts.tests.len());
}
