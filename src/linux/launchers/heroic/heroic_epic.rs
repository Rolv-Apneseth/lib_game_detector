use std::{
    io::{self},
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::heroic::parse_all_games_from_library,
    utils::{get_launch_command, some_if_dir, some_if_file},
};

use super::ParsableLibraryData;

#[derive(Debug)]
pub struct HeroicEpic {
    path_legendary_library: PathBuf,
    path_icons: PathBuf,
}

impl HeroicEpic {
    pub fn new(path_heroic_config: &Path) -> Self {
        let path_install_info = path_heroic_config.join("store_cache/legendary_library.json");
        let path_icons = path_heroic_config.join("icons");

        debug!(
            "Heroic Launcher's legendary_library json file exists: {}",
            path_install_info.exists()
        );

        HeroicEpic {
            path_legendary_library: path_install_info,
            path_icons,
        }
    }

    /// Parse all relevant games' data from `legendary_library.json`
    #[tracing::instrument]
    fn parse_legendary_library(&self) -> Result<Vec<ParsableLibraryData>, io::Error> {
        trace!(
            "Parsing Heroic Launcher Legendary library file at {:?}",
            self.path_legendary_library
        );

        parse_all_games_from_library(&self.path_legendary_library).map(|data| {
            if data.is_empty() {
                warn!(
                    "No games were parsed from the Legendary library file at {:?}",
                    self.path_legendary_library
                )
            };

            data
        })
    }
}

impl Launcher for HeroicEpic {
    fn is_detected(&self) -> bool {
        self.path_legendary_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::HeroicGamesEpicGames
    }

    #[tracing::instrument(skip(self))]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.parse_legendary_library().map_err(|e| {
            error!("Error parsing the Heroic Games Legendary library file: {e}");
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

                let launch_command = get_launch_command(
                    "xdg-open",
                    Arc::new([&format!("heroic://launch/legendary/{app_id}")]),
                );

                let path_game_dir = some_if_dir(PathBuf::from(install_path));
                let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.jpg")));

                trace!("Heroic Launcher (Epic) - Game directory found for '{title}': {path_game_dir:?}");
                trace!("Heroic Launcher (Epic) - Box art found for '{title}': {path_box_art:?}");

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
    fn test_heroic_epic_launcher() -> Result<(), anyhow::Error> {
        let launcher = HeroicEpic::new(&get_mock_heroic_config_path());

        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;

        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, "Fall Guys");
        assert_eq!(games[1].title, "Rocket League");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_none());
        assert!(games[1].path_box_art.is_some());

        Ok(())
    }
}
