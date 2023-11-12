use itertools::Itertools;
use nom::{bytes::complete::take_until, IResult};
use std::{
    fs::{read_dir, read_to_string, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    parsers::{parse_double_quoted_key_value, parse_unquoted_value, parse_until_key_unquoted},
    utils::{clean_game_title, some_if_dir, some_if_file},
};

#[derive(Debug, Clone)]
pub struct ParsableGamePathsData {
    game_dir: String,
    executable_name: String,
    run_id: String,
}

#[derive(Debug, Clone)]
pub struct ParsableGameYmlData {
    executable_name: String,
    title: String,
    slug: String,
}

#[derive(Debug)]
pub struct ParsableDataCombined {
    game_dir: String,
    run_id: String,
    title: String,
    slug: String,
}

impl ParsableDataCombined {
    fn combine(paths_data: ParsableGamePathsData, yml_data: ParsableGameYmlData) -> Self {
        ParsableDataCombined {
            game_dir: paths_data.game_dir,
            run_id: paths_data.run_id,
            title: yml_data.title,
            slug: yml_data.slug,
        }
    }
}

// UTILS --------------------------------------------------------------------------------
/// Used for parsing relevant game's data from the given game `.yml` file's contents
// A bit complicated due to edge cases where only the executable name is defined in the file
#[tracing::instrument(skip(file_content))]
fn parse_game_yml<'a>(
    file_content: &'a str,
    file_path: &Path,
) -> IResult<&'a str, Result<ParsableGameYmlData, ()>> {
    // EXECUTABLE_NAME
    let key_exe = "exe";
    let (file_content, _) = parse_until_key_unquoted(file_content, key_exe)?;
    // let (mut file_content, exe_path) = parse_unquoted_value(file_content, key_exe)?;
    let (mut file_content, line) = take_until("\n")(file_content)?;
    let executable_name = match line.rsplit_once('/').map(|t| t.1.to_owned()) {
        Some(e) => e,
        None => {
            // TODO: Handle this better somehow. Can't figure out how to return a nom error.
            error!("Error parsing '{key_exe}' line in game yml file at {file_path:?}");
            return Ok((file_content, Err(())));
        }
    };

    // SLUG
    let key_slug = "game_slug";
    let slug: String;

    match parse_until_key_unquoted(file_content, key_slug) {
        // Use value parsed from file for the slug, if one is found
        Ok((f, _)) => {
            (file_content, slug) = parse_unquoted_value(f, key_slug)?;
        }
        // Otherwise attempt to read the slug from the file's name (usually in the form
        // `{slug}-{number}.yml`)
        Err(e) => {
            let Some(slug_from_filename) = file_path
                .file_name()
                .and_then(|s| s.to_string_lossy().rsplit_once('-').map(|f| f.0.to_owned()))
            else {
                return Err(e);
            };

            slug = slug_from_filename;
        }
    }

    // TITLE
    let key_title = "name";
    let mut title: String = String::new();

    if let Ok((f, _)) = parse_until_key_unquoted(file_content, key_title) {
        (file_content, title) = parse_unquoted_value(f, key_title)?;
    };

    // Guess the title from the slug if it wasn't found in the file
    if title.is_empty() {
        let mut title_from_slug = slug.split('-').collect::<Vec<&str>>().join(" ");

        if let Some(first_char) = title_from_slug.get_mut(0..1) {
            first_char.make_ascii_uppercase();
        };

        title = title_from_slug;
    };

    Ok((
        file_content,
        Ok(ParsableGameYmlData {
            executable_name,
            title,
            slug,
        }),
    ))
}

