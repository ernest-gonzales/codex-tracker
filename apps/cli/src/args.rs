use std::env;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub port: Option<u16>,
    pub no_open: bool,
}

pub fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut parsed = CliArgs::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --port".to_string())?;
                let port = value
                    .parse::<u16>()
                    .map_err(|_| format!("invalid port value: {value}"))?;
                parsed.port = Some(port);
            }
            "--no-open" => {
                parsed.no_open = true;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                return Err(format!("unknown argument: {arg}"));
            }
        }
    }

    Ok(parsed)
}

pub fn print_help() {
    println!(
        "Codex Tracker CLI\n\n\
Usage:\n  codex-tracker [--port <port>] [--no-open]\n\n\
Options:\n  --port <port>  Override the configured port for this run only\n  --no-open      Do not open the browser automatically\n  -h, --help     Show this help message\n"
    );
}
