pub mod cli;
pub mod grep;

pub use cli::{CliError, Parser, Result};
pub use grep::{GrepArgs, GrepError, grep};
