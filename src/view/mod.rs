use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{fs::File, io::Read};

mod args;
mod error;
mod format;

pub use args::ViewArgs;
pub use error::{Result, ViewError};
pub use format::FileFormat;

pub fn view_files(args: ViewArgs) -> Result<()> {
    match args.file_paths.len() {
        0 => view_stdin(&args)?,
        1 => view_single_file(&args)?,
        _ => view_multiple_files(&args)?,
    };

    Ok(())
}

fn view_stdin(args: &ViewArgs) -> Result<()> {
    todo!("stdin");
}

fn view_single_file(args: &ViewArgs) -> Result<()> {
    let file_path = &args.file_paths[0];
    view_file_of_format(file_path, &args)
}

fn output_file_separator() {
    println!("")
}

fn view_multiple_files(args: &ViewArgs) -> Result<()> {
    for (i, file_path) in args.file_paths.iter().enumerate() {
        if i > 0 {
            output_file_separator();
        }

        println!("==> {} <==", file_path.display());
        if let Err(e) = view_file_of_format(file_path, args) {
            println!("view file error: {}", e);
        }
    }

    Ok(())
}

fn view_file_of_format(file_path: &Path, args: &ViewArgs) -> Result<()> {
    match args.format {
        FileFormat::Text => view_text(file_path, args)?,
        FileFormat::Hex => view_hex(file_path, args)?,
    }
    Ok(())
}

fn view_text(file_path: &Path, args: &ViewArgs) -> Result<()> {
    let f = File::open(file_path)?;
    let mut reader = BufReader::new(f);
    let mut buffer = String::new();

    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        println!("{}", line);
        buffer.clear();
    }
    Ok(())
}

fn view_hex(file_path: &Path, args: &ViewArgs) -> Result<()> {
    let mut f = File::open(file_path)?;
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
