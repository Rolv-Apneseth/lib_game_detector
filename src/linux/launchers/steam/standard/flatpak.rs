use std::path::{Path, PathBuf};

use super::{
    LAUNCHER, get_games_from_libraries, get_steam_flatpak_dir, get_steam_libraries,
    get_steamapps_dir,
};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    macros::logs::debug_path,
};

#[derive(Debug)]
pub struct SteamFlatpak {
    path_steam_dir: PathBuf,
    path_libraries_vdf: PathBuf,
}

impl SteamFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let path_steam_dir = get_steam_flatpak_dir(path_home);
        let path_libraries_vdf = get_steamapps_dir(&path_steam_dir).join("libraryfolders.vdf");

        debug_path!("main Steam directory", path_steam_dir);
        debug_path!("libraryfolders.vdf", path_libraries_vdf);

        SteamFlatpak {
            path_steam_dir,
            path_libraries_vdf,
        }
    }
}

impl Launcher for SteamFlatpak {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_steam_dir.is_dir() && self.path_libraries_vdf.is_file()
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let libraries = get_steam_libraries(&self.path_steam_dir, &self.path_libraries_vdf, false)?;
        get_games_from_libraries(libraries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux::test_utils::get_mock_file_system_path;

    #[test]
    fn test_steam_launcher() {
        let path_files_system_mock = get_mock_file_system_path();
        let launcher = SteamFlatpak::new(&path_files_system_mock);

        assert!(launcher.is_detected());

        // Minor check to ensure debug formatting for `SupportedLaunchers` works as intended
        assert_eq!(format!("{:?}", launcher.get_launcher_type()), "Steam");

        // Library paths in `libraryfolders.vdf` mock are invalid library paths
        let games_res = launcher.get_detected_games();
        assert!(games_res.is_ok_and(|g| g.is_empty()));
    }
}