// LUTRIS LAUNCHER ----------------------------------------------------------------------
#[derive(Debug)]
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

        debug!("Lutris games directory exists: {}", path_games_dir.is_dir());
        debug!(
            "Lutris box art directory exists: {}",
            path_box_art_dir.is_dir()
        );
        debug!(
            "Lutris `game-paths.json` file exists: {}",
            path_game_paths_json.is_file()
        );

        Lutris {
            path_games_dir,
            path_box_art_dir,
            path_game_paths_json,
        }
    }

    /// Parse data from the Lutris `game-paths.json` file
    #[tracing::instrument(skip(self))]
    fn parse_game_paths_json(&self) -> Result<Arc<[ParsableGamePathsData]>, io::Error> {
        let game_paths_json_file = File::open(&self.path_game_paths_json).map_err(|e| {
            error!(
                "Error with reading game-paths.json file at {:?}:\n{e}",
                self.path_game_paths_json
            );
            e
        })?;

        Ok(BufReader::new(game_paths_json_file)
            .lines()
            .flatten()
            .filter_map(|line| {
                parse_double_quoted_key_value(&line)
                    .map(|(_, (run_id, exe_path))| {
                        let Some((parsed_game_dir, parsed_executable_name)) =
                            exe_path.rsplit_once('/')
                        else {
                            error!("Error extracting executable name from {:?}", exe_path);
                            return None;
                        };

                        Some(ParsableGamePathsData {
                            game_dir: parsed_game_dir.to_owned(),
                            executable_name: parsed_executable_name.to_owned(),
                            run_id: run_id.to_owned(),
                            // exe_path: PathBuf::from(exe_path),
                        })
                    })
                    .ok()
                    .flatten()
            })
            // Remove duplicate values as this file regularly has duplicates for some reason
            .sorted_by(|a, b| a.run_id.cmp(&b.run_id))
            .dedup_by(|a, b| a.run_id == b.run_id)
            .collect())
    }

    /// Parse data from the Lutris games directory, which contains 1 `.yml` file for each game
    #[tracing::instrument(skip(self))]
    fn parse_games_dir(&self) -> Result<Arc<[ParsableGameYmlData]>, io::Error> {
        Ok(read_dir(&self.path_games_dir)
            .map_err(|e| {
                error!("Error with reading games directory for Lutris: {e:?}");
                e
            })?
            .flatten()
            .filter_map(|path| self.get_parsable_game_yml_data(path.path()))
            .collect())
    }

    /// Parse relevant game data from a given Lutris game's `.yml` file
    #[tracing::instrument(skip(self))]
    fn get_parsable_game_yml_data(&self, path_game_yml: PathBuf) -> Option<ParsableGameYmlData> {
        let file_content = &read_to_string(&path_game_yml)
            .map_err(|e| {
                error!(
                    "Error with reading Lutris game `.yml` file at {:?}:\n{e}",
                    &path_game_yml
                )
            })
            .ok()?;

        let (_, parsed_data) = parse_game_yml(file_content, &path_game_yml).ok()?;
        parsed_data.ok()
    }

    /// Get all relevant game data by combining data from the `game-paths.json` file and
    /// each game's `.yml` file.
    /// Matching of the data from these sources is done using the executable path of the
    /// game, which is the only thing defined in both sources
    #[tracing::instrument]
    pub fn parse_game_data(&self) -> Result<Arc<[ParsableDataCombined]>, io::Error> {
        let game_paths_data = self.parse_game_paths_json()?;
        let game_yml_data = self.parse_games_dir()?;

        Ok(game_paths_data
            .iter()
            .cloned()
            .filter_map(|paths_data| {
                game_yml_data
                    .iter()
                    .find(|g| g.executable_name == paths_data.executable_name)
                    .map(|yml_data| ParsableDataCombined::combine(paths_data, yml_data.clone()))
            })
            .collect())
    }
}

impl Launcher for Lutris {
    fn is_detected(&self) -> bool {
        self.path_game_paths_json.exists()
            && self.path_games_dir.is_dir()
            && self.path_box_art_dir.is_dir()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        SupportedLaunchers::Lutris
    }

    #[tracing::instrument(skip(self))]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.parse_game_data()?;

        if parsed_data.is_empty() {
            warn!("No games (at least not with sufficient data) found for Lutris launcher");
        }

        Ok(parsed_data
            .iter()
            .map(
                |ParsableDataCombined {
                     game_dir,
                     run_id,
                     title,
                     slug,
                 }| {
                    let launch_command =
                        format!("env LUTRIS_SKIP_INIT=1 lutris lutris:rungameid/{run_id}");

                    let path_box_art =
                        some_if_file(self.path_box_art_dir.join(format!("{}.jpg", slug)));
                    let path_game_dir = some_if_dir(PathBuf::from(game_dir));

                    trace!("Lutris - Game directory found for '{title}': {path_game_dir:?}");
                    trace!("Lutris - Box art found for '{title}': {path_box_art:?}");

                    Game {
                        title: clean_game_title(title),
                        launch_command,
                        path_box_art,
                        path_game_dir,
                    }
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::linux::test_utils::get_mock_file_system_path;

    use super::*;

    #[test]
    fn test_lutris_launcher() -> Result<(), anyhow::Error> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Lutris::new(
            &path_file_system_mock.join(".config"),
            &path_file_system_mock.join(".cache"),
        );

        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;
        assert_eq!(games.len(), 3);

        assert_eq!(games[0].title, "GOG Galaxy");
        assert_eq!(games[1].title, "Epic Games Store");
        assert_eq!(games[2].title, "Warcraft 3");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());
        assert!(games[2].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_some());
        assert!(games[1].path_box_art.is_some());
        assert!(games[2].path_box_art.is_some());

        Ok(())
    }
}
