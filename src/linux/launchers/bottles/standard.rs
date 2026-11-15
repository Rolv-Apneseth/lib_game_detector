use std::{
    path::{Path, PathBuf},
    process::Command,
};

use super::{LAUNCHER, parse_game_data};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::bottles::get_bottles_dir,
    macros::logs::{debug_path, warn_no_games},
    utils::get_launch_command,
};

fn get_cmd(args: [&str; 5]) -> Command {
    get_launch_command("bottles-cli", args, [])
}

#[derive(Debug)]
pub struct Bottles {
    path_bottles_data_dir: PathBuf,
    path_bottles_library: PathBuf,
    path_bottles_data: PathBuf,
    #[cfg(test)]
    _force_path_bottles_dir: PathBuf,
}

impl Bottles {
    pub fn new(path_data: &Path) -> Self {
        let path_bottles_data_dir = path_data.join("bottles");

        let path_bottles_library = path_bottles_data_dir.join("library.yml");
        let path_bottles_data = path_bottles_data_dir.join("data.yml");

        debug_path!("bottles data dir", path_bottles_data_dir);
        debug_path!("library.yml", path_bottles_library);
        debug_path!("data.yml", path_bottles_data);

        Bottles {
            path_bottles_data_dir,
            path_bottles_library,
            path_bottles_data,
            #[cfg(test)]
            _force_path_bottles_dir: PathBuf::default(),
        }
    }
}

impl Launcher for Bottles {
    fn is_detected(&self) -> bool {
        self.path_bottles_library.exists()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        // Parsing of custom bottles dir only applied at this stage, not in the constructor
        #[allow(unused_variables)]
        let path_bottles_dir =
            get_bottles_dir(&self.path_bottles_data_dir, &self.path_bottles_data);

        // Testing override, since the custom path will be invalid
        #[cfg(test)]
        let path_bottles_dir = self._force_path_bottles_dir.clone();

        let parsed_data = parse_game_data(&path_bottles_dir, &self.path_bottles_library)?;
        if parsed_data.is_empty() {
            warn_no_games!();
        }

        Ok(parsed_data
            .into_iter()
            .map(|d| d.into_game(&path_bottles_dir, get_cmd))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{
        super::test_utils::{check_bottles_launcher, get_test_bottles_path},
        *,
    };
    use crate::{
        data::Game,
        error::GamesParsingError,
        linux::test_utils::{assert_cmd, get_mock_file_system_path},
    };

    #[test_case(false; "standard")]
    #[test_case(true; "standard + external library")]
    fn test_bottles_launcher(use_custom_bottles_path: bool) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let mut launcher = Bottles::new(&path_file_system_mock.join(".local/share"));

        let path_bottles_dir = get_test_bottles_path(
            &path_file_system_mock,
            &launcher.path_bottles_data_dir,
            use_custom_bottles_path,
        );
        launcher._force_path_bottles_dir = path_bottles_dir.clone();

        check_bottles_launcher(launcher, &path_bottles_dir, |g: &Game| {
            assert_cmd(g, "bottles-cli")
        })
    }
}
