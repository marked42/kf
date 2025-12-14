use std::io::{self, BufRead, IsTerminal, Write};

mod args;
mod error;
mod finder;
mod matcher;
mod reporter;

pub use args::GrepArgs;
pub use error::GrepError;
use error::Result;
use finder::FilesFinder;
use matcher::MatchesFinder;
use reporter::FileMatchesReporter;

pub fn grep(args: GrepArgs) -> Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let has_matches = if args.files.is_empty() {
        grep_stdin(&args, &mut writer)?
    } else {
        grep_files(&args, &mut writer)?
    };
    writer.flush()?;

    if has_matches {
        Ok(())
    } else {
        Err(GrepError::NoMatches)
    }
}

fn grep_stdin<W: Write>(args: &GrepArgs, writer: &mut W) -> io::Result<bool> {
    let reader = std::io::stdin().lock();
    if reader.is_terminal() {
        grep_interactive_stdin(reader, args, writer)?;
        Ok(true)
    } else {
        grep_piped_stdin(reader, args, writer)
    }
}

fn grep_piped_stdin<R: BufRead, W: Write>(
    mut reader: R,
    args: &GrepArgs,
    writer: &mut W,
) -> io::Result<bool> {
    let finder = MatchesFinder::from_args(args);
    let result = finder.find_matches_from_stdin(&mut reader)?;
    if !result.is_empty() {
        let mut reporter = FileMatchesReporter::new(args, writer);
        reporter.output_stdin_matches(&result)?;
    }

    Ok(!result.is_empty())
}

fn grep_interactive_stdin<R: BufRead, W: Write>(
    mut reader: R,
    args: &GrepArgs,
    writer: &mut W,
) -> io::Result<()> {
    // reuse single String buffer in every loop iteration
    let mut buffer = String::new();
    let mut reporter = FileMatchesReporter::new(args, writer);

    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        reporter.output_line_text(line)?;
        buffer.clear();
    }

    Ok(())
}

fn grep_files<W: Write>(args: &GrepArgs, writer: &mut W) -> io::Result<bool> {
    let files_finder = FilesFinder::from_args(args);
    let matches_finder = MatchesFinder::from_args(args);
    let mut reporter = FileMatchesReporter::new(args, writer);

    let mut has_matches = false;
    for file_result in files_finder.find_files() {
        match file_result {
            Ok(file_path) => match matches_finder.find_matches_from_file(&file_path) {
                Ok(result) if !result.is_empty() => {
                    if has_matches {
                        reporter.output_file_separator()?;
                    }
                    reporter.output_file_matches(&result)?;
                    has_matches = true;
                }
                Ok(_) => continue,
                Err(e) => {
                    writeln!(
                        io::stderr(),
                        "Error reading file {}: {}",
                        file_path.display(),
                        e
                    )?;
                }
            },
            Err(e) => {
                eprintln!("Error accessing file: {}", e);
            }
        }
    }

    Ok(has_matches)
}
