# json-colorizer

A high-performance JSON formatter, pretty-printer, colorizer, and query tool for Rust — with advanced `jq`-style queries and NDJSON support.

[![Crates.io](https://img.shields.io/crates/v/json-colorizer.svg)](https://crates.io/crates/json-colorizer)
[![Docs.rs](https://docs.rs/json-colorizer/badge.svg)](https://docs.rs/json-colorizer)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

- 🎨 **Powerful Colorization** — Vibrant syntax highlighting using a high-performance formatter.
- 🔍 **Advanced Queries** — Support for wildcards (`.*`, `[]`), array slicing (`[0:5]`), and multiple results.
- 🥞 **NDJSON Support** — Process streams of multiple JSON objects (Newline Delimited JSON).
- 📐 **Pretty-print** — Configurable indentation and key sorting.
- 📦 **Compact mode** — Minified single-line output.
- 🔌 **Library & CLI** — Clean API for Rust projects or a standalone CLI tool.
- ⚡ **High Performance** — Leverages `serde_json::ser::Formatter` for efficient streaming output.

## Install (CLI)

```bash
cargo install json-colorizer
```

## CLI Usage

```bash
# Pretty-print with colors
echo '{"name":"Alice","scores":[95,87]}' | json-colorizer

# Sort object keys alphabetically
cat data.json | json-colorizer -S

# Advanced Querying (Wildcards & Slices)
# Get all names from an array of objects
echo '[{"name":"Alice"},{"name":"Bob"}]' | json-colorizer -q '[].name'

# Array slicing
echo '[0,1,2,3,4,5]' | json-colorizer -q '[0:3]'

# Raw output (omit quotes for strings, ideal for piping)
json-colorizer test.ndjson -q ".tags[]" -r

# Process NDJSON (multiple objects)
cat log.ndjson | json-colorizer -q '.message'
```

### CLI Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--compact` | `-c` | Minified single-line output |
| `--query` | `-q` | Dot-path query (e.g. `.users[0].name`) |
| `--raw-output` | `-r` | Raw output (no quotes for strings) |
| `--sort-keys` | `-S` | Sort object keys alphabetically |
| `--indent` | `-i` | Custom indentation size (default: 2) |

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
json-colorizer = "0.1"
```

```rust
use json_colorizer::{format_json, format_json_compact, query, FormatOptions, Theme};
use serde_json::json;

let data = json!({
    "users": [
        {"name": "Alice", "score": 95},
        {"name": "Bob", "score": 87}
    ]
});

// Pretty-print (with default theme)
println!("{}", format_json(&data, &FormatOptions::default()));

// Query (returns a Vec of matches)
let results = query(&data, ".users[].name").unwrap();
assert_eq!(results, vec![&json!("Alice"), &json!("Bob")]);

// Custom Formatting
let opts = FormatOptions {
    indent: 4,
    color: true,
    sort_keys: true,
    theme: Theme::default(),
};
println!("{}", format_json(&data, &opts));
```

## API

| Function | Description |
|----------|-------------|
| `format_json(value, opts)` | Pretty-print with theme-based colorization |
| `format_json_compact(value)` | Minified single-line output |
| `query(value, path)` | Advanced query → `Result<Vec<&Value>, QueryError>` |
| `parse_and_format(str, opts)` | Parse JSON string → formatted output |

### Types

- **`FormatOptions`** — `indent`, `color`, `sort_keys`, `theme`
- **`Theme`** — Customizable colors for `key`, `string`, `number`, `boolean`, `null`
- **`QueryError`** — `KeyNotFound`, `IndexOutOfBounds`, `InvalidQuery`

## License

MIT
