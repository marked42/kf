use std::{borrow::Cow, str::FromStr};

use thiserror::Error;

pub type RangePos = i64;
pub type RangeCount = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum RangeSpec {
    /// line number "10" "-1"
    Single(RangePos),

    /// "10..20"
    Range(RangePos, RangePos),

    // "10.."
    From(RangePos),

    /// "..20"
    To(RangePos),

    /// "10+5"
    FromCount(RangePos, RangeCount),

    /// 1,3,5,7..9
    List(Vec<RangeSpec>),

    /// ~1..3
    Complement(Box<RangeSpec>),

    ///
    All,
}

impl RangeSpec {
    pub fn normalize_line(line: RangePos, total: RangeCount) -> RangePos {
        if line < 0 {
            line + 1 + total as RangePos
        } else {
            line
        }
    }

    pub fn is_negative(line: RangePos) -> bool {
        line < 0
    }

    pub fn normalize<'a>(&'a self, total: RangeCount) -> Cow<'a, RangeSpec> {
        match self {
            RangeSpec::Single(pos) => {
                if RangeSpec::is_negative(*pos) {
                    let normalized = RangeSpec::normalize_line(*pos, total);
                    Cow::Owned(RangeSpec::Single(normalized))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::Range(start, end) => {
                if RangeSpec::is_negative(*start) || RangeSpec::is_negative(*end) {
                    let normalized_start = RangeSpec::normalize_line(*start, total);
                    let normalized_end = RangeSpec::normalize_line(*end, total);
                    Cow::Owned(RangeSpec::Range(normalized_start, normalized_end))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::From(start) => {
                if RangeSpec::is_negative(*start) {
                    let normalized = RangeSpec::normalize_line(*start, total);
                    Cow::Owned(RangeSpec::From(normalized))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::To(end) => {
                if RangeSpec::is_negative(*end) {
                    let normalized = RangeSpec::normalize_line(*end, total);
                    Cow::Owned(RangeSpec::To(normalized))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::FromCount(start, count) => {
                if RangeSpec::is_negative(*start) {
                    let normalized = RangeSpec::normalize_line(*start, total);
                    Cow::Owned(RangeSpec::FromCount(normalized, *count))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::List(specs) => {
                let mut has_changed = false;
                let normalized_specs: Vec<Cow<'a, RangeSpec>> = specs
                    .iter()
                    .map(|spec| {
                        let normalized = spec.normalize(total);
                        if let Cow::Owned(_) = &normalized {
                            has_changed = true;
                        }
                        normalized
                    })
                    .collect();

                if has_changed {
                    let owned_specs =
                        normalized_specs.into_iter().map(|spec| spec.into_owned()).collect();
                    Cow::Owned(RangeSpec::List(owned_specs))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::Complement(spec) => {
                let normalized_spec = spec.normalize(total);
                if let Cow::Owned(_) = &normalized_spec {
                    Cow::Owned(RangeSpec::Complement(normalized_spec.into_owned().into()))
                } else {
                    Cow::Borrowed(self)
                }
            }
            RangeSpec::All => Cow::Borrowed(self),
        }
    }

    pub fn contains(&self, line_no: RangePos) -> bool {
        match self {
            RangeSpec::Single(pos) => *pos == line_no,
            RangeSpec::Range(start, end) => *start <= line_no && line_no <= *end,
            RangeSpec::From(start) => *start <= line_no,
            RangeSpec::To(end) => line_no <= *end,
            RangeSpec::FromCount(start, count) => {
                *start <= line_no && line_no <= *start + (*count as RangePos - 1)
            }
            RangeSpec::List(range_specs) => range_specs.iter().any(|spec| spec.contains(line_no)),
            RangeSpec::Complement(range_spec) => !range_spec.contains(line_no),
            RangeSpec::All => true,
        }
    }
}

impl Default for RangeSpec {
    fn default() -> Self {
        RangeSpec::All
    }
}

pub struct RangeSpecParser<'a> {
    pos: usize,
    input: &'a str,
}

fn byte_to_digit(byte: u8) -> Result<RangePos, String> {
    if byte.is_ascii_digit() {
        Ok((byte - b'0') as RangePos)
    } else {
        Err(format!("{byte} is not valid ascii digit"))
    }
}

impl<'a> RangeSpecParser<'a> {
    fn new(input: &'a str) -> Self {
        RangeSpecParser { input, pos: 0 }
    }

    fn advance(&mut self, len: usize) {
        self.pos += len;
    }

    fn peek(&self, len: usize) -> &str {
        let end = usize::min(self.input.len(), self.pos + len);
        &self.input[self.pos..end]
    }

    fn peek_all(&self) -> &str {
        &self.input[self.pos..self.input.len()]
    }

    fn find<F>(&self, predicate: F) -> Option<usize>
    where
        F: Fn(u8) -> bool,
    {
        let mut i = self.pos;
        while let Some(byte) = self.input.as_bytes().get(i) {
            if predicate(*byte) {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn peek_until<F>(&self, predicate: F) -> &str
    where
        F: Fn(u8) -> bool,
    {
        let end = self.find(predicate).unwrap_or(self.input.as_bytes().len());
        &self.input[self.pos..end]
    }

    fn peek_until_whitespace(&self) -> &str {
        self.peek_until(|b| u8::is_ascii_whitespace(&b))
    }

    fn take(&mut self, text: &str) -> Result<bool, ParseError> {
        if self.peek(text.len()) != text {
            return Err(ParseError::UnexpectedInput {
                expected: text.to_string(),
                actual: self.peek(text.len()).to_string(),
            });
        }

        self.advance(text.len());
        Ok(true)
    }

    fn start_with(&self, text: &str) -> bool {
        self.peek(text.len()) == text
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).map(|v| *v)
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn parse(&mut self) -> Result<RangeSpec, ParseError> {
        if self.eof() {
            return Err(ParseError::Empty);
        }

        let range = if self.input == "-" {
            self.take("-")?;
            Ok(RangeSpec::All)
        } else if self.start_with("~") {
            self.take("~")?;
            Ok(RangeSpec::Complement(Box::new(self.parse_list_or_basic()?)))
        } else {
            self.parse_list_or_basic()
        }?;

        if !self.eof() {
            Err(ParseError::UnconsumedInput(self.peek_all().to_string()))
        } else {
            Ok(range)
        }
    }

    fn parse_list_or_basic(&mut self) -> Result<RangeSpec, ParseError> {
        if self.input.contains(",") {
            self.parse_list()
        } else {
            self.parse_basic()
        }
    }

    fn parse_list(&mut self) -> Result<RangeSpec, ParseError> {
        let mut ranges: Vec<RangeSpec> = Vec::new();

        while !self.eof() {
            let range = self.parse_basic()?;
            ranges.push(range);
            if let Some(byte) = self.peek_byte() {
                if byte == b',' {
                    self.advance(1);
                } else {
                    break;
                }
            }
        }

        Ok(RangeSpec::List(ranges))
    }

    fn parse_number(&mut self) -> Result<RangePos, ParseError> {
        let start_pos = self.pos;

        let Some(byte) = self.peek_byte() else {
            return Err(ParseError::EarlyEof(
                "expect number starting with '-' or digit(1-9)".to_string(),
            ));
        };

        let (sign, mut value) = match byte {
            b'-' => {
                self.advance(1);
                (-1, 0 as RangePos)
            }
            b'0' => {
                self.advance(1);
                return Ok(0);
            }
            b'1'..=b'9' => {
                self.advance(1);
                (
                    1,
                    byte_to_digit(byte).expect(&format!("{} is ascii digit", byte)),
                )
            }
            _ => {
                return Err(ParseError::InvalidNumber(
                    self.peek_until_whitespace().to_string(),
                ));
            }
        };

        while let Some(byte) = self.peek_byte() {
            if !byte.is_ascii_digit() {
                break;
            }
            let number_text = self.input
                [start_pos..self.find(|b| !b.is_ascii_digit()).unwrap_or(self.input.len())]
                .to_string();

            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(byte_to_digit(byte).expect("{byte} is ascii digit")))
                .ok_or_else(|| ParseError::NumberTooLarge(number_text))?;
            self.advance(1);
        }

        Ok(sign * value)
    }

    fn parse_to(&mut self) -> Result<RangeSpec, ParseError> {
        self.take("..")?;
        let val = self.parse_number()?;
        Ok(RangeSpec::To(val))
    }

    fn parse_basic(&mut self) -> Result<RangeSpec, ParseError> {
        let Some(byte) = self.peek_byte() else {
            return Err(ParseError::EarlyEof(
                "'-' or digit(1-9) or '..'".to_string(),
            ));
        };

        match byte {
            b'-' | b'1'..=b'9' => {
                let start = self.parse_number()?;
                match self.peek_byte() {
                    Some(b'.') => {
                        self.take("..")?;
                        let text = self.peek_until_whitespace();
                        if text.is_empty() {
                            return Ok(RangeSpec::From(start));
                        }
                        let end = self.parse_number()?;
                        Ok(RangeSpec::Range(start, end))
                    }
                    Some(b'+') => {
                        self.take("+")?;
                        let count = self.parse_number()?;
                        if count < 0 {
                            Err(ParseError::InvalidRangeCount(count))
                        } else {
                            Ok(RangeSpec::FromCount(start, count as RangeCount))
                        }
                    }
                    _ => Ok(RangeSpec::Single(start)),
                }
            }
            b'.' => self.parse_to(),
            _ => Err(ParseError::InvalidNumber(
                self.peek_until_whitespace().to_string(),
            )),
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("empty range string not allowed")]
    Empty,

    #[error("Unexpected early eof, {0}")]
    EarlyEof(String),

    #[error("number should start with '-' or digit(1-9), get {0}")]
    InvalidNumber(String),

    #[error("number too large: {0}")]
    NumberTooLarge(String),

    #[error("invalid negative range count: {0}")]
    InvalidRangeCount(i64),

    #[error("unexpected input, expected: {}, actual: {}", expected, actual)]
    UnexpectedInput { expected: String, actual: String },

    #[error("Unconsumed input: {0}")]
    UnconsumedInput(String),
}

impl FromStr for RangeSpec {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RangeSpecParser::new(s).parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_spec_parsing() {
        assert_eq!("-".parse::<RangeSpec>().unwrap(), RangeSpec::All);

        assert_eq!("10".parse::<RangeSpec>().unwrap(), RangeSpec::Single(10));

        assert_eq!(
            "10..20".parse::<RangeSpec>().unwrap(),
            RangeSpec::Range(10, 20)
        );

        assert_eq!("..20".parse::<RangeSpec>().unwrap(), RangeSpec::To(20));

        assert_eq!("10..".parse::<RangeSpec>().unwrap(), RangeSpec::From(10));

        assert_eq!(
            "10+5".parse::<RangeSpec>().unwrap(),
            RangeSpec::FromCount(10, 5)
        );

        assert_eq!(
            "1,3,5,10..20".parse::<RangeSpec>().unwrap(),
            RangeSpec::List(vec![
                RangeSpec::Single(1),
                RangeSpec::Single(3),
                RangeSpec::Single(5),
                RangeSpec::Range(10, 20),
            ])
        );

        assert_eq!(
            "~-5".parse::<RangeSpec>().unwrap(),
            RangeSpec::Complement(Box::new(RangeSpec::Single(-5)))
        );
    }

    #[test]
    fn test_parse_error_empty() {
        let result = "".parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::Empty));
    }

    #[test]
    fn test_parse_error_early_eof() {
        let result = "10+".parse::<RangeSpec>();
        assert_eq!(
            result,
            Err(ParseError::EarlyEof(
                "expect number starting with '-' or digit(1-9)".to_string()
            ))
        );

        let result = "..".parse::<RangeSpec>();
        assert_eq!(
            result,
            Err(ParseError::EarlyEof(
                "expect number starting with '-' or digit(1-9)".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_error_invalid_number() {
        let result = "abc".parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::InvalidNumber("abc".to_string())));

        let result = "012".parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::InvalidNumber("012".to_string())));

        let result = "10..abc".parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::InvalidNumber("abc".to_string())));
    }

    #[test]
    fn number_too_large() {
        let number = "99999999999999999999999999999999999".to_string();
        let result = number.parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::NumberTooLarge(number)));
    }

    #[test]
    fn test_parse_error_invalid_range_count() {
        let result = "10+-5".parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::InvalidRangeCount(-5)));

        let result = "-10+-5".parse::<RangeSpec>();
        assert_eq!(result, Err(ParseError::InvalidRangeCount(-5)));
    }

    #[test]
    fn test_parse_error_unexpected_input() {
        let result = "~abc".parse::<RangeSpec>();
        assert!(result.is_err());

        let result = "10 20 30".parse::<RangeSpec>();
        assert_eq!(
            result,
            Err(ParseError::UnconsumedInput(" 20 30".to_string()))
        );
    }
}
