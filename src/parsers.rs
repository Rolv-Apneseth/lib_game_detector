use nom::{
    bytes::complete::{is_not, tag, take_till, take_until},
    character::complete::{alpha1, char},
    sequence::{delimited, preceded},
    AsChar, IResult, Parser,
};
// GENERAL ----------------------------------------------------------------------------------------
pub fn parse_between_double_quotes(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"')).parse(input)
}

pub fn parse_not_double_quote(input: &str) -> IResult<&str, &str> {
    is_not("\"")(input)
}

pub fn parse_not_alphanumeric(input: &str) -> IResult<&str, &str> {
    take_till(|a| (a as u8).is_alphanum())(input)
}

pub fn parse_till_end_of_line(input: &str) -> IResult<&str, &str> {
    take_till(|a| (a as u8).is_newline())(input)
}

/// Parse both the key and value from a given line of a `.json` file (both values must be quoted)
pub fn parse_double_quoted_key_value(line: &str) -> IResult<&str, (&str, &str)> {
    let (line, key) = preceded(parse_not_double_quote, parse_between_double_quotes).parse(line)?;
    let (line, value) =
        preceded(parse_not_double_quote, parse_between_double_quotes).parse(line)?;

    Ok((line, (key, value)))
}

// KEYS -------------------------------------------------------------------------------------------
/// Parses up to the next occurrence of a desired key in a `.json` file
/// e.g. "keyName": value
pub fn parse_until_key_json<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let quoted_key = format!("\"{key}\"");

    let (file_content, _) = take_until(quoted_key.as_str()).parse(file_content)?;
    Ok((file_content, quoted_key))
}

/// Parses up to the next occurrence of a desired key in a `.yml` or `.json`-style file without quotes
/// e.g. keyName: value
pub fn parse_until_key_yml<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let key_with_colon = format!("{key}:");

    let (file_content, _) = take_until(key_with_colon.as_str()).parse(file_content)?;
    Ok((file_content, key_with_colon))
}

/// Parses up to the next occurrence of a desired key in a `.cfg` file
/// e.g. keyName=value
pub fn parse_until_key_cfg<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let key_with_equals = format!("{key}=");

    let (file_content, _) = take_until(key_with_equals.as_str()).parse(file_content)?;
    Ok((file_content, key_with_equals))
}

// VALUES -----------------------------------------------------------------------------------------
/// Find the next occurrence of a key in a `.json` file and returns the matching value (quoted)
/// e.g. "keyName": "value"
pub fn parse_value_json<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let (file_content, matched_key) = parse_until_key_json(file_content, key)?;
    let (file_content, _) = tag(matched_key.as_str()).parse(file_content)?;

    let (file_content, value) =
        preceded(parse_not_double_quote, parse_between_double_quotes).parse(file_content)?;

    Ok((file_content, value.to_string()))
}

/// Find the next occurrence of a key in a `.json` file and returns the matching value (unquoted)
/// e.g. "keyName": false
pub fn parse_value_json_unquoted<'a>(
    file_content: &'a str,
    key: &'a str,
) -> IResult<&'a str, String> {
    let (file_content, matched_key) = parse_until_key_json(file_content, key)?;
    let (file_content, _) = tag(matched_key.as_str()).parse(file_content)?;

    let (file_content, value) = preceded(parse_not_alphanumeric, alpha1).parse(file_content)?;

    Ok((file_content, value.to_string()))
}

/// Find the next occurrence of a key in a `.yml` file and returns the matching value
/// e.g. keyName: value
pub fn parse_value_yml<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let (file_content, matched_key) = parse_until_key_yml(file_content, key)?;

    let (file_content, _) = tag(matched_key.as_str()).parse(file_content)?;

    let (file_content, value) =
        preceded(parse_not_alphanumeric, parse_till_end_of_line).parse(file_content)?;

    Ok((file_content, value.to_string()))
}

