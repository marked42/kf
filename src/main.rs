use kf::{Parser, cli, grep};

fn main() {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Grep(args) => grep::grep(args),
    }
}
