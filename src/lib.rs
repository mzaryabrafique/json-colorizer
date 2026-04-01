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
//! assert_eq!(score, vec![&json!(95)]);
//! ```

use colored::{Color, Colorize};
use serde::Serialize;
use serde_json::ser::{Formatter, PrettyFormatter};
use serde_json::Value;
use std::io::{self, Write};

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
    /// Whether to sort object keys (default: false).
    pub sort_keys: bool,
    /// Color theme for the output.
    pub theme: Theme,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            color: true,
            sort_keys: false,
            theme: Theme::default(),
        }
    }
}

/// Colors for different JSON components.
#[derive(Debug, Clone)]
pub struct Theme {
    pub key: Color,
    pub string: Color,
    pub number: Color,
    pub boolean: Color,
    pub null: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            key: Color::Cyan,
            string: Color::Green,
            number: Color::Yellow,
            boolean: Color::Magenta,
            null: Color::BrightBlack, // Dimmed null
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

pub fn format_json(value: &Value, opts: &FormatOptions) -> String {
    let mut writer = Vec::new();
    let indent_buf = vec![b' '; opts.indent];

    if opts.color {
        let formatter = ColorFormatter::new(&indent_buf, &opts.theme);
        let mut ser = serde_json::Serializer::with_formatter(&mut writer, formatter);
        value.serialize(&mut ser).unwrap();
    } else {
        let formatter = PrettyFormatter::with_indent(&indent_buf);
        let mut ser = serde_json::Serializer::with_formatter(&mut writer, formatter);
        value.serialize(&mut ser).unwrap();
    }

    String::from_utf8(writer).unwrap_or_default()
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
/// Query a nested value (or values) inside `root` using a dot-path string.
///
/// Supported syntax:
/// - `.key` or `."quoted key"` — object key access
/// - `[n]` — array index access (0-based)
/// - `.*` — all values in an object
/// - `[]` — all elements in an array
/// - `[start:end]` — array slice (exclusive end)
///
/// Returns a vector of references to matching values.
///
/// ```rust
/// use json_colorizer::query;
/// use serde_json::json;
///
/// let data = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
/// let results = query(&data, ".users[].name").unwrap();
/// assert_eq!(results.len(), 2);
/// assert_eq!(results[0], &json!("Alice"));
/// assert_eq!(results[1], &json!("Bob"));
/// ```
pub fn query<'a>(root: &'a Value, path: &str) -> Result<Vec<&'a Value>, QueryError> {
    let segments = parse_query(path)?;
    let mut current_matches = vec![root];

    for seg in &segments {
        let mut next_matches = Vec::new();
        for val in current_matches {
            match seg {
                Segment::Key(key) => {
                    if let Some(v) = val.get(key) {
                        next_matches.push(v);
                    }
                }
                Segment::Index(idx) => {
                    if let Some(v) = val.get(*idx) {
                        next_matches.push(v);
                    }
                }
                Segment::Wildcard => {
                    if let Some(obj) = val.as_object() {
                        for v in obj.values() {
                            next_matches.push(v);
                        }
                    } else if let Some(arr) = val.as_array() {
                        for v in arr {
                            next_matches.push(v);
                        }
                    }
                }
                Segment::Slice(start, end) => {
                    if let Some(arr) = val.as_array() {
                        let start = start.unwrap_or(0);
                        let end = end.unwrap_or(arr.len()).min(arr.len());
                        if start < end {
                            for v in &arr[start..end] {
                                next_matches.push(v);
                            }
                        }
                    }
                }
            }
        }
        if next_matches.is_empty() {
            // If any segment fails to match anything, it's an error for that path branch.
            // But we only return error if NO matches were found at all?
            // jq behavior: if you query .a.b and .a is null, it's an error or null.
            // Let's be strict: if a key is not found, return KeyNotFound.
            // But for wildcards, empty result is fine?
            // Let's check segments:
            match seg {
                Segment::Key(key) if !path.is_empty() => return Err(QueryError::KeyNotFound(key.clone())),
                Segment::Index(idx) => return Err(QueryError::IndexOutOfBounds(*idx)),
                _ => {} // Empty wildcard or slice is fine
            }
        }
        current_matches = next_matches;
    }

