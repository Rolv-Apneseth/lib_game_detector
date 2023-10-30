use nom::IResult;
use std::{
    fs::read_to_string,
    io::{self},
    path::{Path, PathBuf},
};
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    parsers::{parse_double_quoted_value, parse_until_key},
    utils::{clean_game_title, some_if_dir, some_if_file},
};

#[derive(Debug)]
struct ParsableGOGInstalledData {
    app_id: String,
    install_path: String,
    title: String,
}

/// Utility function which parses a single game from the Heroic Games GOG store `installed.json` file
///
/// Unfortunately a separate parser function is needed for GOG's `gog_store/installed.json` file because:
/// 1. `store_cache/gog_library.json` has `is_installed` as always false
/// 2. `gog_store/library.json` is empty for some reason
#[tracing::instrument(skip_all)]
fn parse_game_from_gog_installed(file_content: &str) -> IResult<&str, ParsableGOGInstalledData> {
    // INSTALL_PATH
    let key_path = "install_path";
    let (file_content, _) = parse_until_key(file_content, key_path)?;
    let (file_content, install_path) = parse_double_quoted_value(file_content, key_path)?;

    // ID
    let key_id = "appName";
    let (file_content, _) = parse_until_key(file_content, key_id)?;
    let (file_content, app_id) = parse_double_quoted_value(file_content, key_id)?;

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
            title: clean_game_title(&title),
            install_path,
        },
    ))
}

#[derive(Debug)]
pub struct HeroicGOG {
    path_gog_installed_games: PathBuf,
    path_icons: PathBuf,
}

impl HeroicGOG {
    pub fn new(path_heroic_config: &Path) -> Self {
        let path_gog_installed_games = path_heroic_config.join("gog_store/installed.json");
        let path_icons = path_heroic_config.join("icons");

        debug!(
            "Heroic GOG installed games json file exists: {}",
            path_gog_installed_games.exists()
        );

        HeroicGOG {
            path_gog_installed_games,
            path_icons,
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
        SupportedLaunchers::HeroicGOG
    }

    #[tracing::instrument(skip(self))]
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

                let launch_command = format!("xdg-open heroic://launch/gog/{app_id}");

                let path_game_dir = some_if_dir(PathBuf::from(install_path));
                let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.png")));

                trace!(
                    "Heroic Launcher (GOG) - Game directory found for '{title}': {path_game_dir:?}"
                );
                trace!("Heroic Launcher (GOG) - Box art found for '{title}': {path_box_art:?}");

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
