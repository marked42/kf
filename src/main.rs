use std::process;

use kf::{CliError, Parser, Result, cli, grep};

fn main() {
    match try_main() {
        Ok(_) => process::exit(0),
        Err(CliError::Usage(msg)) => {
            eprintln!("wrong usage: {}", msg);
            process::exit(2);
        }
        Err(CliError::Grep(e)) => {
            eprintln!("grep error: {}", e);
            process::exit(1);
        }
    }
}

fn try_main() -> Result<()> {
    let cli = cli::Cli::try_parse().map_err(|e| CliError::Usage(e.to_string()))?;

    match cli.command {
        cli::Command::Grep(args) => grep::grep(args)?,
    }

    Ok(())
}
