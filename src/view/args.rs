use std::path::PathBuf;

use clap::{Parser, builder::RangedU64ValueParser};

use super::format::FileFormat;

const BYTES_PER_LINE: u64 = 16;

#[derive(Debug, Parser)]
pub struct ViewArgs {
    #[arg(
        index = 1,
        help = "Files to view in specified format, standard input use when not files specified"
    )]
    pub file_paths: Vec<PathBuf>,

    #[arg(long, help = "File format", default_value = "text")]
    pub format: FileFormat,

    #[arg(long,
        help = "bytes per line for hex view",
        default_value_t = BYTES_PER_LINE as usize,
        value_parser = RangedU64ValueParser::<usize>::new().range(BYTES_PER_LINE..(usize::MAX as u64))
    )]
    pub bytes_per_line: usize,

    #[arg(
        short,
        long,
        help = "Suppress printing of header when multiple files are provided"
    )]
    pub quite: bool,
}
