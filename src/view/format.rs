use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FileFormat {
    /// Text format
    Text,
    /// Hex format
    Hex,
}

impl Default for FileFormat {
    fn default() -> Self {
        FileFormat::Text
    }
}

impl std::str::FromStr for FileFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(FileFormat::Text),
            "hex" => Ok(FileFormat::Hex),
            _ => Err(format!("Invalid file format: {}", s)),
        }
    }
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let format_str = match self {
            FileFormat::Text => "text",
            FileFormat::Hex => "hex",
        };
        write!(f, "{}", format_str)
    }
}
