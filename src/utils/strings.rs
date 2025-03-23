/// Cleans up parsed game title
pub fn clean_game_title(title: impl AsRef<str>) -> String {
    title.as_ref().replace(['™', '®'], "")
}

#[cfg(test)]
pub mod test {
    use test_case::test_case;

    use super::*;

    #[test_case("Soon™", "Soon")]
    #[test_case("Game®", "Game")]
    #[test_case("®T™i®t™l®e™", "Title")]
    fn test_clean_game_title(dirty: &str, clean: &str) {
        assert_eq!(clean_game_title(dirty), String::from(clean));
    }
}
