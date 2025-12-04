pub use clap::{Parser, Subcommand};
use regex::{Regex, RegexBuilder};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, GrepError>;

#[derive(Parser, Debug)]
#[command(
    name = "kf",
    version = "0.1",
    about = "file view / search tool",
    author = "kos"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Searches pattern in target files or directories
    Grep(GrepArgs),
}

#[derive(Debug, Parser)]
pub struct GrepArgs {
    #[arg(index = 1, help = "Pattern to search", value_parser = validate_regex_value)]
    pattern: String,

    #[arg(
        index = 2,
        help = "Target files or directories to search in, search from standard input when not specified"
    )]
    pub files: Vec<String>,

    #[arg(short, long, help = "Recursively search files in directory")]
    pub recursive: bool,

    #[arg(short, long, help = "Count occurrences")]
    pub count: bool,

    #[arg(short, long, help = "Invert match")]
    pub invert_match: bool,

    #[arg(long, help = "Case insensitive pattern match")]
    pub ignore_case: bool,

    #[arg(
        long,
        help = "Display matched pattern in highlight color",
        default_value_t = true
    )]
    pub color: bool,
}

fn validate_regex_value(s: &str) -> Result<String> {
    Regex::new(s).map_err(GrepError::InvalidRegex)?;
    Ok(s.to_string())
}

impl GrepArgs {
    pub fn compiled_pattern(&self) -> Result<Regex> {
        let mut builder = RegexBuilder::new(&self.pattern);
        builder.case_insensitive(self.ignore_case);
        builder.build().map_err(GrepError::InvalidRegex)
    }
}

#[derive(Error, Debug)]
pub enum GrepError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
}
