pub use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::{GrepArgs, GrepError};

pub type Result<T> = std::result::Result<T, CliError>;

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

#[derive(Error, Debug)]
pub enum CliError {
    // TODO: need dynamic error type
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Grep(#[from] GrepError),
}
