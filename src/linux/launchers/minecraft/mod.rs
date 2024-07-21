pub mod at;
pub mod prism;

pub fn get_minecraft_title(title: &str) -> String {
    format!("Minecraft: {title}")
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test_case(true, "abc", "Minecraft: abc")]
    #[test_case(true, "012 xyz", "Minecraft: 012 xyz")]
    #[test_case(false, "abc", "abc")]
    fn test_minecraft_at_launcher(should_pass: bool, input: &str, expected_output: &str) {
        assert_eq!(should_pass, get_minecraft_title(input) == expected_output)
    }
}
