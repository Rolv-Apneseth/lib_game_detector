use std::path::Path;

use super::{HeroicEpicPaths, LAUNCHER, parse_legendary_library};
use crate::{
    data::{GamesResult, Launcher},
    linux::launchers::heroic::get_launch_command_for_heroic_source,
};

#[derive(Debug)]
pub struct HeroicEpic {
    paths: HeroicEpicPaths,
}

impl HeroicEpic {
    pub fn new(path_config: &Path) -> Self {
        let paths = HeroicEpicPaths::new_standard(path_config);
        HeroicEpic { paths }
    }
}

impl Launcher for HeroicEpic {
    fn is_detected(&self) -> bool {
        self.paths.legendary_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = parse_legendary_library(&self.paths.legendary_library)?;
        Ok(parsed_data
            .into_iter()
            .map(|d| {
                d.into_game(
                    "legendary",
                    &self.paths.icons,
                    LAUNCHER.clone(),
                    get_launch_command_for_heroic_source,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_heroic_epic_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_xdg, get_mock_file_system_path},
    };

    #[test]
    fn test_heroic_epic_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicEpic::new(&path_file_system_mock.join(".config"));

        check_heroic_epic_launcher(launcher, assert_cmd_xdg)
    }
}
