pub mod cli;
pub mod echo;
pub mod grep;
pub mod hex;
pub mod view;

pub use cli::{CliError, Parser, Result};
pub use echo::{EchoArgs, EchoError, echo};
pub use grep::{GrepArgs, GrepError, grep};
pub use hex::{HexArgs, view_hex};
pub use view::{ViewArgs, ViewError, view_files};
