use std::env;
use std::fs::File;
use std::io::{self, Cursor, Read};

use ingest::{latest_context_from_reader, usage_totals_from_reader};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: ingest_cli <path|->");
        std::process::exit(2);
    }

    let path = &args[1];
    let mut data = String::new();
    if path == "-" {
        let mut stdin = io::stdin();
        stdin.read_to_string(&mut data).unwrap_or_else(|err| {
            eprintln!("failed to read stdin: {}", err);
            std::process::exit(1);
        });
    } else {
        let mut file = File::open(path).unwrap_or_else(|err| {
            eprintln!("failed to open {}: {}", path, err);
            std::process::exit(1);
        });
        file.read_to_string(&mut data).unwrap_or_else(|err| {
            eprintln!("failed to read {}: {}", path, err);
            std::process::exit(1);
        });
    }

    let totals = usage_totals_from_reader(Cursor::new(&data));
    let context = latest_context_from_reader(Cursor::new(&data));

    match totals {
        Some(value) => {
            println!("total_tokens {}", value.total_tokens);
            println!("input_tokens {}", value.input_tokens);
            println!("output_tokens {}", value.output_tokens);
            println!("cached_input_tokens {}", value.cached_input_tokens);
            println!("reasoning_output_tokens {}", value.reasoning_output_tokens);
        }
        None => {
            eprintln!("no token_count totals found");
            std::process::exit(3);
        }
    }

    if let Some(context) = context {
        let percent_left = context.percent_left().unwrap_or(0.0);
        println!("context_used {}", context.context_used);
        println!("context_window {}", context.context_window);
        println!("context_left_percent {:.2}", percent_left);
    }
}
