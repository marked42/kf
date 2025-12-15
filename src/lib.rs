pub mod cli;
pub mod echo;
pub mod grep;
pub mod view;

pub use cli::{CliError, Parser, Result};
pub use echo::{EchoArgs, EchoError, echo};
pub use grep::{GrepArgs, GrepError, grep};
pub use view::{ViewArgs, ViewError, view_file};
