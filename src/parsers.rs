use nom::{
    bytes::complete::{is_not, tag, take_till, take_until},
    character::{
        complete::{alpha1, char},
        is_alphanumeric, is_newline,
    },
    sequence::{delimited, preceded},
    IResult,
};
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

/// For parsing up to the next occurence of a desired key
pub fn parse_until_key<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, &'a str> {
    let mut quoted_key = String::from("\t\"");
    quoted_key.push_str(key);

    let (line, value) = take_until(quoted_key.as_str())(file_content)?;

    Ok((line, value))
}

/// For parsing up to the next occurence of a desired key, where keys are unquoted
pub fn parse_until_key_unquoted<'a>(
    file_content: &'a str,
    key: &'a str,
) -> IResult<&'a str, &'a str> {
    let mut key_with_colon = String::from(key);
    key_with_colon.push(':');

    let (line, value) = take_until(key_with_colon.as_str())(file_content)?;

    Ok((line, value))
}
