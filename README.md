# jpp

JSONPath processor written in Rust, compliant with [RFC 9535](https://datatracker.ietf.org/doc/html/rfc9535).

**[Demo](https://1gy.github.io/jpp/)**

## Installation

```bash
cargo install --path crates/jpp_cli
```

## Usage

```bash
# Query from file
jpp '$.store.book[*].author' data.json

# Query from stdin
cat data.json | jpp '$.store.book[*].author'
```

## Example

```bash
echo '{"items": [1, 2, 3]}' | jpp '$.items[*]'
# Output:
# [
#   1,
#   2,
#   3
# ]
```

## Development

```bash
just build    # Build
just test     # Run tests
just lint     # Run linter
just format   # Format code
```

## License

MIT
