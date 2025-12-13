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

#[derive(Debug, Clone)]
struct FileMatches<'a> {
    file_path: &'a Path,
    matches: Vec<LineMatch>,
}

impl FileMatches<'_> {
    fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

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

fn grep_interactive<R: BufRead, W: Write>(
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

fn grep_stdin<W: Write>(args: &GrepArgs, writer: &mut W) -> io::Result<bool> {
    let reader = std::io::stdin().lock();
    if reader.is_terminal() {
        grep_interactive(reader, args, writer)?;
        return Ok(true);
    }

    // redirect stdout to pipe
    let finder = MatchesFinder::from_args(args);
    let result = finder.find_matches_from_stdin(reader)?;
    if result.is_empty() {
        return Ok(false);
    }

    let mut reporter = FileMatchesReporter::new(args, writer);
    reporter.output_stdin_matches(&result)?;
    Ok(true)
}

fn grep_files<W: Write>(args: &GrepArgs, writer: &mut W) -> io::Result<bool> {
    let files = find_files(args.files.as_slice(), args.recursive);
    let mut has_matches = false;

    let finder = MatchesFinder::from_args(args);
    let mut reporter = FileMatchesReporter::new(args, writer);

    for file_result in files {
        match file_result {
            Ok(file_path) => match finder.find_matches_from_file(&file_path, args) {
                Ok(result) if !result.matches.is_empty() => {
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

struct FileMatchesReporter<'a, W: Write> {
    args: &'a GrepArgs,
    writer: &'a mut W,
}

impl<'a, W: Write> FileMatchesReporter<'a, W> {
    fn new(args: &'a GrepArgs, writer: &'a mut W) -> Self {
        Self { args, writer }
    }

    fn output_file_separator(&mut self) -> io::Result<()> {
        if !self.args.count {
            self.output_newline()
        } else {
            Ok(())
        }
    }

    fn output_stdin_matches(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        if self.args.count {
            self.output_matches_count(result)
        } else {
            self.output_matched_lines(result)
        }
    }

    fn output_file_matches(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        if self.args.count {
            self.output_file_match_count(result)
        } else {
            self.output_file_matched_lines(result)
        }
    }

    fn output_matches_count(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        write!(self.writer, "{}", result.matches.len())?;
        self.output_newline()
    }

    fn output_file_match_count(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        let file_path = result.file_path.to_string_lossy();

        self.output_file_path(&file_path)?;
        write!(self.writer, ":")?;
        self.output_matches_count(result)
    }

    fn output_file_matched_lines(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        let path = result.file_path.to_string_lossy();

        self.output_file_path(&path)?;
        self.output_newline()?;
        self.output_matched_lines(result)?;

        Ok(())
    }

    fn output_matched_lines(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        for LineMatch { line, line_number } in &result.matches {
            self.output_line_number(*line_number)?;
            self.output_line_text(line)?;
        }

        Ok(())
    }

    fn output_file_path(&mut self, path: &str) -> io::Result<()> {
        if self.args.color {
            write!(self.writer, "{}", path.magenta().bold())
        } else {
            write!(self.writer, "{}", path)
        }
    }

    fn output_line_number(&mut self, line_number: usize) -> io::Result<()> {
        if self.args.color {
            write!(self.writer, "{}:", line_number.to_string().green())
        } else {
            write!(self.writer, "{}:", line_number)
        }
    }

    fn output_line_text(&mut self, line: &str) -> io::Result<()> {
        if self.args.color {
            write!(self.writer, "{}", self.highlight_pattern(line.trim()))?;
        } else {
            write!(self.writer, "{}", line.trim())?;
        }
        self.output_newline()
    }

    fn output_newline(&mut self) -> io::Result<()> {
        writeln!(self.writer)
    }

    fn highlight_pattern<'b>(&self, line: &'b str) -> Cow<'b, str> {
        self.args.pattern.replace_all(line, "$0".red().to_string())
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

struct MatchesFinder<'a> {
    pattern: &'a Regex,
    invert_match: bool,
}

impl<'a> MatchesFinder<'a> {
    fn from_args(args: &'a GrepArgs) -> Self {
        MatchesFinder {
            pattern: &args.pattern,
            invert_match: args.invert_match,
        }
    }

    fn find_matches_from_file<'b, P: AsRef<Path>>(
        &self,
        file: &'b P,
    ) -> io::Result<FileMatches<'b>> {
        let reader = BufReader::new(File::open(file)?);
        let matches = self.find_matches_from_reader(reader)?;

        Ok(FileMatches {
            file_path: file.as_ref(),
            matches,
        })
    }

    fn find_matches_from_stdin<R: BufRead>(&self, reader: R) -> io::Result<FileMatches<'_>> {
        Ok(FileMatches {
            file_path: Path::new("stdin"),
            matches: self.find_matches_from_reader(reader)?,
        })
    }

    fn find_matches_from_reader<R: BufRead>(&self, reader: R) -> io::Result<Vec<LineMatch>> {
        let mut matches = vec![];

        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            if self.is_match(&line) {
                matches.push(LineMatch {
                    line,
                    line_number: index + 1,
                });
            }
        }

        Ok(matches)
    }

    fn is_match(&self, line: &str) -> bool {
        self.pattern.is_match(line) ^ self.invert_match
    }
}
