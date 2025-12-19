use std::{fs::File, io::Read, path::PathBuf};

use clap::{Parser, builder::RangedU64ValueParser};
use thiserror::Error;

const BYTES_PER_LINE: u64 = 16;

#[derive(Debug, Parser)]
pub struct HexArgs {
    #[arg(index = 1, help = "File to view in specified format")]
    pub file_path: PathBuf,

    #[arg(long,
        help = "bytes per line for hex view",
        default_value_t = BYTES_PER_LINE as usize,
        value_parser = RangedU64ValueParser::<usize>::new().range(BYTES_PER_LINE..(usize::MAX as u64))
    )]
    pub bytes_per_line: usize,
}

#[derive(Error, Debug)]
pub enum HexError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HexError>;

pub fn view_hex(args: HexArgs) -> Result<()> {
    let mut f = File::open(args.file_path)?;
    let mut pos = 0;
    let mut buffer = vec![0; args.bytes_per_line];

    while let Ok(_) = f.read_exact(&mut buffer) {
        print!("[0x{:08x}] ", pos);
        pos += args.bytes_per_line;
        for byte in &buffer {
            match *byte {
                0x00 => print!(". "),
                0xff => print!("## "),
                _ => print!("{:02x} ", byte),
            }
        }
        println!();
    }

    Ok(())
}
