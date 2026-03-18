use clap::Parser;
use json_colorizer::{format_json, format_json_compact, query, FormatOptions};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
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
        eprintln!("error: no input provided");
        process::exit(1);
    }

    // ── Parse JSON ──────────────────────────────────────────────
    let value: Value = serde_json::from_str(&input).unwrap_or_else(|e| {
        eprintln!("error: invalid JSON: {}", e);
        process::exit(1);
    });

    // ── Apply query (if any) ────────────────────────────────────
    let target = match &args.query {
        Some(q) => query(&value, q).unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            process::exit(1);
        }),
        None => &value,
    };

    // ── Output ──────────────────────────────────────────────────
    if args.compact {
        println!("{}", format_json_compact(target));
    } else {
        let opts = FormatOptions::default();
        println!("{}", format_json(target, &opts));
    }
}
