pub mod cli;
pub mod grep;
pub mod view;

pub use cli::{CliError, Parser, Result};
pub use grep::{GrepArgs, GrepError, grep};
pub use view::{ViewArgs, ViewError, view_file};
