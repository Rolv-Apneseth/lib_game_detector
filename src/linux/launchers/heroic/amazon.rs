use std::{
    io::{self},
    path::{Path, PathBuf},
};

use tracing::{error, trace, warn};

use super::ParsableLibraryData;
use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::heroic::{
        get_heroic_config_path, get_launch_command_for_heroic_source,
        parse_all_games_from_library_common,
    },
    macros::logs::{debug_path, warn_no_games},
    utils::{some_if_dir, some_if_file},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::HeroicGamesAmazon;

#[derive(Debug)]
pub struct HeroicAmazon {
    path_nile_library: PathBuf,
    path_icons: PathBuf,
    is_using_flatpak: bool,
}

impl HeroicAmazon {
    pub fn new(path_home: &Path, path_config: &Path) -> Self {
        let (path_heroic_config, is_using_flatpak) = get_heroic_config_path(path_home, path_config);

        let path_nile_library = path_heroic_config.join("store_cache/nile_library.json");
        let path_icons = path_heroic_config.join("icons");

        debug_path!("Nile library JSON file", path_nile_library);

        HeroicAmazon {
            path_nile_library,
            path_icons,
            is_using_flatpak,
        }
    }

    /// Parse all relevant games' data from `nile_library.json`
    #[tracing::instrument]
    fn parse_nile_library(&self) -> Result<Vec<ParsableLibraryData>, io::Error> {
        trace!(
            "{LAUNCHER} - Parsing Nile library file at {:?}",
            self.path_nile_library
        );

        parse_all_games_from_library_common(&self.path_nile_library).inspect(|data| {
            if data.is_empty() {
                warn!(
                    "{LAUNCHER} - No games were parsed from the Nile library file at {:?}",
                    self.path_nile_library
                )
            };
        })
    }
}

impl Launcher for HeroicAmazon {
    fn is_detected(&self) -> bool {
        self.path_nile_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self
            .parse_nile_library()
            .inspect_err(|e| error!("{LAUNCHER} - Error parsing the Nile library file: {e}"))?;

        if parsed_data.is_empty() {
            warn_no_games!();
        };

        Ok(parsed_data
            .into_iter()
            .map(|parsed_data| {
                let ParsableLibraryData {
                    app_id,
                    install_path,
                    title,
                } = parsed_data;

                let launch_command =
                    get_launch_command_for_heroic_source("nile", &app_id, self.is_using_flatpak);
                trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

                let path_game_dir = some_if_dir(PathBuf::from(install_path));
                let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.jpg")));

                trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
                trace!("{LAUNCHER} - Box art for '{title}': {path_box_art:?}");

                Game {
                    title,
                    launch_command,
                    path_box_art,
                    path_game_dir,
                    path_icon: None,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case(false, ".config"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_heroic_amazon_launcher(
        is_testing_flatpak: bool,
        path_config: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicAmazon::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_config),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;

        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, "Metal Slug");
        assert_eq!(games[1].title, "Metal Slug X");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_none());
        assert!(games[1].path_box_art.is_some());

        assert!(games.iter().all(|g| g.path_icon.is_none()));

        Ok(())
    }
}
