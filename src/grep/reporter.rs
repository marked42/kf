use std::{
    borrow::Cow,
    io::{self, Write},
    path::Path,
};

use colored::Colorize;
use regex::Regex;

use super::args::GrepArgs;
use super::matcher::{FileMatches, LineMatch};

pub struct FileMatchesReporter<'a, W: Write> {
    pattern: &'a Regex,
    count: bool,
    color: bool,
    writer: &'a mut W,
}

impl<'a, W: Write> FileMatchesReporter<'a, W> {
    pub fn new(args: &'a GrepArgs, writer: &'a mut W) -> Self {
        Self {
            pattern: &args.pattern,
            count: args.count,
            color: args.color,
            writer,
        }
    }

    pub fn output_file_separator(&mut self) -> io::Result<()> {
        if !self.count {
            self.output_newline()
        } else {
            Ok(())
        }
    }

    pub fn output_stdin_matches(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        if self.count {
            self.output_matches_count(result)
        } else {
            self.output_matched_lines(result)
        }
    }

    pub fn output_file_matches(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        if self.count {
            self.output_file_match_count(result)
        } else {
            self.output_file_matched_lines(result)
        }
    }

    fn output_matches_count(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        write!(self.writer, "{}", result.len())?;
        self.output_newline()
    }

    fn output_file_match_count(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        self.output_file_path(&result.file_path)?;
        write!(self.writer, ":")?;
        self.output_matches_count(result)
    }

    fn output_file_matched_lines(&mut self, result: &FileMatches<'_>) -> io::Result<()> {
        self.output_file_path(&result.file_path)?;
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

    fn output_file_path(&mut self, path: &Path) -> io::Result<()> {
        let path = path.to_string_lossy();
        if self.color {
            write!(self.writer, "{}", path.magenta().bold())
        } else {
            write!(self.writer, "{}", path)
        }
    }

    fn output_line_number(&mut self, line_number: usize) -> io::Result<()> {
        if self.color {
            write!(self.writer, "{}:", line_number.to_string().green())
        } else {
            write!(self.writer, "{}:", line_number)
        }
    }

    pub fn output_line_text(&mut self, line: &str) -> io::Result<()> {
        if self.color {
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
        if self.color && self.pattern.is_match(line) {
            self.pattern.replace_all(line, "$0".red().to_string())
        } else {
            Cow::Borrowed(line)
        }
    }
}
