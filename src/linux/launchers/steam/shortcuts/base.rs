use std::path::{Path, PathBuf};

use tracing::warn;

use super::{LAUNCHER, get_steam_dir};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::steam::shortcuts::get_shortcut_games,
    macros::logs::debug_path,
};

#[derive(Debug)]
pub struct SteamShortcuts {
    path_steam_userdata_dir: PathBuf,
}

impl SteamShortcuts {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let path_steam_userdata_dir = get_steam_dir(path_home, path_data).join("userdata");

        debug_path!("userdata dir path", path_steam_userdata_dir);

        Self {
            path_steam_userdata_dir,
        }
    }
}

impl Launcher for SteamShortcuts {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_steam_userdata_dir.is_dir()
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        get_shortcut_games(&self.path_steam_userdata_dir, false)
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{
        super::{super::test_utils::assert_cmd_steam, test_utils::check_shortcuts_launcher},
        *,
    };
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case("", ".local/share"; "standard")]
    #[test_case("steam_symlinks", ""; "standard - fallback steam root")]
    fn test_steam_shortcuts_launcher(
        path_home: &str,
        path_data: &str,
    ) -> Result<(), GamesParsingError> {
        let path_files_system_mock = get_mock_file_system_path();
        let launcher = SteamShortcuts::new(
            &path_files_system_mock.join(path_home),
            &path_files_system_mock.join(path_data),
        );
        check_shortcuts_launcher(launcher, assert_cmd_steam)
    }
}
