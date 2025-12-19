use std::path::PathBuf;

use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::error::ErrorKind;
use clap::{Parser, builder::RangedU64ValueParser};

use super::format::FileFormat;
use super::range::RangeSpec;

const BYTES_PER_LINE: u64 = 16;

#[derive(Debug, Parser)]
pub struct ViewArgs {
    #[arg(
        index = 1,
        help = "Files to view in specified format, standard input use when not files specified"
    )]
    pub file_paths: Vec<PathBuf>,

    #[arg(long, help = "File format", default_value = "text")]
    pub format: FileFormat,

    #[arg(long,
        help = "bytes per line for hex view",
        default_value_t = BYTES_PER_LINE as usize,
        value_parser = RangedU64ValueParser::<usize>::new().range(BYTES_PER_LINE..(usize::MAX as u64))
    )]
    pub bytes_per_line: usize,

    #[arg(
        short,
        long,
        help = "Suppress printing of header when multiple files are provided"
    )]
    pub quite: bool,

    #[arg(
        short = 'n',
        long,
        help = "Lines to output. Use '-' for all lines, e.g., '1-5' or '10'",
        default_value = "-",
        // required to work with default_missing_value
        num_args = 0..=1,
        default_missing_value = "-",
        value_parser = clap::value_parser!(RangeSpec))
    ]
    pub lines: RangeSpec,
}

#[derive(Clone)]
pub struct RangeSpecValueParser;

impl ValueParserFactory for RangeSpec {
    type Parser = RangeSpecValueParser;

    fn value_parser() -> Self::Parser {
        RangeSpecValueParser
    }
}

impl TypedValueParser for RangeSpecValueParser {
    type Value = RangeSpec;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value_str = value
            .to_str()
            .ok_or_else(|| clap::Error::new(ErrorKind::InvalidUtf8).with_cmd(cmd))?;

        value_str.parse().map_err(|err_msg| {
            let mut err = clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
            if let Some(arg) = arg {
                err.insert(
                    clap::error::ContextKind::InvalidArg,
                    clap::error::ContextValue::String(arg.to_string()),
                );
            }
            err.insert(
                clap::error::ContextKind::InvalidValue,
                clap::error::ContextValue::String(format!("{}: {}", value_str, err_msg)),
            );
            err
        })
    }
}
