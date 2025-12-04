use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{fs, io};

use colored::Colorize;
use regex::Regex;

use crate::cli::GrepArgs;

type LineMatch = (String, usize);

pub fn grep(args: GrepArgs) {
    let pattern = match args.compiled_pattern() {
        Ok(pattern) => pattern,
        Err(e) => {
            eprintln!("Error compiling pattern: {}", e);
            return;
        }
    };

    if args.files.is_empty() {
        find_match_from_terminal(&pattern, args.color);
    } else {
        find_match_from_files(&args, &pattern);
    }
}

fn find_match_from_terminal(pattern: &Regex, color: bool) {
    let reader = std::io::stdin().lock();

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

fn find_match_from_files(args: &GrepArgs, pattern: &Regex) {
    let files = &find_files(args.files.as_slice(), args.recursive);

    for file in files {
        match file {
            Ok(file) => {
                if let Err(e) = process_file(file, &pattern, &args) {
                    eprintln!("Error processing file: {}", e);
                    eprintln!("");
                }
            }
            Err(e) => {
                eprintln!("Error accessing file: {}", e);
                eprintln!("");
            }
        }
    }
}

fn process_file(file: &str, pattern: &Regex, args: &GrepArgs) -> io::Result<()> {
    let lines = find_matched_lines_in_file(file, &pattern, args.invert_match)?;

    if lines.is_empty() {
        return Ok(());
    }

    if args.count {
        output_file_match_count(file, lines.len(), args.color);
    } else {
        output_file_matched_lines(file, lines, &pattern, args.color);
    }

    Ok(())
}

fn output_file_matched_lines(path: &str, lines: Vec<LineMatch>, pattern: &Regex, color: bool) {
    if color {
        println!("{}", path.magenta().bold());
    } else {
        println!("{}", path);
    }

    for (line, num) in lines {
        if color {
            println!(
                "{}:{}",
                num.to_string().green(),
                pattern.replace_all(line.trim(), "$0".red().to_string())
            );
        } else {
            println!("{}:{}", num, line.trim(),);
        }
    }
    println!("");
}

fn output_file_match_count(path: &str, count: usize, color: bool) {
    if color {
        print!("{}", path.magenta().bold());
    } else {
        print!("{}", path);
    }
    println!(":{}", count);
}

fn find_files(paths: &[String], recursive: bool) -> Vec<Result<String, io::Error>> {
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

fn find_directory_files<P: AsRef<Path>>(
    path: P,
    recursive: bool,
) -> Result<Vec<String>, io::Error> {
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

fn find_matched_lines_in_file(
    file_path: impl AsRef<Path>,
    pattern: &Regex,
    invert_match: bool,
) -> Result<Vec<LineMatch>, io::Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

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
