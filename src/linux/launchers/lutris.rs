use log::{debug, error, trace, warn};
use std::{
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{
    data::{Game, Launcher, SupportedLaunchers},
    utils::{parse_double_quoted_key_value, parse_unquoted_value, some_if_dir, some_if_file},
};

#[derive(Debug)]
pub struct GamePathsJsonData {
    // exe_path: PathBuf,
    executable_name: String,
    run_id: String,
}

#[derive(Debug)]
pub struct GameYmlData {
    executable_name: String,
    title: String,
    slug: String,
}

#[derive(Debug)]
pub struct CombinedGameData {
    // exe_path: PathBuf,
    run_id: String,
    title: String,
    slug: String,
}

pub struct Lutris {
    path_games_dir: PathBuf,
    path_box_art_dir: PathBuf,
    path_game_paths_json: PathBuf,
}

impl Lutris {
    pub fn new(path_config: &Path, path_cache: &Path) -> Self {
        let path_config_lutris = path_config.join("lutris");
        let path_cache_lutris = path_cache.join("lutris");

        let path_games_dir = path_config_lutris.join("games");
        let path_box_art_dir = path_cache_lutris.join("coverart");
        let path_game_paths_json = path_cache_lutris.join("game-paths.json");

        debug!(
            "Lutris games directory exists: {}\nLutris box art directory exists: {}\nLutris
        `game-paths.json` file exists: {}",
            path_games_dir.is_dir(),
            path_box_art_dir.is_dir(),
            path_game_paths_json.is_file()
        );

        Lutris {
            path_games_dir,
            path_box_art_dir,
            path_game_paths_json,
        }
    }

    /// Parse data from the Lutris `game-paths.json` file
    fn parse_game_paths_json(&self) -> Result<Vec<GamePathsJsonData>, ()> {
        let game_paths_json_file = File::open(&self.path_game_paths_json).map_err(|e| {
            error!(
                "Error with reading game-paths.json file at {:?}:\n{e}",
                self.path_game_paths_json
            )
        })?;

        let mut game_paths_data = BufReader::new(game_paths_json_file)
            .lines()
            .flatten()
            .filter_map(|line| {
                parse_double_quoted_key_value(&line)
                    .map(|(_, (run_id, exe_path))| {
                        let Some(parsed_executable_name) = exe_path.rsplit_once('/').map(|f| f.1) else
                        {
                            error!("Error extracting executable name from {:?}", exe_path);
                            return None;
                        };

                        Some(GamePathsJsonData {
                            executable_name: parsed_executable_name.to_owned(),
                            run_id: run_id.to_owned(),
                            // exe_path: PathBuf::from(exe_path),
                        })
                    })
                    .ok().and_then(|o| o)
            })
            .collect::<Vec<GamePathsJsonData>>();

        // Remove duplicate values as this file regularly has duplicates for some reason
        game_paths_data.sort_by(|a, b| a.run_id.cmp(&b.run_id));
        game_paths_data.dedup_by(|a, b| a.run_id == b.run_id);

        Ok(game_paths_data)
    }

    /// Parse data from the Lutris games directory, which contains 1 `.yml` file for each game
    fn parse_games_dir(&self) -> Result<Vec<GameYmlData>, ()> {
        Ok(read_dir(&self.path_games_dir)
            .map_err(|e| error!("Error with reading games directory for Lutris: {e:?}"))?
            .flatten()
            .filter_map(|path| {
                self.parse_game_yml(path.path())
                    .map_err(|e| {
                        error!(
                            "Error with parsing a lutris game_yml file at {:?}\n{e:?}",
                            path.path()
                        )
                    })
                    .ok()
                    .and_then(|o| o)
            })
            .collect::<Vec<GameYmlData>>())
    }

    /// Parse data from a given Lutris game's `.yml` file path
    fn parse_game_yml(&self, path_game_yml: PathBuf) -> Result<Option<GameYmlData>, ()> {
        let mut title = String::new();
        let mut slug = String::new();
        let mut executable_name = String::new();

        let game_yml_file = File::open(&path_game_yml).map_err(|e| {
            error!(
                "Error with reading game's `.yml` file at {:?}:\n{e}",
                path_game_yml
            )
        })?;

        for line in BufReader::new(game_yml_file).lines().flatten().skip(1) {
            if !title.is_empty() && !slug.is_empty() && !executable_name.is_empty() {
                break;
            };

            if let Ok((_, exe_path)) = parse_unquoted_value(&line, "exe") {
                executable_name = exe_path
                    .rsplit_once('/')
                    .map(|t| t.1)
                    .ok_or_else(|| error!("Error parsing exe line in game yml file. Line: {line}"))?
                    .to_owned();
                continue;
            }

            if let Ok((_, parsed_slug)) = parse_unquoted_value(&line, "game_slug") {
                slug = parsed_slug;
                continue;
            }

            if let Ok((_, parsed_title)) = parse_unquoted_value(&line, "name") {
                title = parsed_title;
                continue;
            }
        }

        // Guess slug from game's .yml file name if it wasn't found in the file
        if slug.is_empty() {
            if let Some(slug_from_filename) = path_game_yml.file_name().and_then(|s| {
                s.to_string_lossy()
                    .rsplit_once('-')
                    .map(|f| f.0.to_string())
            }) {
                slug = slug_from_filename;
            }
        };

        // Guess title from slug if it wasn't found in the file
        if title.is_empty() {
            let mut title_from_slug = slug.split('-').collect::<Vec<&str>>().join(" ");

            if let Some(first_char) = title_from_slug.get_mut(0..1) {
                first_char.make_ascii_uppercase();
            };

            title = title_from_slug;
        }

        if executable_name.is_empty() || slug.is_empty() || title.is_empty() {
            debug!(
                "Could not find relevant data fields for Lutris game from file:
{path_game_yml:?}"
            );

            Ok(None)
        } else {
            Ok(Some(GameYmlData {
                title,
                slug,
                executable_name,
            }))
        }
    }

    /// Get all relevant game data by combining data from the `game-paths.json` file and
    /// each games `.yml` file.
    /// Matching of the data from these sources is done using the executable path of the
    /// game, which is the only thing defined in both sources
    pub fn parse_game_data(&self) -> Result<Vec<CombinedGameData>, ()> {
        let mut combined_data = Vec::new();

        let game_paths_data = self.parse_game_paths_json()?;
        let game_yml_data = self.parse_games_dir()?;

        for path_data in game_paths_data {
            if let Some(combined_datum) = game_yml_data
                .iter()
                .find(|g| g.executable_name == path_data.executable_name)
                .map(|yml_data| CombinedGameData {
                    // exe_path: path_data.exe_path,
                    run_id: path_data.run_id,
                    title: yml_data.title.clone(),
                    slug: yml_data.slug.clone(),
                })
            {
                combined_data.push(combined_datum);
            }
        }

        Ok(combined_data)
    }
}

impl Launcher for Lutris {
    fn is_detected(&self) -> bool {
        self.path_game_paths_json.exists()
            && self.path_games_dir.is_dir()
            && self.path_box_art_dir.is_dir()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::Lutris
    }

    fn get_detected_games(&self) -> Result<Vec<Game>, ()> {
        let mut games = Vec::new();

        let game_data = self.parse_game_data()?;

        for CombinedGameData {
            // exe_path,
            run_id,
            title,
            slug,
        } in game_data
        {
            let launch_command = format!("env LUTRIS_SKIP_INIT=1 lutris lutris:rungameid/{run_id}");

            let path_box_art = some_if_file(self.path_box_art_dir.join(format!("{}.jpg", slug)));
            trace!("Lutris - Box art found for '{title}': {path_box_art:?}");

            games.push(Game {
                title,
                launch_command,
                path_box_art,
                path_game_dir: None,
            })
        }

        if games.is_empty() {
            warn!("No games (at least not with sufficient data) found for Lutris launcher");
        }

        Ok(games)
    }
}
