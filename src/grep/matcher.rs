use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

use regex::Regex;

use super::args::GrepArgs;

#[derive(Debug, Clone)]
pub struct LineMatch {
    pub line: String,
    pub line_number: usize,
}

#[derive(Debug)]
pub struct FileMatches<'a> {
    pub file_path: &'a Path,
    pub matches: Vec<LineMatch>,
}

impl FileMatches<'_> {
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }
}

pub struct MatchesFinder<'a> {
    pattern: &'a Regex,
    invert_match: bool,
}

impl<'a> MatchesFinder<'a> {
    pub fn from_args(args: &'a GrepArgs) -> Self {
        MatchesFinder {
            pattern: &args.pattern,
            invert_match: args.invert_match,
        }
    }

    pub fn find_matches_from_file<'b>(&self, file: &'b Path) -> io::Result<FileMatches<'b>> {
        let reader = BufReader::new(File::open(file)?);
        let matches = self.find_matches_from_reader(reader)?;

        Ok(FileMatches {
            file_path: file,
            matches,
        })
    }

    pub fn find_matches_from_stdin<R: BufRead>(&self, reader: R) -> io::Result<FileMatches<'_>> {
        Ok(FileMatches {
            file_path: Path::new("stdin"),
            matches: self.find_matches_from_reader(reader)?,
        })
    }

    fn find_matches_from_reader<R: BufRead>(&self, reader: R) -> io::Result<Vec<LineMatch>> {
        reader
            .lines()
            .enumerate()
            .filter_map(|(index, line)| match line {
                Ok(line) if self.is_match(&line) => Some(Ok(LineMatch {
                    line,
                    line_number: index + 1,
                })),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect()
    }

    fn is_match(&self, line: &str) -> bool {
        self.pattern.is_match(line) ^ self.invert_match
    }
}