    Ok(current_matches)
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Key(String),
    Index(usize),
    Wildcard,                   // .* or []
    Slice(Option<usize>, Option<usize>), // [start:end]
}

fn parse_query(query: &str) -> Result<Vec<Segment>, QueryError> {
    let mut segments = Vec::new();
    let q = query.strip_prefix('.').unwrap_or(query);

    if q.is_empty() {
        return Ok(segments);
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
                if let Some(&'*') = chars.peek() {
                    segments.push(Segment::Wildcard);
                    chars.next();
                }
            }
            '*' => {
                segments.push(Segment::Wildcard);
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
                    return Err(QueryError::InvalidQuery("unclosed bracket".to_string()));
                }

                if idx_buf.is_empty() {
                    segments.push(Segment::Wildcard);
                } else if idx_buf.contains(':') {
                    // Slice [start:end]
                    let parts: Vec<&str> = idx_buf.split(':').collect();
                    let start = if parts[0].is_empty() {
                        None
                    } else {
                        Some(parts[0].parse().map_err(|_| {
                            QueryError::InvalidQuery(format!("invalid slice start: {}", parts[0]))
                        })?)
                    };
                    let end = if parts.len() < 2 || parts[1].is_empty() {
                        None
                    } else {
                        Some(parts[1].parse().map_err(|_| {
                            QueryError::InvalidQuery(format!("invalid slice end: {}", parts[1]))
                        })?)
                    };
                    segments.push(Segment::Slice(start, end));
                } else if let Ok(idx) = idx_buf.parse::<usize>() {
                    segments.push(Segment::Index(idx));
                } else {
                    // Quoted key or raw key in brackets
                    let key = idx_buf.trim_matches('"').trim_matches('\'').to_string();
                    segments.push(Segment::Key(key));
                }
            }
            '"' => {
                // Quoted key in dot notation: ."quoted key"
                chars.next(); // consume '"'
                let mut key_buf = String::new();
                let mut found_quote = false;
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next();
                        found_quote = true;
                        break;
                    }
                    key_buf.push(c);
                    chars.next();
                }
                if !found_quote {
                    return Err(QueryError::InvalidQuery("unclosed quote".to_string()));
                }
                segments.push(Segment::Key(key_buf));
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
//  ColorFormatter implementation
// ═══════════════════════════════════════════════════════════════

struct ColorFormatter<'a> {
    pretty: PrettyFormatter<'a>,
    is_key: bool,
    theme: &'a Theme,
}

impl<'a> ColorFormatter<'a> {
    fn new(indent: &'a [u8], theme: &'a Theme) -> Self {
        Self {
            pretty: PrettyFormatter::with_indent(indent),
            is_key: false,
            theme,
        }
    }
}

