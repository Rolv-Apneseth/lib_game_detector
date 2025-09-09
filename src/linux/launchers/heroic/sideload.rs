use std::{
    io,
    path::{Path, PathBuf},
};

use nom::IResult;
use tracing::{error, trace, warn};

use super::ParsableLibraryData;
use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::heroic::{
        get_heroic_config_path, get_launch_command_for_heroic_source, parse_all_games_from_library,
    },
    macros::logs::{debug_path, warn_no_games},
    parsers::{parse_value_json, parse_value_json_unquoted},
    utils::{clean_game_title, some_if_dir, some_if_file},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::HeroicGamesSideload;

/// Utility function which parses a single game from the Heroic Games side-load apps `library.json` file
///
/// A separate parser function is necessary for this library file because the required fields are
/// listed in a different order to the Nile and Legendary library files.
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_sideload_library(file_content: &str) -> IResult<&str, ParsableLibraryData> {
    // ID
    let (file_content, app_id) = parse_value_json(file_content, "app_name")?;

    // Keep checkpoint of file content because `is_installed` comes after the `install_path`
    // and `title` may come before install info
    let file_content_checkpoint = file_content;

    // IS_INSTALLED
    let (_, is_installed) = parse_value_json_unquoted(file_content, "is_installed")?;

    // Continue to next game if not installed
    if is_installed == *"false" {
        return parse_game_from_sideload_library(file_content);
    }

    // TITLE
    let (file_content, title) = parse_value_json(file_content_checkpoint, "title")?;

    // INSTALL_PATH
    let (file_content, install_path) = parse_value_json(file_content, "folder_name")?;

    Ok((
        file_content,
        ParsableLibraryData {
            app_id,
            title: clean_game_title(title),
            install_path,
        },
    ))
}

#[derive(Debug)]
pub struct HeroicSideload {
    path_sideload_library: PathBuf,
    path_icons: PathBuf,
    is_using_flatpak: bool,
}

impl HeroicSideload {
    pub fn new(path_home: &Path, path_config: &Path) -> Self {
        let (path_heroic_config, is_using_flatpak) = get_heroic_config_path(path_home, path_config);

        let path_sideload_library = path_heroic_config.join("sideload_apps/library.json");
        let path_icons = path_heroic_config.join("icons");

        debug_path!("sideloaded apps library JSON file", path_sideload_library);

        Self {
            path_sideload_library,
            path_icons,
            is_using_flatpak,
        }
    }

    /// Parse all relevant games' data from `library.json`
    #[tracing::instrument(level = "trace")]
    fn parse_sideload_library(&self) -> Result<Vec<ParsableLibraryData>, io::Error> {
        trace!(
            "{LAUNCHER} - Parsing sideload library file at {:?}",
            self.path_sideload_library
        );

        parse_all_games_from_library(
            &self.path_sideload_library,
            parse_game_from_sideload_library,
        )
        .inspect(|data| {
            if data.is_empty() {
                warn!(
                    "{LAUNCHER} - No games were parsed from the Legendary library file at {:?}",
                    self.path_sideload_library
                )
            };
        })
    }
}

impl Launcher for HeroicSideload {
    fn is_detected(&self) -> bool {
        self.path_sideload_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.parse_sideload_library().map_err(|e| {
            error!("Error parsing the Heroic Games Legendary library file: {e}");
            e
        })?;

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

                let launch_command = get_launch_command_for_heroic_source(
                    "sideload",
                    &app_id,
                    self.is_using_flatpak,
                );
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
    fn test_heroic_epic_launcher(
        is_testing_flatpak: bool,
        path_config: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = HeroicSideload::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_config),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;

        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, "Resistance - Fall of Man");
        assert_eq!(games[1].title, "Little Big Planet 3");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_some());
        assert!(games[1].path_box_art.is_some());

        Ok(())
    }
}
