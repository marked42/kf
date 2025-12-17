use std::fs::File;
use std::io::{BufRead, BufReader, IsTerminal};
use std::path::Path;

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
    let mut reader = std::io::stdin().lock();
    if reader.is_terminal() {
        view_interactive_stdin(&mut reader, args)
    } else {
        view_piped_stdin(&mut reader, args)
    }
}

fn view_interactive_stdin(reader: &mut impl BufRead, args: &ViewArgs) -> Result<()> {
    // reuse single String buffer in every loop iteration
    let mut buffer = String::new();

    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        println!("{}", line);
        buffer.clear();
    }

    Ok(())
}

fn view_piped_stdin(reader: &mut impl BufRead, args: &ViewArgs) -> Result<()> {
    view_reader_of_format(reader, args)
}

fn view_single_file(args: &ViewArgs) -> Result<()> {
    let file_path = &args.file_paths[0];
    view_single_file_by_path(file_path, args)
}

fn view_single_file_by_path(file_path: &Path, args: &ViewArgs) -> Result<()> {
    let f = File::open(file_path)?;
    let mut reader = BufReader::new(f);

    view_reader_of_format(&mut reader, args)
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
        if let Err(e) = view_single_file_by_path(file_path, args) {
            eprintln!("view file error: {}", e);
        }
    }

    Ok(())
}

fn view_reader_of_format(reader: &mut impl BufRead, args: &ViewArgs) -> Result<()> {
    match args.format {
        FileFormat::Text => view_reader_text(reader, args)?,
        FileFormat::Hex => view_reader_hex(reader, args)?,
    }
    Ok(())
}

fn view_reader_text(reader: &mut impl BufRead, args: &ViewArgs) -> Result<()> {
    let mut buffer = String::new();

    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        println!("{}", line);
        buffer.clear();
    }
    Ok(())
}

fn view_reader_hex(reader: &mut impl BufRead, args: &ViewArgs) -> Result<()> {
    let mut pos = 0;
    let mut buffer = vec![0; args.bytes_per_line];

    while let Ok(_) = reader.read_exact(&mut buffer) {
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
