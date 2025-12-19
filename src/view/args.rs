use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ViewArgs {
    // TODO: view from stdin when no file specified
    #[arg(index = 1, help = "File to view in specified format")]
    pub file_path: PathBuf,
}