/// Find the next occurrence of a key in a `.cfg` file and returns the matching value
/// e.g. keyName=value
pub fn parse_value_cfg<'a>(file_content: &'a str, key: &'a str) -> IResult<&'a str, String> {
    let (file_content, matched_key) = parse_until_key_cfg(file_content, key)?;
    let (file_content, _) = tag(matched_key.as_str()).parse(file_content)?;

    let (file_content, value) = parse_till_end_of_line(file_content)?;

    Ok((file_content, value.to_string()))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test_case("\t\"key\": \"value\"", "key", "value", true)]
    #[test_case("\"key\": \"value\"", "key", "value", false)]
    #[test_case("\"key\": false", "key", "false", false)]
    #[test_case("\n{\tkey: \"value\"}", "key", "value", false)]
    #[test_case("{\n\t\"emn\": \"abc\"}", "emn", "abc", true)]
    #[test_case("\t\"key\": \"new\nline\"", "key", "new\nline", true)]
    #[test_case("\tkey=value", "key", "value", false)]
    #[test_case("{\"key1\": \"val1\", \"key2\": \"val2\"\n}", "key1", "val1", true)]
    #[test_case("{\"key1\": \"val2\", \"key2\": \"val2\"\n}", "key1", "val1", false)]
    #[test_case("{\"key1\": \"val1\", \"key2\": \"val2\"\n}", "key2", "val1", false)]
    fn test_parse_double_quoted_key_value(line: &str, key: &str, value: &str, should_pass: bool) {
        if let Ok((_, (parsed_key, parsed_value))) = parse_double_quoted_key_value(line) {
            assert_eq!(key == parsed_key && value == parsed_value, should_pass);
        } else {
            assert!(!should_pass);
        }
    }

    #[test_case("\"key\": value", "key", true)]
    #[test_case("\n\t\"aio\": value", "aio", true)]
    #[test_case("{\n\"wrong_key\": value1\n\"key\": value\n}", "key", true)]
    #[test_case("{\"wrong_key\": value1\n\"abc\": value2\n}", "abc", true)]
    #[test_case("{\"key\": value1}", "abc", false)]
    #[test_case("{key: value1\n\"abc\": value2\n}", "key", false)]
    fn test_parse_until_key_json(file_content: &str, key: &str, should_pass: bool) {
        assert_eq!(parse_until_key_json(file_content, key).is_ok(), should_pass);
    }

    #[test_case("key: value", "key", true)]
    #[test_case("lmn: value", "lmn", true)]
    #[test_case("{\nwrong_key: value1\nkey: value\n}", "key", true)]
    #[test_case("key1: value1\nkey: value2\n", "key", true)]
    #[test_case("{\n\twrong_key: value1\n\tabc: value2\n}", "abc", true)]
    #[test_case("\"key\": value", "key", false)]
    #[test_case("{\n\"wrong_key\": value1\n\"key\": value\n}", "key", false)]
    fn test_parse_until_key_yml(file_content: &str, key: &str, should_pass: bool) {
        assert_eq!(parse_until_key_yml(file_content, key).is_ok(), should_pass);
    }

    #[test_case("key=value", "key", true)]
    #[test_case("\n\taio=value", "aio", true)]
    #[test_case("\nwrong_key=value1\nkey=value\n", "key", true)]
    #[test_case("wrong_key=value1\nabc=value2\n", "abc", true)]
    #[test_case("key=value1", "abc", false)]
    #[test_case("key: value1\n\"key\"=value2\n", "key", false)]
    fn test_parse_until_key_cfg(file_content: &str, key: &str, should_pass: bool) {
        assert_eq!(parse_until_key_cfg(file_content, key).is_ok(), should_pass);
    }

    #[test_case("{\"key\": \"value\"}", "key", "value", true)]
    #[test_case("{\"key\": \"value\"}", "key", "wrong_value", false)]
    #[test_case(
        "{\"wrong_key1\": false, \"wrong_key2\": \"value1\"\n\"key\": \"value2\"\n}",
        "key",
        "value2",
        true
    )]
    #[test_case("{key: \"value\"}", "key", "value", false)]
    #[test_case(
        "{\"wrong_key1\": false, \"wrong_key2\": \"value1\"\n\"key\": value2\n}",
        "key",
        "value2",
        false
    )]
    fn test_parse_value_json(file_content: &str, key: &str, value: &str, should_pass: bool) {
        if let Ok((_, parsed_value)) = parse_value_json(file_content, key) {
            assert_eq!(value == parsed_value, should_pass);
        } else {
            assert!(!should_pass);
        }
    }

    #[test_case("\"key\": true", "key", "true", true)]
    #[test_case("\"key\": true", "key", "false", false)]
    #[test_case("{key: \"true\"}", "key", "true", false)]
    #[test_case("key=value", "key", "value", false)]
    #[test_case(
        "{\"wrong_key1\": false, \"wrong_key2\": \"value1\"\n\"key\": true\n}",
        "key",
        "true",
        true
    )]
    #[test_case(
        "{\"wrong_key1\": false, \"wrong_key2\": \"value1\"\n\"key\": \"value2\"\n}",
        "key",
        "value2",
        false
    )]
    fn test_parse_value_json_unquoted(
        file_content: &str,
        key: &str,
        value: &str,
        should_pass: bool,
    ) {
        if let Ok((_, parsed_value)) = parse_value_json_unquoted(file_content, key) {
            assert_eq!(value == parsed_value, should_pass);
        } else {
            assert!(!should_pass);
        }
    }

    #[test_case("data:\n\tkey: value", "key", "value", true)]
    #[test_case("data:\n\t\"key\": value", "key", "value", false)]
    #[test_case("key=value", "key", "value", false)]
    #[test_case(
        "data:\n\twrong_key1: false\n\twrong_key2: value1\n\tkey: value2\n",
        "key",
        "value2",
        true
    )]
    #[test_case(
        "data:\n\twrong_key1: false\n\twrong_key2: value1\n\tkey: abc\n",
        "key",
        "value2",
        false
    )]
    fn test_parse_value_yml(file_content: &str, key: &str, value: &str, should_pass: bool) {
        if let Ok((_, parsed_value)) = parse_value_yml(file_content, key) {
            assert_eq!(value == parsed_value, should_pass);
        } else {
            assert!(!should_pass);
        }
    }

    #[test_case("\nkey=value", "key", "value", true)]
    #[test_case("key=value1\n", "key", "value", false)]
    #[test_case("key=value with space\n", "key", "value with space", true)]
    #[test_case("key=no new\nline\n", "key", "no new", true)]
    #[test_case("key=\nvalue", "key", "value", false)]
    #[test_case("\"key\": \"value\"", "key", "value", false)]
    #[test_case(
        "wrong_key1=false\nwrong_key2=value1\nabc=value2\n",
        "abc",
        "value2",
        true
    )]
    #[test_case(
        "data:\n\twrong_key1=false\n\twrong_key2=value1\n\tkey=abc\n",
        "key",
        "value2",
        false
    )]
    fn test_parse_value_cfg(file_content: &str, key: &str, value: &str, should_pass: bool) {
        if let Ok((_, parsed_value)) = parse_value_cfg(file_content, key) {
            assert_eq!(value == parsed_value, should_pass);
        } else {
            assert!(!should_pass);
        }
    }
}
