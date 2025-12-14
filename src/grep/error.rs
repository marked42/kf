use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GrepError {
    #[error("Invalid pattern: {0}")]
    InvalidPattern(#[from] regex::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("No matches found")]
    NoMatches,
}

pub type Result<T> = std::result::Result<T, GrepError>;
