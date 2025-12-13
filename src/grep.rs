use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use clap::builder::PossibleValuesParser;
use clap::{ArgAction, Args, FromArgMatches};
use colored::Colorize;
use regex::{Regex, RegexBuilder};
use thiserror::Error;

#[derive(Debug)]
pub struct GrepArgs {
    pub pattern: Regex,
    pub files: Vec<String>,
    pub recursive: bool,
    pub count: bool,
    pub invert_match: bool,
    pub ignore_case: bool,
    pub color: bool,
}

impl Args for GrepArgs {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd
            .arg(
                clap::Arg::new("pattern")
                    .required(true)
                    .index(1)
                    .value_name("PATTERN")
                    .help("Pattern to search")
            )
            .arg(
                clap::Arg::new("files")
                    .required(false)
                    .index(2)
                    .value_name("FILES")
                    .num_args(0..)
                    .help("Target files or directories to search in, search from standard input when not specified")
            )
            .arg(
                clap::Arg::new("recursive")
                    .short('r')
                    .long("recursive")
                    .action(ArgAction::SetTrue)
                    .help("Recursively search files in directory")
            )
            .arg(
                clap::Arg::new("count")
                    .short('c')
                    .long("count")
                    .action(ArgAction::SetTrue)
                    .help("Count occurrences")
            )
            .arg(
                clap::Arg::new("invert_match")
                    .short('v')
                    .long("invert-match")
                    .action(ArgAction::SetTrue)
                    .help("Invert match")
            )
            .arg(
                clap::Arg::new("ignore_case")
                    .short('i')
                    .long("ignore-case")
                    .action(ArgAction::SetTrue)
                    .help("Case insensitive pattern match")
            )
            .arg(
                clap::Arg::new("color")
                    .long("color")
                    .value_name("WHEN")
                    .num_args(0..=1)
                    .default_missing_value("always")
                    .default_value("auto")
                    .value_parser(PossibleValuesParser::new(["always", "auto", "never"]))
                    .help("Use markers to highlight the matching strings")
            )
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        cmd
    }
}

impl FromArgMatches for GrepArgs {
    fn from_arg_matches(matches: &clap::ArgMatches) -> std::result::Result<Self, clap::Error> {
        let pattern = matches.get_one::<String>("pattern").unwrap();
        let ignore_case = matches.get_flag("ignore_case");

        let mut builder = RegexBuilder::new(&pattern);
        builder.case_insensitive(ignore_case);
        let pattern = builder.build().map_err(|e| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!("Invalid regex pattern '{}': {}", pattern, e),
            )
        })?;

        let files = matches
            .get_many::<String>("files")
            .map(|v| v.cloned().collect())
            .unwrap_or_else(|| Vec::new());

        let recursive = matches.get_flag("recursive");
        let count = matches.get_flag("count");
        let invert_match = matches.get_flag("invert_match");
        let color = matches.get_one::<String>("color").unwrap();
        let color = match color.as_str() {
            "always" => true,
            "never" => false,
            "auto" => std::io::stdout().is_terminal(),
            _ => {
                panic!("Invalid color option, defaulting to 'auto'");
            }
        };

        // 步骤4: 创建完整的 GrepArgs
        Ok(GrepArgs {
            pattern,
            files,
            recursive,
            count,
            invert_match,
            ignore_case,
            color,
        })
    }

    fn update_from_arg_matches(
        &mut self,
        _: &clap::ArgMatches,
    ) -> std::result::Result<(), clap::Error> {
        Ok(())
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

#[derive(Debug, Clone)]
struct LineMatch {
    line: String,
    line_number: usize,
}

pub fn grep(args: GrepArgs) -> Result<()> {
    let pattern = &args.pattern;

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let has_matches = if args.files.is_empty() {
        grep_stdin(&pattern, &args, &mut writer)?
    } else {
        grep_files(&pattern, &args, &mut writer)?
    };
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

    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        let has_match = pattern.is_match(line);
        let colored_output = has_match && args.color;

        if colored_output {
            writeln!(writer, "{}", highlight_pattern(line, pattern))?;
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
        // redirect stdout to pipe
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

    // zsh would print '%' when output not ending with newline
    if has_matches {
        writeln!(writer, "")?;
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
        output_file_match_count(file_path, lines.len(), args, writer)
    } else {
        output_file_matched_lines(file_path, lines, &pattern, args, writer)
    }
}

fn output_file_matched_lines<W: Write, P: AsRef<Path>>(
    path: &P,
    lines: &Vec<LineMatch>,
    pattern: &Regex,
    args: &GrepArgs,
    writer: &mut W,
) -> io::Result<()> {
    let path = path.as_ref().to_string_lossy();
    if args.color {
        writeln!(writer, "{}", path.magenta().bold())?;
    } else {
        writeln!(writer, "{}", path)?;
    }

    for (index, LineMatch { line, line_number }) in lines.iter().enumerate() {
        if index > 0 {
            writeln!(writer, "")?;
        }

        if args.color {
            write!(
                writer,
                "{}:{}",
                line_number.to_string().green(),
                highlight_pattern(line.trim(), pattern)
            )?;
        } else {
            write!(writer, "{}:{}", line_number, line.trim())?;
        }
    }

    Ok(())
}

fn output_file_match_count<W: Write, P: AsRef<Path>>(
    file_path: &P,
    count: usize,
    args: &GrepArgs,
    writer: &mut W,
) -> io::Result<()> {
    let file_path = file_path.as_ref().to_string_lossy();
    if args.color {
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
            matches.push(LineMatch {
                line,
                line_number: index + 1,
            });
        }
    }

    Ok(matches)
}

fn highlight_pattern<'a>(line: &'a str, pattern: &Regex) -> Cow<'a, str> {
    pattern.replace_all(line, "$0".red().to_string())
}
