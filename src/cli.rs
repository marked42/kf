pub use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::{
    EchoArgs, EchoError, GrepArgs, GrepError, HexArgs, ViewArgs, ViewError, hex::HexError,
};

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
    /// View specified file in different formats
    View(ViewArgs),
    /// Echo command
    Echo(EchoArgs),
    /// View file in hex format
    Hex(HexArgs),
}

#[derive(Error, Debug)]
pub enum CliError {
    // TODO: need dynamic error type
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Grep(#[from] GrepError),

    #[error(transparent)]
    View(#[from] ViewError),

    #[error(transparent)]
    Echo(#[from] EchoError),

    #[error(transparent)]
    Hex(#[from] HexError),
}
