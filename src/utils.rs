use nom::{
    bytes::complete::{is_not, tag, take_till},
    character::{
        complete::{alpha1, char},
        is_alphanumeric, is_newline,
    },
    sequence::{delimited, preceded},
    IResult,
};
use std::path::PathBuf;

/// Cleans up parsed game title
pub fn clean_game_title(title: &str) -> String {
    title.replace(['™', '®'], "")
}

/// Returns an Option containing the given PathBuf, if the PathBuf points to an actual file
pub fn some_if_file(path: PathBuf) -> Option<PathBuf> {
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Returns an Option containing the given PathBuf, if the PathBuf points to an actual directory
pub fn some_if_dir(path: PathBuf) -> Option<PathBuf> {
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

// NOM PARSERS -------------------------------------------------------------------
pub fn parse_between_double_quotes(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"'))(input)
}

pub fn parse_not_double_quote(input: &str) -> IResult<&str, &str> {
    is_not("\"")(input)
}

pub fn parse_not_alphanumeric(input: &str) -> IResult<&str, &str> {
    take_till(|a| is_alphanumeric(a as u8))(input)
}

pub fn parse_till_end_of_line(input: &str) -> IResult<&str, &str> {
    take_till(|a| is_newline(a as u8))(input)
}

/// For when both the key and value are needed (or key doesn't matter), and are both double quoted
pub fn parse_double_quoted_key_value(line: &str) -> IResult<&str, (&str, &str)> {
    let (line, key) = preceded(parse_not_double_quote, parse_between_double_quotes)(line)?;
    let (line, value) = preceded(parse_not_double_quote, parse_between_double_quotes)(line)?;

    Ok((line, (key, value)))
}

/// For general parsing of config files, matches a given key and returns the associated value
pub fn parse_double_quoted_value<'a>(line: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let (line, matched_key) = preceded(parse_not_double_quote, parse_between_double_quotes)(line)?;

    // Ensure parsed key matches given key
    tag(key)(matched_key)?;

    let (line, value) = preceded(parse_not_double_quote, parse_between_double_quotes)(line)?;

    Ok((line, value.to_string()))
}

/// For parsing of values in JSON files which aren't double quoted, like boolean values
pub fn parse_unquoted_json_value<'a>(line: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let (line, matched_key) = preceded(parse_not_double_quote, parse_between_double_quotes)(line)?;

    // Ensure parsed key matches given key
    tag(key)(matched_key)?;

    let (line, value) = preceded(parse_not_alphanumeric, alpha1)(line)?;

    Ok((line, value.to_string()))
}

/// For parsing values from `.yml` config files
pub fn parse_unquoted_value<'a>(line: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let (line, matched_key) = preceded(parse_not_alphanumeric, is_not(":"))(line)?;

    tag(key)(matched_key)?;

    let (line, value) = preceded(parse_not_alphanumeric, parse_till_end_of_line)(line)?;

    Ok((line, value.to_string()))
}
