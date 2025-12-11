use std::fs::File;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::Parser;
use colored::Colorize;
use regex::{Regex, RegexBuilder};
use thiserror::Error;

#[derive(Debug, Parser)]
pub struct GrepArgs {
    #[arg(index = 1, help = "Pattern to search", value_parser = validate_regex_value)]
    pattern: String,

    #[arg(
        index = 2,
        help = "Target files or directories to search in, search from standard input when not specified"
    )]
    pub files: Vec<String>,

    #[arg(short, long, help = "Recursively search files in directory")]
    pub recursive: bool,

    #[arg(short, long, help = "Count occurrences")]
    pub count: bool,

    #[arg(short, long, help = "Invert match")]
    pub invert_match: bool,

    #[arg(long, help = "Case insensitive pattern match")]
    pub ignore_case: bool,

    #[arg(
        long,
        help = "Display matched pattern in highlight color",
        default_value_t = true
    )]
    pub color: bool,
}

fn validate_regex_value(s: &str) -> std::result::Result<String, String> {
    Regex::new(s)
        .map(|re| re.to_string())
        .map_err(|e| format!("Invalid regex pattern: {}", e))
}

impl GrepArgs {
    // TODO: pattern should be resolved once in entrance
    pub fn compiled_pattern(&self) -> std::result::Result<Regex, regex::Error> {
        let mut builder = RegexBuilder::new(&self.pattern);
        builder.case_insensitive(self.ignore_case);
        builder.build()
    }
}

