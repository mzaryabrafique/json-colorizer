//! # json-colorizer
//!
//! A fast, lightweight JSON formatter, pretty-printer, colorizer, and query library for Rust.
//!
//! Use it to pretty-print JSON with syntax highlighting in the terminal,
//! compact-print JSON, or query nested values with dot-path notation.
//!
//! ## Quick Start
//!
//! ```rust
//! use json_colorizer::{format_json, format_json_compact, query, FormatOptions};
//! use serde_json::json;
//!
//! let value = json!({"name": "Alice", "scores": [95, 87, 100]});
//!
//! // Pretty-print with colors
//! let output = format_json(&value, &FormatOptions::default());
//! println!("{}", output);
//!
//! // Compact output
//! let compact = format_json_compact(&value);
//! println!("{}", compact);
//!
//! // Query a nested value
//! let score = query(&value, ".scores[0]").unwrap();
//! assert_eq!(score, &json!(95));
//! ```

use colored::Colorize;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════
//  Public types
// ═══════════════════════════════════════════════════════════════

/// Options for controlling JSON output formatting.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Number of spaces per indentation level (default: 2).
    pub indent: usize,
    /// Whether to colorize the output (default: true).
    pub color: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            color: true,
        }
    }
}

/// Error returned when a query path is invalid or does not match the JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// A key was not found in an object.
    KeyNotFound(String),
    /// An index was out of bounds in an array.
    IndexOutOfBounds(usize),
    /// The query string could not be parsed.
    InvalidQuery(String),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::KeyNotFound(key) => write!(f, "key '{}' not found", key),
            QueryError::IndexOutOfBounds(idx) => write!(f, "index [{}] out of bounds", idx),
            QueryError::InvalidQuery(msg) => write!(f, "invalid query: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}

// ═══════════════════════════════════════════════════════════════
//  Core public API
// ═══════════════════════════════════════════════════════════════

/// Format a JSON value as a pretty-printed string.
///
/// When `opts.color` is `true`, output includes ANSI color codes suitable
/// for terminal display (**cyan** keys, **green** strings, **yellow** numbers,
/// **magenta** booleans, **red** null).
///
/// ```rust
/// use json_colorizer::{format_json, FormatOptions};
/// use serde_json::json;
///
/// let val = json!({"greeting": "hello"});
/// let out = format_json(&val, &FormatOptions { indent: 4, color: false });
/// assert!(out.contains("greeting"));
/// ```
pub fn format_json(value: &Value, opts: &FormatOptions) -> String {
    let mut buf = String::new();
    if opts.color {
        write_colored(&mut buf, value, 0, false, opts.indent);
    } else {
        write_plain(&mut buf, value, 0, false, opts.indent);
    }
    buf
}

/// Format a JSON value as a compact (single-line, no whitespace) string.
///
/// ```rust
/// use json_colorizer::format_json_compact;
/// use serde_json::json;
///
/// let val = json!({"a": 1});
/// assert_eq!(format_json_compact(&val), r#"{"a":1}"#);
/// ```
pub fn format_json_compact(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Query a nested value inside `root` using a dot-path string.
///
/// Supported syntax:
/// - `.key` — object key access
/// - `[n]` — array index access
/// - Chaining: `.data.users[0].name`
///
/// ```rust
/// use json_colorizer::query;
/// use serde_json::json;
///
/// let data = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
/// let name = query(&data, ".users[1].name").unwrap();
/// assert_eq!(name, &json!("Bob"));
/// ```
pub fn query<'a>(root: &'a Value, path: &str) -> Result<&'a Value, QueryError> {
    let segments = parse_query(path)?;
    let mut current = root;

    for seg in &segments {
        match seg {
            Segment::Key(key) => {
                current = current
                    .get(key.as_str())
                    .ok_or_else(|| QueryError::KeyNotFound(key.clone()))?;
            }
            Segment::Index(idx) => {
                current = current
                    .get(*idx)
                    .ok_or(QueryError::IndexOutOfBounds(*idx))?;
            }
        }
    }
    Ok(current)
}

/// Parse a raw JSON string and return the formatted (colorized) output.
///
/// This is a convenience function combining [`serde_json::from_str`] with
/// [`format_json`].
pub fn parse_and_format(json_str: &str, opts: &FormatOptions) -> Result<String, serde_json::Error> {
    let value: Value = serde_json::from_str(json_str)?;
    Ok(format_json(&value, opts))
}

