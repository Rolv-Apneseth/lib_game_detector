pub use flatpak::HeroicSideloadFlatpak;
pub use standard::HeroicSideload;

mod flatpak;
mod standard;

use std::{
    io,
    path::{Path, PathBuf},
};

use nom::IResult;
use tracing::{error, trace, warn};

use super::ParsableLibraryData;
use crate::{
    data::SupportedLaunchers,
    linux::launchers::heroic::{
        get_heroic_config_path, get_heroic_config_path_flatpak, parse_all_games_from_library,
    },
    macros::logs::{debug_path, warn_no_games},
    parsers::{parse_value_json, parse_value_json_unquoted},
    utils::clean_game_title,
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
pub struct HeroicSideloadPaths {
    sideload_library: PathBuf,
    icons: PathBuf,
}

impl HeroicSideloadPaths {
    fn _new(path_heroic_config: &Path) -> Self {
        let sideload_library = path_heroic_config.join("sideload_apps/library.json");
        let icons = path_heroic_config.join("icons");

        debug_path!("Sideload library JSON file", sideload_library);
        debug_path!("Icons dir", icons);

        Self {
            sideload_library,
            icons,
        }
    }

    fn new_standard(path_config: &Path) -> Self {
        let path_heroic_config = get_heroic_config_path(path_config);
        Self::_new(&path_heroic_config)
    }

    fn new_flatpak(path_home: &Path) -> Self {
        let path_heroic_config = get_heroic_config_path_flatpak(path_home);
        Self::_new(&path_heroic_config)
    }
}

/// Parse all relevant games' data from `library.json`
#[tracing::instrument(level = "trace")]
fn parse_sideload_library(
    path_sideload_library: &Path,
) -> Result<Vec<ParsableLibraryData>, io::Error> {
    trace!(
        "{LAUNCHER} - Parsing sideload library file at {}",
        path_sideload_library.display()
    );

    parse_all_games_from_library(path_sideload_library, parse_game_from_sideload_library)
        .inspect(|data| {
            if data.is_empty() {
                warn_no_games!();
            };
        })
        .inspect_err(|_| {
            error!(
                "Error parsing the Heroic Games Legendary library file at: {}",
                path_sideload_library.display()
            );
        })
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
    };

    pub fn check_heroic_sideload_launcher<F>(
        launcher: impl Launcher,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;
        assert_eq!(games.len(), 2);

        assert_cmd(&games[0]);

        assert_eq!(games[0].title, "Resistance - Fall of Man");
        assert_eq!(games[1].title, "Little Big Planet 3");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_some());
        assert!(games[1].path_box_art.is_some());

        Ok(())
    }
}
