use std::io::{BufRead, BufReader};
use std::{fs::File, io::Read};

mod args;
mod error;
mod format;

pub use args::ViewArgs;
pub use error::{Result, ViewError};
pub use format::FileFormat;

pub fn view_file(args: ViewArgs) -> Result<()> {
    match args.format {
        FileFormat::Text => view_text(args)?,
        FileFormat::Hex => view_hex(args)?,
    }
    Ok(())
}

fn view_text(args: ViewArgs) -> Result<()> {
    let f = File::open(&args.file_path)?;
    let mut reader = BufReader::new(f);
    let mut buffer = String::new();

    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        println!("{}", line);
        buffer.clear();
    }
    Ok(())
}

fn view_hex(args: ViewArgs) -> Result<()> {
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
