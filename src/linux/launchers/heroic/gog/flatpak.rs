use std::path::Path;

use tracing::warn;

use super::{HeroicGogPaths, LAUNCHER, parse_gog_installed};
use crate::{
    data::{GamesResult, Launcher},
    linux::launchers::heroic::get_launch_command_for_heroic_source_flatpak,
};

#[derive(Debug)]
pub struct HeroicGOGFlatpak {
    paths: HeroicGogPaths,
}

impl HeroicGOGFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let paths = HeroicGogPaths::new_flatpak(path_home);
        HeroicGOGFlatpak { paths }
    }
}

impl Launcher for HeroicGOGFlatpak {
    fn is_detected(&self) -> bool {
        self.paths.gog_installed_games.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = parse_gog_installed(&self.paths.gog_installed_games)?;

        Ok(parsed_data
            .into_iter()
            .map(|d| {
                d.into_game(
                    &self.paths.icons,
                    get_launch_command_for_heroic_source_flatpak,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_heroic_gog_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_heroic_gog_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicGOGFlatpak::new(&path_file_system_mock);

        check_heroic_gog_launcher(launcher, assert_cmd_flatpak)
    }
}
