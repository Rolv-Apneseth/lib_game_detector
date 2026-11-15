use std::{path::Path, process::Command};

use super::{LAUNCHER, LutrisPaths, get_db_data};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    macros::logs::warn_no_games,
    utils::get_launch_command,
};

fn get_cmd(uri: &str, env_vars: [(&str, &str); 1]) -> Command {
    get_launch_command("lutris", [uri], env_vars)
}

#[derive(Debug)]
pub struct Lutris {
    paths: LutrisPaths,
}

impl Lutris {
    pub fn new(path_data: &Path, path_config: &Path, path_cache: &Path) -> Self {
        let paths = LutrisPaths::new(path_data, path_config, path_cache);

        Lutris { paths }
    }
}

impl Launcher for Lutris {
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
    use test_case::test_case;

    use super::{super::test_utils::check_lutris_launcher, *};
    use crate::{
        data::Game,
        error::GamesParsingError,
        linux::test_utils::{assert_cmd, get_mock_file_system_path},
    };

    #[test_case(".config", ".cache"; "standard")]
    #[test_case("invalid/path", ".cache"; "fallback")]
    fn test_lutris_launcher(path_config: &str, path_cache: &str) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Lutris::new(
            &path_file_system_mock.join(".local/share"),
            &path_file_system_mock.join(path_config),
            &path_file_system_mock.join(path_cache),
        );

        check_lutris_launcher(launcher, |g: &Game| assert_cmd(g, "lutris"))
    }
}
