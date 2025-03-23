use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

/// Returns an Option containing the given `PathBuf`, if the `PathBuf` points to an actual file
pub fn some_if_file(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// Returns an Option containing the given `PathBuf`, if the `PathBuf` points to an actual directory
pub fn some_if_dir(path: PathBuf) -> Option<PathBuf> {
    path.is_dir().then_some(path)
}

/// Returns the first existing image file path (based on a set number of image extensions) for a given
/// directory path and file name
///
/// e.g. dir/path/file_name.{png,jpg,jpeg} will return the first path which actually exists (or
/// `None` if none of them exist)
pub fn get_existing_image_path(base_path: &Path, file_name: impl Display) -> Option<PathBuf> {
    ["png", "jpg", "jpeg"]
        .iter()
        .find_map(|ext| some_if_file(base_path.join(format!("{file_name}.{ext}"))))
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_get_existing_image_path() {
        let base = PathBuf::new();
        assert_eq!(get_existing_image_path(&base, "does_not_exist.jpg"), None);
    }
}
