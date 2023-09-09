use log::{debug, trace, warn};
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
struct ParsableGOGInstalledData {
    app_id: String,
    install_path: String,
    title: String,
}

pub struct HeroicGOG {
    path_gog_installed_games: PathBuf,
    path_icons: PathBuf,
}

impl HeroicGOG {
    pub fn new(path_heroic_config: &Path) -> Self {
        // store_cache/gog_install_info.json does not include game install paths
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

    /// Returns a vec containing Game structs corresponding to the installed GOG games
    fn parse_gog_installed(&self) -> Result<Vec<ParsableGOGInstalledData>, io::Error> {
        let mut parsed_data = Vec::new();

        let mut curr_install_path = String::new();
        let mut curr_title = String::new();

        trace!(
            "Parsing heroic library file for installed GOG games at {:?}",
            self.path_gog_installed_games
        );

        for line in BufReader::new(File::open(&self.path_gog_installed_games)?)
            .lines()
            .flatten()
        {
            if let Ok((_, install_path)) = parse_double_quoted_value(&line, "install_path") {
                let Some(title) = install_path
                        .rsplit_once('/')
                        .map(|split_path| clean_game_title(split_path.1) ) else {
                    continue;
                };

                curr_install_path = install_path;
                curr_title = title;
            };

            if let Ok((_, app_id)) = parse_double_quoted_value(&line, "appName") {
                parsed_data.push(ParsableGOGInstalledData {
                    app_id,
                    title: curr_title.clone(),
                    install_path: curr_install_path.clone(),
                });

                continue;
            };
        }

        if parsed_data.is_empty() {
            warn!(
                "No games were parsed from the gog_store/installed.json file at {:?}",
                self.path_gog_installed_games
            )
        }

        Ok(parsed_data)
    }
}

impl Launcher for HeroicGOG {
    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::HeroicGOG
    }

    fn is_detected(&self) -> bool {
        self.path_gog_installed_games.exists()
    }

    fn get_detected_games(&self) -> Result<Vec<Game>, ()> {
        let mut games = Vec::new();

        for parsed_data in self.parse_gog_installed().map_err(|_| ())? {
            let ParsableGOGInstalledData {
                app_id,
                install_path,
                title,
            } = parsed_data;

            let launch_command = format!("xdg-open heroic://launch/gog/{app_id}");

            let path_game_dir = some_if_dir(PathBuf::from(install_path));
            let path_box_art = some_if_file(self.path_icons.join(format!("{app_id}.png")));

            trace!("Heroic Launcher (GOG) - Game directory found for '{title}': {path_game_dir:?}");
            trace!("Heroic Launcher (GOG) - Box art found for '{title}': {path_box_art:?}");

            games.push(Game {
                path_game_dir,
                title: title.to_owned(),
                launch_command,
                path_box_art,
            });
        }

        if games.is_empty() {
            warn!(
                "No games found for Heroic launcher GOG library at {:?}",
                self.path_gog_installed_games
            );
        }

        Ok(games)
    }
}
