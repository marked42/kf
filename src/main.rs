use std::process;

use kf::{CliError, Parser, Result, cli, echo, grep, hex, view};

fn main() {
    match try_main() {
        Ok(_) => process::exit(0),
        Err(CliError::Usage(msg)) => {
            eprintln!("wrong usage: {}", msg);
            process::exit(2);
        }
        Err(CliError::Grep(e)) => match e {
            kf::GrepError::NoMatches => {
                // grep convention exit 1 when no matches
                eprintln!("grep error: {}", e);
                process::exit(1);
            }
            _ => {
                eprintln!("grep error: {}", e);
                process::exit(2);
            }
        },
        Err(CliError::View(e)) => {
            eprintln!("view error: {}", e);
            process::exit(3);
        }
        Err(CliError::Echo(e)) => {
            eprintln!("echo error: {}", e);
            process::exit(3);
        }
        Err(CliError::Hex(e)) => {
            eprintln!("{}", e);
            process::exit(3);
        }
    }
}

fn try_main() -> Result<()> {
    let cli = cli::Cli::try_parse().map_err(|e| CliError::Usage(e.to_string()))?;

    match cli.command {
        cli::Command::Grep(args) => grep::grep(args)?,
        cli::Command::View(args) => view::view_file(args)?,
        cli::Command::Echo(args) => echo::echo(args)?,
        cli::Command::Hex(args) => hex::view_hex(args)?,
    }

    Ok(())
}
