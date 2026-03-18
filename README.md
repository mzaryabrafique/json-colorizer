# json-colorizer

A fast, lightweight JSON formatter, pretty-printer, colorizer, and query tool for Rust — with jq-style dot-path queries.

[![Crates.io](https://img.shields.io/crates/v/json-colorizer.svg)](https://crates.io/crates/json-colorizer)
[![Docs.rs](https://docs.rs/json-colorizer/badge.svg)](https://docs.rs/json-colorizer)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- 🎨 **Syntax-colored** JSON output (cyan keys, green strings, yellow numbers, magenta bools, red null)
- 📐 **Pretty-print** with configurable indentation
- 📦 **Compact mode** (minified single-line output)
- 🔍 **jq-style queries** — `.data.users[0].name`
- 📄 Read from **stdin or file**
- 🔌 Use as a **library** or a **CLI tool**

## Install (CLI)

```bash
cargo install json-colorizer
```

## CLI Usage

```bash
# Pretty-print with colors
echo '{"name":"Alice","scores":[95,87]}' | json-colorizer

# Compact output
echo '{"name":"Alice"}' | json-colorizer --compact

# Query nested values
echo '{"data":{"users":[{"name":"Alice"}]}}' | json-colorizer -q '.data.users[0].name'

# Read from file
json-colorizer package.json
json-colorizer data.json -q '.results' --compact
```

## Library Usage

Add to your `Cargo.toml` (no CLI dependencies pulled in):

```toml
[dependencies]
json-colorizer = { version = "0.1", default-features = false }
```

```rust
use json_colorizer::{format_json, format_json_compact, query, FormatOptions};
use serde_json::json;

let data = json!({
    "users": [
        {"name": "Alice", "score": 95},
        {"name": "Bob", "score": 87}
    ]
});

// Pretty-print (with ANSI colors)
println!("{}", format_json(&data, &FormatOptions::default()));

// Pretty-print (no colors, custom indent)
let opts = FormatOptions { indent: 4, color: false };
println!("{}", format_json(&data, &opts));

// Compact
println!("{}", format_json_compact(&data));

// Query
let name = query(&data, ".users[0].name").unwrap();
assert_eq!(name, &json!("Alice"));
```

## API

| Function | Description |
|----------|-------------|
| `format_json(value, opts)` | Pretty-print with optional ANSI colors |
| `format_json_compact(value)` | Minified single-line output |
| `query(value, path)` | Dot-path query → `Result<&Value, QueryError>` |
| `parse_and_format(str, opts)` | Parse JSON string → formatted output |

### Types

- **`FormatOptions`** — `indent: usize` (default 2), `color: bool` (default true)
- **`QueryError`** — `KeyNotFound`, `IndexOutOfBounds`, `InvalidQuery`

## License

MIT