// ═══════════════════════════════════════════════════════════════
//  Query parser internals
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
enum Segment {
    Key(String),
    Index(usize),
}

fn parse_query(query: &str) -> Result<Vec<Segment>, QueryError> {
    let mut segments = Vec::new();
    let q = query.strip_prefix('.').unwrap_or(query);

    if q.is_empty() {
        return Ok(segments); // root query
    }

    let mut chars = q.chars().peekable();
    let mut buf = String::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '.' => {
                if !buf.is_empty() {
                    segments.push(Segment::Key(buf.clone()));
                    buf.clear();
                }
                chars.next();
            }
            '[' => {
                if !buf.is_empty() {
                    segments.push(Segment::Key(buf.clone()));
                    buf.clear();
                }
                chars.next(); // consume '['
                let mut idx_buf = String::new();
                let mut found_bracket = false;
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        found_bracket = true;
                        break;
                    }
                    idx_buf.push(c);
                    chars.next();
                }
                if !found_bracket {
                    return Err(QueryError::InvalidQuery(
                        "unclosed bracket".to_string(),
                    ));
                }
                if let Ok(idx) = idx_buf.parse::<usize>() {
                    segments.push(Segment::Index(idx));
                } else {
                    // bracket notation for keys: ["key"] or ['key']
                    let key = idx_buf.trim_matches('"').trim_matches('\'').to_string();
                    segments.push(Segment::Key(key));
                }
            }
            _ => {
                buf.push(ch);
                chars.next();
            }
        }
    }
    if !buf.is_empty() {
        segments.push(Segment::Key(buf));
    }

    Ok(segments)
}

// ═══════════════════════════════════════════════════════════════
//  Colorized writer
// ═══════════════════════════════════════════════════════════════

fn pad(buf: &mut String, indent_size: usize, level: usize) {
    for _ in 0..(indent_size * level) {
        buf.push(' ');
    }
}

