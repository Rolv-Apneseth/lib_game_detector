// PATHS:
// - ~/.local/share/faugus-launcher/
// - Flatpak: ~/.var/app/io.github.Faugus.faugus-launcher/data/faugus-launcher

use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use nom::IResult;
use tracing::{debug, error, trace};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    macros::logs::{debug_fallback_flatpak, debug_path, warn_no_games},
    parsers::{parse_value_json, parse_value_json_unquoted},
    utils::{
        clean_game_title, get_existing_image_path, get_launch_command, get_launch_command_flatpak,
        some_if_dir, some_if_file,
    },
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Faugus;

#[derive(Debug, Clone)]
pub struct ParsableGameData {
    id: String,
    title: String,
    runner: String,
    path_game_dir: Option<PathBuf>,
    is_hidden: bool,
    path_icon: Option<PathBuf>,
    path_box_art: Option<PathBuf>,
}

// UTILS --------------------------------------------------------------------------------
/// Used to parse a single game's relevant data from the given Faugus games.json file's contents
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_library<'a>(file_content: &'a str) -> IResult<&'a str, ParsableGameData> {
    // ID
    let (file_content, id) = parse_value_json(file_content, "gameid")?;

    // TITLE
    let (file_content, title) = parse_value_json(file_content, "title")?;

    // GAME DIR
    let (file_content, game_dir) = parse_value_json(file_content, "path")?;
    let path_game_dir = some_if_dir(
        PathBuf::from(game_dir.trim())
            .parent()
            .map(ToOwned::to_owned)
            .unwrap_or_default(),
    );

    // RUNNER
    let (file_content, runner) = parse_value_json(file_content, "runner")?;

    // BOX ART
    let (file_content, box_art) = parse_value_json(file_content, "cover")?;
    let path_box_art = some_if_file(PathBuf::from(box_art));

    // HIDDEN
    let (file_content, hidden) = parse_value_json_unquoted(file_content, "hidden")?;
    let is_hidden = hidden == "true";

    // ICON
    let (file_content, icon) = parse_value_json(file_content, "icon")?;
    let path_icon = some_if_file(PathBuf::from(icon.trim()));

    Ok((
        file_content,
        ParsableGameData {
            id,
            title,
            runner,
            is_hidden,
            path_game_dir,
            path_icon,
            path_box_art,
        },
    ))
}

// FAUGUS LAUNCHER ----------------------------------------------------------------------
#[derive(Debug)]
pub struct Faugus {
    path_games_json: PathBuf,
    path_icon_dir: PathBuf,
    path_banner_dir: PathBuf,
    path_box_art_dir: PathBuf,
    is_using_flatpak: bool,
}

impl Faugus {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let mut path_data_faugus = path_data.join("faugus-launcher");
        let mut is_using_flatpak = false;

        if !path_data_faugus.is_dir() {
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            path_data_faugus =
                path_home.join(".var/app/io.github.Faugus.faugus-launcher/data/faugus-launcher");
        }

        let path_box_art_dir = path_data_faugus.join("covers");
        let path_banner_dir = path_data_faugus.join("banners");
        let path_icon_dir = path_data_faugus.join("icons");
        let path_games_json = path_data_faugus.join("games.json");

        debug_path!("box art directory", path_box_art_dir);
        debug_path!("icons directory", path_icon_dir);
        debug_path!("banners directory", path_banner_dir);
        debug_path!("games.json file", path_games_json);

        Self {
            path_box_art_dir,
            path_icon_dir,
            path_banner_dir,
            path_games_json,
            is_using_flatpak,
        }
    }

    #[tracing::instrument(level = "trace")]
    fn get_parsable_games_json_data(&self) -> Vec<ParsableGameData> {
        let Ok(file_content) = read_to_string(&self.path_games_json).map_err(|e| {
            error!(
                "Error with reading games.json file at {:?}:\n{e}",
                self.path_games_json
            );
        }) else {
            return vec![];
        };

        let mut parsed_games_data: Vec<ParsableGameData> = Vec::new();
        let mut file_content_str: &str = &file_content;

        while let Ok((new_file_content, parsed_data)) = parse_game_from_library(file_content_str) {
            file_content_str = new_file_content;
            parsed_games_data.push(parsed_data)
        }

        parsed_games_data
    }
}

impl Launcher for Faugus {
    fn is_detected(&self) -> bool {
        self.path_games_json.is_file()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.get_parsable_games_json_data();

        if parsed_data.is_empty() {
            warn_no_games!();
        }

        Ok(parsed_data
            .into_iter()
            .filter_map(
                |ParsableGameData {
                     id,
                     title,
                     runner,
                     is_hidden,
                     path_game_dir,
                     path_icon,
                     path_box_art,
                 }| {
                    // Ignore hidden entries
                    if is_hidden {
                        debug!("Skipping '{title}' - marked as hidden");
                        return None;
                    }
                    // Ignore Steam games to avoid duplicates
                    if runner.to_lowercase() == "steam" {
                        debug!("Skipping '{title}' from Steam");
                        return None;
                    }

                    let path_box_art = path_box_art
                        .or_else(|| get_existing_image_path(&self.path_box_art_dir, &id));
                    let path_icon =
                        path_icon.or_else(|| get_existing_image_path(&self.path_icon_dir, &id));
                    let path_hero = get_existing_image_path(&self.path_banner_dir, &id);

                    trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
                    trace!("{LAUNCHER} - Box art for '{title}': {path_box_art:?}");
                    trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");
                    trace!("{LAUNCHER} - Hero image for '{title}': {path_hero:?}");

                    let launch_command = {
                        let base_args = ["--game", &id];
                        if self.is_using_flatpak {
                            get_launch_command_flatpak(
                                "io.github.Faugus.faugus-launcher",
                                [],
                                base_args,
                                [],
                            )
                        } else {
                            get_launch_command("faugus-launcher", base_args, [])
                        }
                    };
                    trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

                    Some(Game {
                        title: clean_game_title(title),
                        path_icon,
                        launch_command,
                        path_box_art,
                        path_game_dir,
                        source: LAUNCHER.clone(),
                        path_hero,
                        path_header: None,
                    })
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_faugus_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Faugus::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;
        // One entry hidden, one is Steam - both should be filtered out
        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, "TMNT - Shredder's Revenge");
        assert_eq!(games[1].title, "Warcraft III: Reign of Chaos");

        assert!(games[0].path_box_art.is_none());
        assert!(games[1].path_box_art.is_some());

        assert!(games[0].path_hero.is_none());
        assert!(games[1].path_hero.is_some());

        for g in games {
            assert!(g.path_game_dir.is_some());
            assert!(g.path_icon.is_some());
        }

        Ok(())
    }
}
