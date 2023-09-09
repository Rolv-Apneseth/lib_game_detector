use log::{debug, error, trace, warn};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{
    data::{Game, Launcher, SupportedLaunchers},
    utils::{
        clean_game_title, parse_double_quoted_value, parse_unquoted_json_value, some_if_dir,
        some_if_file,
    },
};

#[derive(Debug)]
struct ParsableLibraryData {
    app_id: String,
    install_path: String,
    title: String,
}

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

    /// Returns a vec containing Game structs corresponding to the installed Epic/Legendary games
    fn parse_nile_library(&self) -> Result<Vec<ParsableLibraryData>, io::Error> {
        let mut parsed_data = Vec::new();

        let mut curr_app_id = String::new();
        let mut curr_install_path = String::new();
        let mut curr_is_installed = false;

        trace!(
            "Parsing heroic launcher nile library json file at {:?}",
            self.path_nile_library
        );

        for line in BufReader::new(File::open(&self.path_nile_library)?)
            .lines()
            .flatten()
        {
            if let Ok((_, app_id)) = parse_double_quoted_value(&line, "app_name") {
                curr_app_id = app_id;
                continue;
            };

            if let Ok((_, install_path)) = parse_double_quoted_value(&line, "install_path") {
                curr_install_path = install_path;
                continue;
            };

            if let Ok((_, is_installed)) = parse_unquoted_json_value(&line, "is_installed") {
                curr_is_installed = is_installed == *"true";
                continue;
            }

            if let Ok((_, title)) = parse_double_quoted_value(&line, "title") {
                if !curr_is_installed {
                    continue;
                }

                parsed_data.push(ParsableLibraryData {
                    app_id: curr_app_id.clone(),
                    title: clean_game_title(&title),
                    install_path: curr_install_path.clone(),
                });
            };
        }

        if parsed_data.is_empty() {
            warn!(
                "No games were parsed from the nile_library.json file at {:?}",
                self.path_nile_library
            )
        }

        Ok(parsed_data)
    }
}

impl Launcher for HeroicAmazon {
    fn is_detected(&self) -> bool {
        self.path_nile_library.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::HeroicGamesAmazon
    }

    fn get_detected_games(&self) -> Result<Vec<Game>, ()> {
        let mut games = Vec::new();

        for parsed_data in self.parse_nile_library().map_err(|e| {
            error!("Error parsing the Heroic Games legendary_install_info.json file: {e}");
        })? {
            let ParsableLibraryData {
                app_id,
                install_path,
                title,
            } = parsed_data;

            let launch_command = format!("xdg-open heroic://launch/nile/{app_id}");

            let path_game_dir = some_if_dir(PathBuf::from(install_path));
            let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.jpg")));

            trace!(
                "Heroic Launcher (Amazon) - Game directory found for '{title}': {path_game_dir:?}"
            );
            trace!("Heroic Launcher (Amazon) - Box art found for '{title}': {path_box_art:?}");

            games.push(Game {
                title,
                launch_command,
                path_box_art,
                path_game_dir,
            });
        }

        if games.is_empty() {
            warn!(
                "No games found for Heroic launcher Amazon Prime Gaming library file at: {:?}",
                self.path_nile_library
            )
        }

        Ok(games)
    }
}
