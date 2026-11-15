use std::path::Path;

use tracing::warn;

use super::{HeroicSideloadPaths, LAUNCHER, parse_sideload_library};
use crate::{
    data::{GamesResult, Launcher},
    linux::launchers::heroic::get_launch_command_for_heroic_source_flatpak,
};

#[derive(Debug)]
pub struct HeroicSideloadFlatpak {
    paths: HeroicSideloadPaths,
}

impl HeroicSideloadFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let paths = HeroicSideloadPaths::new_flatpak(path_home);
        Self { paths }
    }
}

impl Launcher for HeroicSideloadFlatpak {
    fn is_detected(&self) -> bool {
        self.paths.sideload_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = parse_sideload_library(&self.paths.sideload_library)?;

        Ok(parsed_data
            .into_iter()
            .map(|d| {
                d.into_game(
                    "sideload",
                    &self.paths.icons,
                    LAUNCHER.clone(),
                    get_launch_command_for_heroic_source_flatpak,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_heroic_sideload_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_heroic_epic_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicSideloadFlatpak::new(&path_file_system_mock);

        check_heroic_sideload_launcher(launcher, assert_cmd_flatpak)
    }
}
