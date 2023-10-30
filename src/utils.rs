use std::path::PathBuf;

/// Cleans up parsed game title
pub fn clean_game_title(title: &str) -> String {
    title.replace(['™', '®'], "")
}

/// Returns an Option containing the given PathBuf, if the PathBuf points to an actual file
pub fn some_if_file(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// Returns an Option containing the given PathBuf, if the PathBuf points to an actual directory
pub fn some_if_dir(path: PathBuf) -> Option<PathBuf> {
    path.is_dir().then_some(path)
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_clean_game_title() {
        assert_eq!(clean_game_title("Soon™"), String::from("Soon"));
        assert_eq!(clean_game_title("Game®"), String::from("Game"));
        assert_eq!(clean_game_title("®T™i®t™l®e™"), String::from("Title"));
    }
}
