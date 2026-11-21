use std::path::Path;

use super::{HeroicAmazonPaths, LAUNCHER, parse_nile_library};
use crate::{
    data::{GamesResult, Launcher},
    linux::launchers::heroic::get_launch_command_for_heroic_source,
};

#[derive(Debug)]
pub struct HeroicAmazon {
    paths: HeroicAmazonPaths,
}

impl HeroicAmazon {
    pub fn new(path_config: &Path) -> Self {
        let paths = HeroicAmazonPaths::new_standard(path_config);
        HeroicAmazon { paths }
    }
}

impl Launcher for HeroicAmazon {
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
                    get_launch_command_for_heroic_source,
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
        linux::test_utils::{assert_cmd_xdg, get_mock_file_system_path},
    };

    #[test]
    fn test_heroic_amazon_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicAmazon::new(&path_file_system_mock.join(".config"));

        check_heroic_amazon_launcher(launcher, assert_cmd_xdg)
    }
}