#[derive(Error, Debug)]
pub enum GrepError {
    #[error("Invalid pattern: {0}")]
    InvalidPattern(#[from] regex::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("No matches found")]
    NoMatches,
}

type Result<T> = std::result::Result<T, GrepError>;
type LineMatch = (String, usize);

pub fn grep(args: GrepArgs) -> Result<()> {
    let pattern = args.compiled_pattern().map_err(GrepError::InvalidPattern)?;

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let has_matches = if args.files.is_empty() {
        grep_stdin(&pattern, &args, &mut writer)?
    } else {
        grep_files(&pattern, &args, &mut writer)?
    };
    // avoid shell output '%' before next command
    // writeln!(writer, "")?;
    writer.flush()?;

    if has_matches {
        Ok(())
    } else {
        Err(GrepError::NoMatches)
    }
}

fn grep_interactive<R: BufRead, W: Write>(
    mut reader: R,
    pattern: &Regex,
    args: &GrepArgs,
    writer: &mut W,
) -> io::Result<()> {
    // reuse single String buffer in every loop iteration
    let mut buffer = String::new();

    loop {
        let bytes_read = reader.read_line(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let line = buffer.trim_end();
        let has_match = pattern.is_match(line);
        let colored_output = has_match && args.color;

        if colored_output {
            writeln!(
                writer,
                "{}",
                pattern.replace_all(&line, "$0".red().to_string())
            )?;
        } else {
            writeln!(writer, "{}", line)?;
        }

        // read_line appends to buffer, clear buffer after iteration
        buffer.clear();
    }

    Ok(())
}

fn grep_stdin<W: Write>(pattern: &Regex, args: &GrepArgs, writer: &mut W) -> io::Result<bool> {
    let reader = std::io::stdin().lock();
    if reader.is_terminal() {
        grep_interactive(reader, &pattern, args, writer)?;
        Ok(true)
    } else {
        let matches = find_matches_in_reader(reader, pattern, args.invert_match)?;
        let has_matches = !matches.is_empty();
        if has_matches {
            output_file_matches(&"stdin", &matches, pattern, args, writer)?;
        }
        Ok(has_matches)
    }
}

fn grep_files<W: Write>(pattern: &Regex, args: &GrepArgs, writer: &mut W) -> io::Result<bool> {
    let files = find_files(args.files.as_slice(), args.recursive);
    let mut has_matches = false;

    for (i, file_result) in files.into_iter().enumerate() {
        match file_result {
            Ok(file_path) => match find_matches_in_file(&file_path, &pattern, args.invert_match) {
                Ok(lines) if !lines.is_empty() => {
                    if has_matches {
                        output_file_match_separator(i, args.count, writer)?;
                    }
                    output_file_matches(&file_path, &lines, pattern, args, writer)?;
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

fn output_file_match_separator<W: Write>(i: usize, count: bool, writer: &mut W) -> io::Result<()> {
    if i > 0 {
        write!(writer, "{}", if count { "\n" } else { "\n\n" })?;
    }
    Ok(())
}

fn output_file_matches<W: Write, P: AsRef<Path>>(
    file_path: &P,
    lines: &Vec<LineMatch>,
    pattern: &Regex,
    args: &GrepArgs,
    writer: &mut W,
) -> io::Result<()> {
    if args.count {
        output_file_match_count(file_path, lines.len(), args.color, writer)
    } else {
        output_file_matched_lines(file_path, lines, &pattern, args.color, writer)
    }
}

fn output_file_matched_lines<W: Write, P: AsRef<Path>>(
    path: &P,
    lines: &Vec<LineMatch>,
    pattern: &Regex,
    color: bool,
    writer: &mut W,
) -> io::Result<()> {
    let path = path.as_ref().to_string_lossy();
    if color {
        writeln!(writer, "{}", path.magenta().bold())?;
    } else {
        writeln!(writer, "{}", path)?;
    }

    for (index, (line, num)) in lines.iter().enumerate() {
        if index > 0 {
            writeln!(writer, "")?;
        }

        if color {
            write!(
                writer,
                "{}:{}",
                num.to_string().green(),
                pattern.replace_all(line.trim(), "$0".red().to_string())
            )?;
        } else {
            write!(writer, "{}:{}", num, line.trim())?;
        }
    }

    Ok(())
}

fn output_file_match_count<W: Write, P: AsRef<Path>>(
    file_path: &P,
    count: usize,
    color: bool,
    writer: &mut W,
) -> io::Result<()> {
    let file_path = file_path.as_ref().to_string_lossy();
    if color {
        write!(writer, "{}:{}", file_path.magenta().bold(), count)
    } else {
        write!(writer, "{}:{}", file_path, count)
    }
}

fn find_files<P: AsRef<Path>>(paths: &[P], recursive: bool) -> Vec<std::io::Result<PathBuf>> {
    let mut files = vec![];

    for path in paths {
        let path = path.as_ref();
        let metadata = fs::metadata(path);

        match metadata {
            Ok(f) => {
                if f.is_file() {
                    files.push(Ok(path.to_path_buf()));
                } else if f.is_dir() {
                    if recursive {
                        match find_files_in_dir(&path, recursive) {
                            Err(e) => files.push(Err(e)),
                            Ok(sub_files) => {
                                files.extend(sub_files.into_iter().map(Ok));
                            }
                        }
                    } else {
                        files.push(Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!(
                                "{} is a directory, use -r to search recursively",
                                path.display()
                            ),
                        )));
                    }
                }
            }
            Err(e) => {
                files.push(Err(e));
            }
        }
    }

    files
}

fn find_files_in_dir<P: AsRef<Path>>(dir_path: &P, recursive: bool) -> io::Result<Vec<PathBuf>> {
    let mut files = vec![];

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() && recursive {
            let mut nested_files = find_files_in_dir(&path, recursive)?;
            files.append(&mut nested_files);
        }
    }

    Ok(files)
}

fn find_matches_in_file<P: AsRef<Path>>(
    file: &P,
    pattern: &Regex,
    invert_match: bool,
) -> io::Result<Vec<LineMatch>> {
    let reader = BufReader::new(File::open(file)?);
    find_matches_in_reader(reader, pattern, invert_match)
}

fn find_matches_in_reader<R: BufRead>(
    reader: R,
    pattern: &Regex,
    invert_match: bool,
) -> io::Result<Vec<LineMatch>> {
    let mut matches = vec![];

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let matched = pattern.is_match(&line);
        if matched ^ invert_match {
            matches.push((line, index + 1));
        }
    }

    Ok(matches)
}
