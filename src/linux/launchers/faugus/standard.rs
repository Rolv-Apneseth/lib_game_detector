use std::{path::Path, process::Command};

use super::{FaugusPaths, LAUNCHER, get_parsable_games_json_data};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    macros::logs::warn_no_games,
    utils::get_launch_command,
};

fn get_cmd(args: [&str; 2]) -> Command {
    get_launch_command("faugus-launcher", args, [])
}

#[derive(Debug)]
pub struct Faugus {
    paths: FaugusPaths,
}

impl Faugus {
    pub fn new(path_data: &Path) -> Self {
        let path_data_faugus = path_data.join("faugus-launcher");
        let paths = FaugusPaths::new(path_data_faugus);

        Self { paths }
    }
}

impl Launcher for Faugus {
    fn is_detected(&self) -> bool {
        self.paths.games_json.is_file()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = get_parsable_games_json_data(&self.paths.games_json)?;

        if parsed_data.is_empty() {
            warn_no_games!();
        }

        Ok(parsed_data
            .into_iter()
            .filter_map(|d| d.try_into_game(&self.paths, get_cmd))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_faugus_launcher, *};
    use crate::{
        data::Game,
        error::GamesParsingError,
        linux::test_utils::{assert_cmd, get_mock_file_system_path},
    };

    #[test]
    fn test_faugus_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Faugus::new(&path_file_system_mock.join(".local/share"));

        check_faugus_launcher(launcher, |g: &Game| {
            assert_cmd(g, "faugus-launcher");
        })
    }
}