impl<'a> Formatter for ColorFormatter<'a> {
    #[inline]
    fn begin_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.begin_array(writer)
    }

    #[inline]
    fn end_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.end_array(writer)
    }

    #[inline]
    fn begin_array_value<W: ?Sized + Write>(&mut self, writer: &mut W, first: bool) -> io::Result<()> {
        self.pretty.begin_array_value(writer, first)
    }

    #[inline]
    fn end_array_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.end_array_value(writer)
    }

    #[inline]
    fn begin_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.begin_object(writer)
    }

    #[inline]
    fn end_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.end_object(writer)
    }

    #[inline]
    fn begin_object_key<W: ?Sized + Write>(&mut self, writer: &mut W, first: bool) -> io::Result<()> {
        self.is_key = true;
        self.pretty.begin_object_key(writer, first)
    }

    #[inline]
    fn end_object_key<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.is_key = false;
        self.pretty.end_object_key(writer)
    }

    #[inline]
    fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.begin_object_value(writer)
    }

    #[inline]
    fn end_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.pretty.end_object_value(writer)
    }

    #[inline]
    fn write_null<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all("null".color(self.theme.null).to_string().as_bytes())
    }

    #[inline]
    fn write_bool<W: ?Sized + Write>(&mut self, writer: &mut W, value: bool) -> io::Result<()> {
        let s = if value { "true" } else { "false" };
        writer.write_all(s.color(self.theme.boolean).bold().to_string().as_bytes())
    }

    #[inline]
    fn write_i64<W: ?Sized + Write>(&mut self, writer: &mut W, value: i64) -> io::Result<()> {
        writer.write_all(value.to_string().color(self.theme.number).to_string().as_bytes())
    }

    #[inline]
    fn write_u64<W: ?Sized + Write>(&mut self, writer: &mut W, value: u64) -> io::Result<()> {
        writer.write_all(value.to_string().color(self.theme.number).to_string().as_bytes())
    }

    #[inline]
    fn write_f64<W: ?Sized + Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        writer.write_all(value.to_string().color(self.theme.number).to_string().as_bytes())
    }

    #[inline]
    fn write_string_fragment<W: ?Sized + Write>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()> {
        if self.is_key {
            writer.write_all(fragment.color(self.theme.key).bold().to_string().as_bytes())
        } else {
            writer.write_all(fragment.color(self.theme.string).to_string().as_bytes())
        }
    }

    #[inline]
    fn begin_string<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if self.is_key {
            writer.write_all("\"".color(self.theme.key).bold().to_string().as_bytes())
        } else {
            writer.write_all("\"".color(self.theme.string).to_string().as_bytes())
        }
    }

    #[inline]
    fn end_string<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if self.is_key {
            writer.write_all("\"".color(self.theme.key).bold().to_string().as_bytes())
        } else {
            writer.write_all("\"".color(self.theme.string).to_string().as_bytes())
        }
    }

    #[inline]
    fn write_raw_fragment<W: ?Sized + Write>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()> {
        writer.write_all(fragment.as_bytes())
    }
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
        let opts = FormatOptions { indent: 2, color: false, ..FormatOptions::default() };
        let out = format_json(&val, &opts);
        assert!(out.contains("\"name\""));
        assert!(out.contains("\"Alice\""));
        assert!(out.contains('\n'));
    }

    #[test]
    fn test_query_simple_key() {
        let val = json!({"greeting": "hello"});
        let result = query(&val, ".greeting").unwrap();
        assert_eq!(result, vec![&json!("hello")]);
    }

    #[test]
    fn test_query_nested() {
        let val = json!({"a": {"b": {"c": 42}}});
        let result = query(&val, ".a.b.c").unwrap();
        assert_eq!(result, vec![&json!(42)]);
    }

    #[test]
    fn test_query_array_index() {
        let val = json!({"items": [10, 20, 30]});
        let result = query(&val, ".items[1]").unwrap();
        assert_eq!(result, vec![&json!(20)]);
    }

    #[test]
    fn test_query_mixed() {
        let val = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
        let result = query(&val, ".users[1].name").unwrap();
        assert_eq!(result, vec![&json!("Bob")]);
    }

    #[test]
    fn test_query_root() {
        let val = json!({"a": 1});
        let result = query(&val, ".").unwrap();
        assert_eq!(result, vec![&val]);
    }

    #[test]
    fn test_query_wildcard_array() {
        let val = json!([1, 2, 3]);
        let result = query(&val, "[]").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], &json!(1));
    }

    #[test]
    fn test_query_wildcard_object() {
        let val = json!({"a": 1, "b": 2});
        let result = query(&val, ".*").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_query_slice() {
        let val = json!([0, 1, 2, 3, 4]);
        let result = query(&val, "[1:4]").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], &json!(1));
        assert_eq!(result[2], &json!(3));
    }

    #[test]
    fn test_query_quoted_key() {
        let val = json!({"key with spaces": "val"});
        let result = query(&val, ".\"key with spaces\"").unwrap();
        assert_eq!(result, vec![&json!("val")]);

        let result2 = query(&val, "[\"key with spaces\"]").unwrap();
        assert_eq!(result2, vec![&json!("val")]);
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
        let opts = FormatOptions { indent: 2, color: false, ..FormatOptions::default() };
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
        let opts = FormatOptions { indent: 2, color: false, ..FormatOptions::default() };
        assert_eq!(format_json(&json!({}), &opts), "{}");
        assert_eq!(format_json(&json!([]), &opts), "[]");
    }

    #[test]
    fn test_escape_special_characters() {
        let val = json!({"msg": "line1\nline2\ttab"});
        let opts = FormatOptions { indent: 2, color: false, ..FormatOptions::default() };
        let out = format_json(&val, &opts);
        assert!(out.contains("\\n"));
        assert!(out.contains("\\t"));
    }
}
