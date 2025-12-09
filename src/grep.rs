use std::fs::File;
use std::io::{BufRead, BufReader, IsTerminal};
use std::path::Path;
use std::{fs, io};

use colored::Colorize;
use regex::Regex;

use crate::cli::{GrepArgs, GrepError};

type Result<T> = std::result::Result<T, GrepError>;
type LineMatch = (String, usize);

pub fn grep(args: GrepArgs) -> Result<()> {
    let pattern = args.compiled_pattern().map_err(GrepError::InvalidRegex)?;

    if args.files.is_empty() {
        let reader = std::io::stdin().lock();
        if reader.is_terminal() {
            output_matches_from_terminal(Box::new(reader), &pattern, args.color);
        } else {
            output_matches_from_stdin(Box::new(reader), "stdin", &pattern, &args);
        }
    } else {
        output_matches_from_files(&args, &pattern);
    }
    // avoid shell output '%' before next command
    println!("");

    Ok(())
}

fn output_matches_from_terminal(reader: Box<dyn BufRead>, pattern: &Regex, color: bool) {
    for line in reader.lines() {
        match line {
            Ok(line) => {
                let m = pattern.find(&line);
                if m.is_some() {
                    if color {
                        println!("{}", pattern.replace_all(&line, "$0".red().to_string()));
                    } else {
                        println!("{}", line);
                    }
                } else {
                    println!("{}", line);
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }
}

fn output_matches_from_stdin(
    reader: Box<dyn BufRead>,
    file: &str,
    pattern: &Regex,
    args: &GrepArgs,
) {
    match find_matched_lines_from_reader(reader, pattern, args.invert_match) {
        Ok(lines) => {
            if lines.is_empty() {
                return;
            }

            output_file_match(file, &lines, pattern, args);
        }
        Err(e) => {
            eprintln!("Error processing stdin: {}", e);
        }
    }
}

fn output_matches_from_files(args: &GrepArgs, pattern: &Regex) {
    let files = &find_files(args.files.as_slice(), args.recursive);

    for (i, file) in files.iter().enumerate() {
        match file {
            Ok(file) => match find_matched_lines_from_file(file, &pattern, args.invert_match) {
                Ok(lines) => {
                    if lines.is_empty() {
                        continue;
                    }

                    output_file_match_separator(i, args.count);
                    output_file_match(file, &lines, pattern, args);
                }
                Err(e) => {
                    eprintln!("Error reading file {}: {}", file, e);
                }
            },
            Err(e) => {
                eprintln!("Error accessing file: {}", e);
            }
        }
    }
}

fn output_file_match_separator(i: usize, count: bool) {
    if i > 0 {
        print!("{}", if count { "\n" } else { "\n\n" });
    }
}

fn output_file_match(file: &str, lines: &Vec<LineMatch>, pattern: &Regex, args: &GrepArgs) {
    if args.count {
        output_file_match_count(file, lines.len(), args.color);
    } else {
        output_file_matched_lines(file, lines, &pattern, args.color);
    }
}

fn output_file_matched_lines(path: &str, lines: &Vec<LineMatch>, pattern: &Regex, color: bool) {
    if color {
        println!("{}", path.magenta().bold());
    } else {
        println!("{}", path);
    }

    for (index, (line, num)) in lines.iter().enumerate() {
        if index > 0 {
            println!("");
        }

        if color {
            print!(
                "{}:{}",
                num.to_string().green(),
                pattern.replace_all(line.trim(), "$0".red().to_string())
            );
        } else {
            print!("{}:{}", num, line.trim(),);
        }
    }
}

fn output_file_match_count(path: &str, count: usize, color: bool) {
    if color {
        print!("{}", path.magenta().bold());
    } else {
        print!("{}", path);
    }
    print!(":{}", count);
}

fn find_files(paths: &[String], recursive: bool) -> Vec<std::io::Result<String>> {
    let mut files = vec![];

    for path in paths {
        let metadata = fs::metadata(path);

        match metadata {
            Ok(f) => {
                if f.is_file() {
                    files.push(Ok(path.to_string()));
                } else if f.is_dir() {
                    if recursive {
                        match find_directory_files(path, recursive) {
                            Err(e) => files.push(Err(e)),
                            Ok(sub_files) => {
                                files.extend(sub_files.into_iter().map(Ok));
                            }
                        }
                    } else {
                        files.push(Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("{} is a directory", path),
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

fn find_directory_files<P: AsRef<Path>>(path: P, recursive: bool) -> std::io::Result<Vec<String>> {
    let mut files: Vec<String> = vec![];

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(p) = path.to_str() {
                files.push(p.to_string());
            }
        } else if path.is_dir() && recursive {
            let mut nested_files = find_directory_files(path, recursive)?;
            files.append(&mut nested_files);
        }
    }

    Ok(files)
}

fn find_matched_lines_from_file(
    file: &str,
    pattern: &Regex,
    invert_match: bool,
) -> io::Result<Vec<LineMatch>> {
    let reader: Box<dyn BufRead> = Box::new(BufReader::new(File::open(file)?));
    let lines = find_matched_lines_from_reader(reader, pattern, invert_match)?;
    Ok(lines)
}

fn find_matched_lines_from_reader(
    reader: Box<dyn BufRead>,
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
