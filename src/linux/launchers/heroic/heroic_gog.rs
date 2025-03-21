use std::{
    fs::read_to_string,
    io::{self},
    path::{Path, PathBuf},
};

use nom::IResult;
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::heroic::{get_heroic_config_path, get_launch_command_for_heroic_source},
    parsers::parse_value_json,
    utils::{clean_game_title, some_if_dir, some_if_file},
};

#[derive(Debug)]
struct ParsableGOGInstalledData {
    app_id: String,
    install_path: String,
    title: String,
}

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::HeroicGamesGOG;

/// Utility function which parses a single game from the Heroic Games GOG store `installed.json` file
///
/// Unfortunately a separate parser function is needed for GOG's `gog_store/installed.json` file because:
/// 1. `store_cache/gog_library.json` has `is_installed` as always false
/// 2. `gog_store/library.json` is empty for some reason
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_gog_installed(file_content: &str) -> IResult<&str, ParsableGOGInstalledData> {
    // INSTALL_PATH
    let (file_content, install_path) = parse_value_json(file_content, "install_path")?;

    // ID
    let (file_content, app_id) = parse_value_json(file_content, "appName")?;

    // TITLE
    let Some(title) = install_path
        .rsplit_once('/')
        .map(|split_path| clean_game_title(split_path.1))
    else {
        return parse_game_from_gog_installed(file_content);
    };

    Ok((
        file_content,
        ParsableGOGInstalledData {
            app_id,
            title: clean_game_title(title),
            install_path,
        },
    ))
}

#[derive(Debug)]
pub struct HeroicGOG {
    path_gog_installed_games: PathBuf,
    path_icons: PathBuf,
    is_using_flatpak: bool,
}

impl HeroicGOG {
    pub fn new(path_home: &Path, path_config: &Path) -> Self {
        let (path_heroic_config, is_using_flatpak) = get_heroic_config_path(path_home, path_config);
        let path_gog_installed_games = path_heroic_config.join("gog_store/installed.json");
        let path_icons = path_heroic_config.join("icons");

        debug!(
            "{LAUNCHER} - installed games json file exists: {}",
            path_gog_installed_games.exists()
        );

        HeroicGOG {
            path_gog_installed_games,
            path_icons,
            is_using_flatpak,
        }
    }

    /// Parse all relevant games' data from GOG's `installed.json`
    #[tracing::instrument]
    fn parse_gog_installed(&self) -> Result<Vec<ParsableGOGInstalledData>, io::Error> {
        trace!(
            "Parsing Heroic Launcher GOG installed games file at {:?}",
            self.path_gog_installed_games
        );

        let mut parsed_data = Vec::new();

        let file_content = read_to_string(&self.path_gog_installed_games)?;
        let mut file_content_str: &str = &file_content;

        // Parse individual games from GOG installed file until no more are found
        loop {
            let Ok((new_file_content, parsed_game_data)) =
                parse_game_from_gog_installed(file_content_str)
            else {
                break;
            };

            file_content_str = new_file_content;
            parsed_data.push(parsed_game_data);
        }

        if parsed_data.is_empty() {
            warn!(
                "No games were parsed from the GOG installed games file at {:?}",
                self.path_gog_installed_games
            )
        };

        Ok(parsed_data)
    }
}

impl Launcher for HeroicGOG {
    fn is_detected(&self) -> bool {
        self.path_gog_installed_games.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.parse_gog_installed().map_err(|e| {
            error!("Error parsing the Heroic Games Legendary library file: {e}");
            e
        })?;

        Ok(parsed_data
            .into_iter()
            .map(|parsed_data| {
                let ParsableGOGInstalledData {
                    app_id,
                    install_path,
                    title,
                } = parsed_data;

                let launch_command =
                    get_launch_command_for_heroic_source("gog", &app_id, self.is_using_flatpak);
                trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

                let path_game_dir = some_if_dir(PathBuf::from(install_path));
                let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.png")));

                trace!("{LAUNCHER} - Game directory found for '{title}': {path_game_dir:?}");
                trace!("{LAUNCHER} - Box art found for '{title}': {path_box_art:?}");

                Game {
                    title,
                    launch_command,
                    path_box_art,
                    path_game_dir,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::linux::test_utils::get_mock_file_system_path;

    #[test_case(false, ".config"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_heroic_gog_launcher(
        is_testing_flatpak: bool,
        path_config: &str,
    ) -> Result<(), anyhow::Error> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicGOG::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_config),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;

        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, "home");
        assert_eq!(games[1].title, "Bread & Fred Demo");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_none());
        assert!(games[1].path_box_art.is_some());

        Ok(())
    }
}
