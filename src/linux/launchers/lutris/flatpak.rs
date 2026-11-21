use std::{path::Path, process::Command};

use super::{LAUNCHER, LutrisPaths, get_db_data};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    macros::logs::warn_no_games,
    utils::get_launch_command_flatpak,
};

fn get_cmd(uri: &str, env_vars: [(&str, &str); 1]) -> Command {
    get_launch_command_flatpak("net.lutrsi.Lutris", [], [uri], env_vars)
}

#[derive(Debug)]
pub struct LutrisFlatpak {
    paths: LutrisPaths,
}

impl LutrisFlatpak {
    pub fn new(path_home: &Path, path_config: &Path, path_cache: &Path) -> Self {
        let path_data = path_home.join(".var/app/net.lutris.Lutris/data");
        let paths = LutrisPaths::new(&path_data, path_config, path_cache);

        LutrisFlatpak { paths }
    }
}

impl Launcher for LutrisFlatpak {
    fn is_detected(&self) -> bool {
        self.paths.pga_db.is_file() && self.paths.box_art_dir.is_dir()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = get_db_data(&self.paths.pga_db)?;

        if parsed_data.is_empty() {
            warn_no_games!();
        }

        Ok(parsed_data
            .into_iter()
            .map(|d| d.into_game(&self.paths, get_cmd))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_lutris_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_lutris_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = LutrisFlatpak::new(
            &path_file_system_mock,
            &path_file_system_mock.join(".config"),
            &path_file_system_mock.join(".cache"),
        );

        check_lutris_launcher(launcher, assert_cmd_flatpak)
    }
}
