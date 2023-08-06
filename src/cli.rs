// command line parsing for jolly
use std::process::ExitCode;

fn help() {
    let description = env!("CARGO_PKG_DESCRIPTION");
    let exe = option_env!("CARGO_BIN_NAME").unwrap_or("jolly");
    let name = env!("CARGO_PKG_NAME");
    println!(
        r#"{description}

Usage: {exe} [OPTIONS] [CONFIG FILE]

Options:
-V, --version	Print version info and exit
-h, --help	Print this help and exit

Use the optional parameter [CONFIG FILE] to use a non-default config file

For more details, see the {name} docs: https://github.com/apgoetz/jolly/blob/main/docs/README.md
"#
    );
}

fn version() {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    let date = env!("JOLLY_BUILD_DATE");

    println!("{name} {version} {date}");
}

fn err_help() {
    let name = option_env!("CARGO_BIN_NAME").unwrap_or("jolly");
    eprintln!("Try '{name} --help' for more information");
}

#[derive(Default)]
pub struct ParsedArgs {
    pub config: Option<String>,
}

pub fn parse_args<I: Iterator<Item = String>>(args: I) -> Result<ParsedArgs, ExitCode> {
    let mut parsed_args = ParsedArgs::default();

    for arg in args.skip(1) {
        if arg == "-V" || arg == "-v" || arg == "--version" {
            version();
            return Err(ExitCode::SUCCESS);
        }

        if arg == "-h" || arg == "--help" {
            help();
            return Err(ExitCode::SUCCESS);
        }

        if arg.starts_with("-") {
            eprintln!("Invalid option '{arg}'");
            err_help();
            return Err(ExitCode::FAILURE);
        }

        if parsed_args.config.is_none() {
            parsed_args.config = Some(arg)
        } else {
            eprintln!("Multiple config files passed, only one config file supported at this time");
            err_help();
            return Err(ExitCode::FAILURE);
        }
    }
    Ok(parsed_args)
}
