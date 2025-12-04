pub mod cli;
pub mod grep;

pub use cli::{GrepArgs, Parser};
pub use grep::grep;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
