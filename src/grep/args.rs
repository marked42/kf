use std::io::{self, IsTerminal};
use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::{ArgAction, Args, FromArgMatches};
use regex::{Regex, RegexBuilder};

#[derive(Debug)]
pub struct GrepArgs {
    pub pattern: Regex,
    pub files: Vec<PathBuf>,
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
        let pattern = matches.get_one::<String>("pattern").ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::MissingRequiredArgument,
                "Pattern argument is required",
            )
        })?;
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
            .map(|v| v.map(|s| s.into()).collect())
            .unwrap_or_default();

        let recursive = matches.get_flag("recursive");
        let count = matches.get_flag("count");
        let invert_match = matches.get_flag("invert_match");
        let color = matches
            .get_one::<String>("color")
            .expect("Color option should have a default value");
        let color = match color.as_str() {
            "always" => true,
            "never" => false,
            "auto" => io::stdout().is_terminal(),
            _ => unreachable!("color value parser ensures this doesn't happen"),
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
