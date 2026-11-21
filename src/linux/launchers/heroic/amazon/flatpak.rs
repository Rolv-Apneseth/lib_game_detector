use std::path::Path;

use super::{HeroicAmazonPaths, LAUNCHER, parse_nile_library};
use crate::{
    data::{GamesResult, Launcher},
    linux::launchers::heroic::get_launch_command_for_heroic_source_flatpak,
};

#[derive(Debug)]
pub struct HeroicAmazonFlatpak {
    paths: HeroicAmazonPaths,
}

impl HeroicAmazonFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let paths = HeroicAmazonPaths::new_flatpak(path_home);
        HeroicAmazonFlatpak { paths }
    }
}

impl Launcher for HeroicAmazonFlatpak {
    fn is_detected(&self) -> bool {
        self.paths.nile_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = parse_nile_library(&self.paths.nile_library)?;

        Ok(parsed_data
            .into_iter()
            .map(|d| {
                d.into_game(
                    "nile",
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
    use super::{super::test_utils::check_heroic_amazon_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_heroic_amazon_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicAmazonFlatpak::new(&path_file_system_mock);

        check_heroic_amazon_launcher(launcher, assert_cmd_flatpak)
    }
}
