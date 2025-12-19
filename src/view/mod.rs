use std::fs::File;
use std::io::{BufRead, BufReader};

mod args;
mod error;

pub use args::ViewArgs;
pub use error::{Result, ViewError};

pub fn view_file(args: ViewArgs) -> Result<()> {
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
