use std::path::PathBuf;

use clap::{Parser, builder::RangedU64ValueParser};

use super::format::FileFormat;

const BYTES_PER_LINE: u64 = 16;

#[derive(Debug, Parser)]
pub struct ViewArgs {
    // TODO: view from stdin when no file specified
    #[arg(index = 1, help = "File to view in specified format")]
    pub file_paths: Vec<PathBuf>,

    #[arg(long, help = "Output format", default_value = "text")]
    pub format: FileFormat,

    #[arg(long,
        help = "bytes per line for hex view",
        default_value_t = BYTES_PER_LINE as usize,
        value_parser = RangedU64ValueParser::<usize>::new().range(BYTES_PER_LINE..(usize::MAX as u64))
    )]
    pub bytes_per_line: usize,
}
