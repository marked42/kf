pub mod cli;
pub mod grep;

pub use cli::{CliError, GrepArgs, Parser, Result};
pub use grep::grep;
