use clap::Parser;
use json_colorizer::{format_json, format_json_compact, query, FormatOptions, Theme};
use serde_json::Value;
use std::fs;
use std::io::{self, Read, IsTerminal};
use std::process;

/// jfmt — A CLI JSON formatter & colorizer
///
/// Pipe JSON in or pass a file path. Get pretty-printed colored output.
#[derive(Parser)]
#[command(name = "jfmt", version, about)]
struct Args {
    /// Output compact (minified) JSON instead of pretty-printed
    #[arg(short, long)]
    compact: bool,

    /// Query a value using dot-path notation (e.g. .data.users[0].name)
    #[arg(short, long)]
    query: Option<String>,

    /// Sort object keys alphabetically
    #[arg(short = 'S', long)]
    sort_keys: bool,

    /// Raw output (omit quotes for strings, print multiple results on new lines)
    #[arg(short = 'r', long)]
    raw: bool,

    /// Indentation size
    #[arg(short = 'i', long, default_value = "2")]
    indent: usize,

    /// Optional file path to read JSON from (reads stdin if omitted)
    file: Option<String>,
}

fn main() {
    let args = Args::parse();

    // ── Read input ──────────────────────────────────────────────
    let input = match &args.file {
        Some(path) => fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("error: failed to read file '{}': {}", path, e);
            process::exit(1);
        }),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                eprintln!("error: failed to read stdin: {}", e);
                process::exit(1);
            });
            buf
        }
    };

    if input.trim().is_empty() {
        return;
    }

    // ── Process Input (NDJSON support) ──────────────────────────
    let stream = serde_json::Deserializer::from_str(&input).into_iter::<Value>();

    for (obj_idx, value_res) in stream.enumerate() {
        let value = value_res.unwrap_or_else(|e| {
            eprintln!("error: invalid JSON (object {}): {}", obj_idx + 1, e);
            process::exit(1);
        });

        // ── Apply query (if any) ────────────────────────────────────
        let results = match &args.query {
            Some(q) => query(&value, q).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            }),
            None => vec![&value],
        };

        // ── Output ──────────────────────────────────────────────────
        let opts = FormatOptions {
            indent: args.indent,
            color: if args.raw { false } else { io::stdout().is_terminal() },
            sort_keys: args.sort_keys,
            theme: Theme::default(),
        };

        for (res_idx, res) in results.iter().enumerate() {
            if args.raw {
                if let Some(s) = res.as_str() {
                    println!("{}", s);
                } else {
                    println!("{}", format_json_compact(res));
                }
            } else if args.compact {
                println!("{}", format_json_compact(res));
            } else {
                // If multiple results and not raw, maybe separate them?
                // jq prints them on new lines.
                if results.len() > 1 && res_idx > 0 {
                    // println!(); // Add newline between results?
                }
                println!("{}", format_json(res, &opts));
            }
        }
    }
}
