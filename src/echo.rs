use clap::Parser;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EchoError {
    #[error("Echo command error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Parser)]
pub struct EchoArgs {
    #[arg(index = 1, num_args=0.., help = "Words to echo")]
    words: Vec<String>,

    #[arg(short = 'n', help = "Do not print the trailing newline character")]
    omit_newline: bool,
}

pub type Result<T> = std::result::Result<T, EchoError>;

pub fn echo(args: EchoArgs) -> Result<()> {
    println!("{:?}", args);

    let ending = if args.omit_newline { "" } else { "\n" };
    print!("{}{}", args.words.join(" "), ending);

    Ok(())
}
