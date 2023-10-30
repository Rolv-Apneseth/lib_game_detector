use std::{
    io::{self},
    path::{Path, PathBuf},
};
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::heroic::parse_all_games_from_library,
    utils::{some_if_dir, some_if_file},
};

use super::ParsableLibraryData;

#[derive(Debug)]
pub struct HeroicAmazon {
    path_nile_library: PathBuf,
    path_icons: PathBuf,
}

impl HeroicAmazon {
    pub fn new(path_heroic_config: &Path) -> Self {
        let path_nile_library = path_heroic_config.join("store_cache/nile_library.json");
        let path_icons = path_heroic_config.join("icons");

        debug!(
            "Heroic Launcher's nile_library json file exists: {}",
            path_nile_library.exists()
        );

        HeroicAmazon {
            path_nile_library,
            path_icons,
        }
    }

    /// Parse all relevant games' data from `nile_library.json`
    #[tracing::instrument]
    fn parse_nile_library(&self) -> Result<Vec<ParsableLibraryData>, io::Error> {
        trace!(
            "Parsing Heroic Launcher Nile library file at {:?}",
            self.path_nile_library
        );

        parse_all_games_from_library(&self.path_nile_library).map(|data| {
            if data.is_empty() {
                warn!(
                    "No games were parsed from the Nile library file at {:?}",
                    self.path_nile_library
                )
            };

            data
        })
    }
}

impl Launcher for HeroicAmazon {
    fn is_detected(&self) -> bool {
        self.path_nile_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::HeroicGamesAmazon
    }

    #[tracing::instrument(skip(self))]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.parse_nile_library().map_err(|e| {
            error!("Error parsing the Heroic Games Nile library file: {e}");
            e
        })?;

        Ok(parsed_data
            .into_iter()
            .map(|parsed_data| {
                let ParsableLibraryData {
                app_id,
                install_path,
                title,
            } = parsed_data;

                let launch_command = format!("xdg-open heroic://launch/nile/{app_id}");

                let path_game_dir = some_if_dir(PathBuf::from(install_path));
                let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.jpg")));

                trace!("Heroic Launcher (Amazon) - Game directory found for '{title}': {path_game_dir:?}");
                trace!("Heroic Launcher (Amazon) - Box art found for '{title}': {path_box_art:?}");

                Game {
                    title,
                    launch_command,
                    path_box_art,
                    path_game_dir,
                }
            })
            .collect()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::linux::test_utils::get_mock_heroic_config_path;

    use super::*;

    #[test]
    fn test_heroic_amazon_launcher() -> Result<(), anyhow::Error> {
        let launcher = HeroicAmazon::new(&get_mock_heroic_config_path());

        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;

        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, "Metal Slug");
        assert_eq!(games[1].title, "Metal Slug X");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_none());
        assert!(games[1].path_box_art.is_some());

        Ok(())
    }
}
