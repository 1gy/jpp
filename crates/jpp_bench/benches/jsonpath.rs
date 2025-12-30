#![allow(clippy::unwrap_used)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use jpp_core::{JsonPath, query};
use serde_json::Value;

const SMALL_JSON: &str = include_str!("../data/small.json");
const MEDIUM_JSON: &str = include_str!("../data/medium.json");
const LARGE_JSON: &str = include_str!("../data/large.json");
const DEEP_JSON: &str = include_str!("../data/deep.json");

fn bench_basic_selectors(c: &mut Criterion) {
    let json: Value = serde_json::from_str(SMALL_JSON).unwrap();

    let mut group = c.benchmark_group("basic_selectors");

    let queries = [
        ("root", "$"),
        ("property", "$.store"),
        ("nested", "$.store.book"),
        ("index", "$.store.book[0]"),
        ("negative_index", "$.store.book[-1]"),
        ("wildcard", "$.store.book[*]"),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(BenchmarkId::new("small", name), &query_str, |b, q| {
            b.iter(|| query(black_box(*q), black_box(&json)))
        });
    }

    group.finish();
}

fn bench_advanced_selectors(c: &mut Criterion) {
    let json: Value = serde_json::from_str(SMALL_JSON).unwrap();

    let mut group = c.benchmark_group("advanced_selectors");

    let queries = [
        ("slice", "$.store.book[0:2]"),
        ("descendant", "$..author"),
        ("compound", "$.store.book[*].author"),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(BenchmarkId::new("small", name), &query_str, |b, q| {
            b.iter(|| query(black_box(*q), black_box(&json)))
        });
    }

    group.finish();
}

fn bench_filters(c: &mut Criterion) {
    let json: Value = serde_json::from_str(SMALL_JSON).unwrap();

    let mut group = c.benchmark_group("filters");

    let queries = [
        ("existence", "$.store.book[?@.isbn]"),
        ("comparison", "$.store.book[?@.price < 10]"),
        (
            "logical",
            r#"$.store.book[?@.price < 10 && @.category == "fiction"]"#,
        ),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(BenchmarkId::new("small", name), &query_str, |b, q| {
            b.iter(|| query(black_box(*q), black_box(&json)))
        });
    }

    group.finish();
}

fn bench_functions(c: &mut Criterion) {
    let json: Value = serde_json::from_str(SMALL_JSON).unwrap();

    let mut group = c.benchmark_group("functions");

    let queries = [
        ("length", "$.store.book[?length(@.title) > 10]"),
        ("match", r#"$.store.book[?match(@.author, "^J")]"#),
        ("search", r#"$.store.book[?search(@.title, "the")]"#),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(BenchmarkId::new("small", name), &query_str, |b, q| {
            b.iter(|| query(black_box(*q), black_box(&json)))
        });
    }

    group.finish();
}

fn bench_by_json_size(c: &mut Criterion) {
    let small: Value = serde_json::from_str(SMALL_JSON).unwrap();
    let medium: Value = serde_json::from_str(MEDIUM_JSON).unwrap();
    let large: Value = serde_json::from_str(LARGE_JSON).unwrap();

    let mut group = c.benchmark_group("json_size");

    let query_str = "$..price";

    group.throughput(Throughput::Bytes(SMALL_JSON.len() as u64));
    group.bench_function("small", |b| {
        b.iter(|| query(black_box(query_str), black_box(&small)))
    });

    group.throughput(Throughput::Bytes(MEDIUM_JSON.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| query(black_box(query_str), black_box(&medium)))
    });

    group.throughput(Throughput::Bytes(LARGE_JSON.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| query(black_box(query_str), black_box(&large)))
    });

    group.finish();
}

fn bench_descendant_chains(c: &mut Criterion) {
    let json: Value = serde_json::from_str(DEEP_JSON).unwrap();

    let mut group = c.benchmark_group("descendant_chains");

    let queries = [
        ("single", "$..value"),
        ("double", "$..a..value"),
        ("triple", "$..a..a..value"),
    ];

    for (name, query_str) in queries {
        group.bench_with_input(BenchmarkId::new("deep", name), &query_str, |b, q| {
            b.iter(|| query(black_box(*q), black_box(&json)))
        });
    }

    group.finish();
}

fn bench_comparison(c: &mut Criterion) {
    let json: Value = serde_json::from_str(SMALL_JSON).unwrap();

    let mut group = c.benchmark_group("comparison");

    // === Property access ===

    // jpp with parsing (includes parse time)
    group.bench_function("jpp/property", |b| {
        b.iter(|| query(black_box("$.store.book"), black_box(&json)))
    });

    // jpp pre-parsed (fair comparison, zero-copy)
    let jpp_property = JsonPath::parse("$.store.book").unwrap();
    group.bench_function("jpp_parsed/property", |b| {
        b.iter(|| jpp_property.query(black_box(&json)))
    });

    // serde_json_path (pre-parsed)
    let sjp_path = serde_json_path::JsonPath::parse("$.store.book").unwrap();
    group.bench_function("serde_json_path/property", |b| {
        b.iter(|| sjp_path.query(black_box(&json)))
    });

    // === Filter query ===

    // jpp with parsing (includes parse time)
    group.bench_function("jpp/filter", |b| {
        b.iter(|| query(black_box("$.store.book[?@.price < 10]"), black_box(&json)))
    });

    // jpp pre-parsed (fair comparison, zero-copy)
    let jpp_filter = JsonPath::parse("$.store.book[?@.price < 10]").unwrap();
    group.bench_function("jpp_parsed/filter", |b| {
        b.iter(|| jpp_filter.query(black_box(&json)))
    });

    // serde_json_path (pre-parsed)
    let sjp_filter = serde_json_path::JsonPath::parse("$.store.book[?@.price < 10]").unwrap();
    group.bench_function("serde_json_path/filter", |b| {
        b.iter(|| sjp_filter.query(black_box(&json)))
    });

    // === Descendant query ===

    // jpp with parsing (includes parse time)
    group.bench_function("jpp/descendant", |b| {
        b.iter(|| query(black_box("$..price"), black_box(&json)))
    });

    // jpp pre-parsed (fair comparison, zero-copy)
    let jpp_desc = JsonPath::parse("$..price").unwrap();
    group.bench_function("jpp_parsed/descendant", |b| {
        b.iter(|| jpp_desc.query(black_box(&json)))
    });

    // serde_json_path (pre-parsed)
    let sjp_desc = serde_json_path::JsonPath::parse("$..price").unwrap();
    group.bench_function("serde_json_path/descendant", |b| {
        b.iter(|| sjp_desc.query(black_box(&json)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_selectors,
    bench_advanced_selectors,
    bench_filters,
    bench_functions,
    bench_by_json_size,
    bench_descendant_chains,
    bench_comparison,
);
criterion_main!(benches);
