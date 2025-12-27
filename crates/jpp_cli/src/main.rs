use serde_json::Value;
use serde_json_path::JsonPath;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "jpp {VERSION} - JSONPath processor (RFC 9535)

Usage: jpp [OPTIONS] <QUERY> [FILE]

Arguments:
  <QUERY>    JSONPath query (RFC 9535 format)
  [FILE]     Input JSON file (reads from stdin if omitted)

Options:
  -h, --help     Show this help message
  -V, --version  Show version"
    );
}

fn print_version() {
    println!("jpp {VERSION}");
}

enum ParsedArgs {
    Help,
    Version,
    Query { query: String, file: Option<String> },
}

fn parse_args() -> Result<ParsedArgs, String> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        return Err("missing required argument: <QUERY>\n\nUsage: jpp [OPTIONS] <QUERY> [FILE]\n\nFor more information, try '--help'".to_string());
    }

    let mut positional = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(ParsedArgs::Help),
            "-V" | "--version" => return Ok(ParsedArgs::Version),
            s if s.starts_with('-') => {
                return Err(format!("unknown option: {s}\n\nUsage: jpp [OPTIONS] <QUERY> [FILE]\n\nFor more information, try '--help'"));
            }
            _ => positional.push(arg.clone()),
        }
    }

    match positional.len() {
        0 => Err("missing required argument: <QUERY>\n\nUsage: jpp [OPTIONS] <QUERY> [FILE]\n\nFor more information, try '--help'".to_string()),
        1 => Ok(ParsedArgs::Query {
            query: positional.into_iter().next().unwrap_or_default(),
            file: None,
        }),
        2 => {
            let mut iter = positional.into_iter();
            Ok(ParsedArgs::Query {
                query: iter.next().unwrap_or_default(),
                file: iter.next(),
            })
        }
        _ => Err("too many arguments\n\nUsage: jpp [OPTIONS] <QUERY> [FILE]\n\nFor more information, try '--help'".to_string()),
    }
}

fn read_input(file: Option<&str>) -> Result<String, String> {
    match file {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| format!("error reading file '{path}': {e}")),
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| format!("error reading stdin: {e}"))?;
            Ok(buffer)
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;

    match args {
        ParsedArgs::Help => {
            print_help();
            Ok(())
        }
        ParsedArgs::Version => {
            print_version();
            Ok(())
        }
        ParsedArgs::Query { query, file } => {
            let input = read_input(file.as_deref())?;

            let json: Value = serde_json::from_str(&input)
                .map_err(|e| format!("error parsing JSON: {e}"))?;

            let path = JsonPath::parse(&query)
                .map_err(|e| format!("error parsing JSONPath query: {e}"))?;

            let results = path.query(&json);

            let output = serde_json::to_string_pretty(&results.all())
                .map_err(|e| format!("error serializing output: {e}"))?;

            println!("{output}");
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("jpp: {e}");
            ExitCode::FAILURE
        }
    }
}