fn write_colored(buf: &mut String, value: &Value, level: usize, trailing_comma: bool, indent_size: usize) {
    let comma = if trailing_comma { "," } else { "" };

    match value {
        Value::Null => {
            buf.push_str(&format!("{}{}", "null".red().dimmed(), comma));
        }
        Value::Bool(b) => {
            buf.push_str(&format!("{}{}", b.to_string().magenta().bold(), comma));
        }
        Value::Number(n) => {
            buf.push_str(&format!("{}{}", n.to_string().yellow(), comma));
        }
        Value::String(s) => {
            buf.push_str(&format!(
                "{}{}",
                format!("\"{}\"", escape_json_string(s)).green(),
                comma
            ));
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                buf.push_str(&format!("[]{}", comma));
                return;
            }
            buf.push_str("[\n");
            for (i, item) in arr.iter().enumerate() {
                pad(buf, indent_size, level + 1);
                let has_comma = i < arr.len() - 1;
                write_colored(buf, item, level + 1, has_comma, indent_size);
                buf.push('\n');
            }
            pad(buf, indent_size, level);
            buf.push_str(&format!("]{}", comma));
        }
        Value::Object(map) => {
            if map.is_empty() {
                buf.push_str(&format!("{{}}{}", comma));
                return;
            }
            buf.push_str("{\n");
            let len = map.len();
            for (i, (key, val)) in map.iter().enumerate() {
                let has_comma = i < len - 1;
                pad(buf, indent_size, level + 1);
                buf.push_str(&format!("{}: ", format!("\"{}\"", key).cyan().bold()));
                write_colored(buf, val, level + 1, has_comma, indent_size);
                buf.push('\n');
            }
            pad(buf, indent_size, level);
            buf.push_str(&format!("}}{}", comma));
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Plain (no-color) writer
// ═══════════════════════════════════════════════════════════════

fn write_plain(buf: &mut String, value: &Value, level: usize, trailing_comma: bool, indent_size: usize) {
    let comma = if trailing_comma { "," } else { "" };

    match value {
        Value::Null => {
            buf.push_str(&format!("null{}", comma));
        }
        Value::Bool(b) => {
            buf.push_str(&format!("{}{}", b, comma));
        }
        Value::Number(n) => {
            buf.push_str(&format!("{}{}", n, comma));
        }
        Value::String(s) => {
            buf.push_str(&format!("\"{}\"{}",  escape_json_string(s), comma));
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                buf.push_str(&format!("[]{}", comma));
                return;
            }
            buf.push_str("[\n");
            for (i, item) in arr.iter().enumerate() {
                pad(buf, indent_size, level + 1);
                let has_comma = i < arr.len() - 1;
                write_plain(buf, item, level + 1, has_comma, indent_size);
                buf.push('\n');
            }
            pad(buf, indent_size, level);
            buf.push_str(&format!("]{}", comma));
        }
        Value::Object(map) => {
            if map.is_empty() {
                buf.push_str(&format!("{{}}{}", comma));
                return;
            }
            buf.push_str("{\n");
            let len = map.len();
            for (i, (key, val)) in map.iter().enumerate() {
                let has_comma = i < len - 1;
                pad(buf, indent_size, level + 1);
                buf.push_str(&format!("\"{}\": ", key));
                write_plain(buf, val, level + 1, has_comma, indent_size);
                buf.push('\n');
            }
            pad(buf, indent_size, level);
            buf.push_str(&format!("}}{}", comma));
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Shared helpers
// ═══════════════════════════════════════════════════════════════

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_compact_output() {
        let val = json!({"a": 1, "b": [2, 3]});
        let out = format_json_compact(&val);
        assert!(out.contains("\"a\":1"));
        assert!(!out.contains('\n'));
    }

    #[test]
    fn test_pretty_plain_output() {
        let val = json!({"name": "Alice"});
        let opts = FormatOptions { indent: 2, color: false };
        let out = format_json(&val, &opts);
        assert!(out.contains("\"name\""));
        assert!(out.contains("\"Alice\""));
        assert!(out.contains('\n'));
    }

    #[test]
    fn test_query_simple_key() {
        let val = json!({"greeting": "hello"});
        let result = query(&val, ".greeting").unwrap();
        assert_eq!(result, &json!("hello"));
    }

    #[test]
    fn test_query_nested() {
        let val = json!({"a": {"b": {"c": 42}}});
        let result = query(&val, ".a.b.c").unwrap();
        assert_eq!(result, &json!(42));
    }

    #[test]
    fn test_query_array_index() {
        let val = json!({"items": [10, 20, 30]});
        let result = query(&val, ".items[1]").unwrap();
        assert_eq!(result, &json!(20));
    }

    #[test]
    fn test_query_mixed() {
        let val = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
        let result = query(&val, ".users[1].name").unwrap();
        assert_eq!(result, &json!("Bob"));
    }

    #[test]
    fn test_query_root() {
        let val = json!({"a": 1});
        let result = query(&val, ".").unwrap();
        assert_eq!(result, &val);
    }

    #[test]
    fn test_query_key_not_found() {
        let val = json!({"a": 1});
        let err = query(&val, ".b").unwrap_err();
        assert!(matches!(err, QueryError::KeyNotFound(_)));
    }

    #[test]
    fn test_query_index_out_of_bounds() {
        let val = json!([1, 2]);
        let err = query(&val, "[5]").unwrap_err();
        assert!(matches!(err, QueryError::IndexOutOfBounds(5)));
    }

    #[test]
    fn test_parse_and_format() {
        let json_str = r#"{"key": "value"}"#;
        let opts = FormatOptions { indent: 2, color: false };
        let result = parse_and_format(json_str, &opts).unwrap();
        assert!(result.contains("\"key\""));
    }

    #[test]
    fn test_parse_and_format_invalid() {
        let opts = FormatOptions::default();
        assert!(parse_and_format("not json", &opts).is_err());
    }

    #[test]
    fn test_empty_object_and_array() {
        let opts = FormatOptions { indent: 2, color: false };
        assert_eq!(format_json(&json!({}), &opts), "{}");
        assert_eq!(format_json(&json!([]), &opts), "[]");
    }

    #[test]
    fn test_escape_special_characters() {
        let val = json!({"msg": "line1\nline2\ttab"});
        let opts = FormatOptions { indent: 2, color: false };
        let out = format_json(&val, &opts);
        assert!(out.contains("\\n"));
        assert!(out.contains("\\t"));
    }
}
