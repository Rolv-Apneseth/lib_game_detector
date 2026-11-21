use std::path::{Path, PathBuf};

use tracing::warn;

use super::{LAUNCHER, get_steam_flatpak_dir};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::steam::shortcuts::get_shortcut_games,
    macros::logs::debug_path,
};

#[derive(Debug)]
pub struct SteamShortcutsFlatpak {
    path_steam_userdata_dir: PathBuf,
}

impl SteamShortcutsFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let path_steam_userdata_dir = get_steam_flatpak_dir(path_home).join("userdata");

        debug_path!("userdata dir path", path_steam_userdata_dir);

        Self {
            path_steam_userdata_dir,
        }
    }
}

impl Launcher for SteamShortcutsFlatpak {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_steam_userdata_dir.is_dir()
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        get_shortcut_games(&self.path_steam_userdata_dir, true)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_shortcuts_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_steam_shortcuts_launcher() -> Result<(), GamesParsingError> {
        let path_files_system_mock = get_mock_file_system_path();
        let launcher = SteamShortcutsFlatpak::new(&path_files_system_mock);
        check_shortcuts_launcher(launcher, assert_cmd_flatpak)
    }
}
