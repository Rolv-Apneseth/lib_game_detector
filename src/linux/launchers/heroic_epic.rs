use log::{debug, error, trace, warn};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{
    data::{Game, Launcher, SupportedLaunchers},
    utils::{clean_game_title, parse_double_quoted_value, some_if_dir, some_if_file},
};

#[derive(Debug)]
struct ParsableInstallInfoData {
    app_id: String,
    install_path: String,
    title: String,
}

pub struct HeroicEpic {
    path_install_info: PathBuf,
    path_icons: PathBuf,
}

impl HeroicEpic {
    pub fn new(path_heroic_config: &Path) -> Self {
        let path_install_info = path_heroic_config.join("store_cache/legendary_install_info.json");
        let path_icons = path_heroic_config.join("icons");

        debug!(
            "Heroic Launcher's legendary_library json file exists: {}",
            path_install_info.exists()
        );

        HeroicEpic {
            path_install_info,
            path_icons,
        }
    }

    /// Returns a vec containing Game structs corresponding to the installed Epic/Legendary games
    fn parse_install_info(&self) -> Result<Vec<ParsableInstallInfoData>, io::Error> {
        let mut parsed_data = Vec::new();

        let mut curr_app_id = String::new();
        let mut curr_title = String::new();

        trace!(
            "Parsing heroic launcher legendary_install_info json file at {:?}",
            self.path_install_info
        );

        for line in BufReader::new(File::open(&self.path_install_info)?)
            .lines()
            .flatten()
        {
            if let Ok((_, app_id)) = parse_double_quoted_value(&line, "app_name") {
                curr_app_id = app_id;
                continue;
            };

            if let Ok((_, title)) = parse_double_quoted_value(&line, "title") {
                curr_title = title;
                continue;
            };

            if let Ok((_, install_path)) = parse_double_quoted_value(&line, "install_path") {
                parsed_data.push(ParsableInstallInfoData {
                    app_id: curr_app_id.clone(),
                    title: clean_game_title(&curr_title),
                    install_path,
                });
                continue;
            };
        }

        if parsed_data.is_empty() {
            warn!(
                "No games were parsed from the legendary_install_info.json file at {:?}",
                self.path_install_info
            )
        }

        Ok(parsed_data)
    }
}

impl Launcher for HeroicEpic {
    fn is_detected(&self) -> bool {
        self.path_install_info.exists()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::HeroicGamesEpicGames
    }

    fn get_detected_games(&self) -> Result<Vec<Game>, ()> {
        let mut games = Vec::new();

        for parsed_data in self.parse_install_info().map_err(|e| {
            error!("Error parsing the Heroic Games legendary_install_info.json file: {e}");
        })? {
            let ParsableInstallInfoData {
                app_id,
                install_path,
                title,
            } = parsed_data;

            let launch_command = format!("xdg-open heroic://launch/legendary/{app_id}");

            let path_game_dir = some_if_dir(PathBuf::from(install_path));
            let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.jpg")));

            trace!(
                "Heroic Launcher (Heroic) - Game directory found for '{title}': {path_game_dir:?}"
            );
            trace!("Heroic Launcher (Heroic) - Box art found for '{title}': {path_box_art:?}");

            games.push(Game {
                title,
                launch_command,
                path_box_art,
                path_game_dir,
            });
        }

        if games.is_empty() {
            warn!(
                "No games found for Heroic launcher Epic Games install_info file at: {:?}",
                self.path_install_info
            )
        }

        Ok(games)
    }
}
