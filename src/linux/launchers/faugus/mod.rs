// PATHS:
// - ~/.local/share/faugus-launcher/
// - Flatpak: ~/.var/app/io.github.Faugus.faugus-launcher/data/faugus-launcher
pub use flatpak::FaugusFlatpak;
pub use standard::Faugus;

mod flatpak;
mod standard;

use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
};

use nom::IResult;
use tracing::{debug, error, trace};

use crate::{
    data::{Game, SupportedLaunchers},
    error::GamesParsingError,
    macros::logs::debug_path,
    parsers::{parse_value_json, parse_value_json_unquoted},
    utils::{clean_game_title, get_existing_image_path, some_if_dir, some_if_file},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Faugus;

#[derive(Debug, Clone)]
struct FaugusPaths {
    box_art_dir: PathBuf,
    banner_dir: PathBuf,
    icons_dir: PathBuf,
    games_json: PathBuf,
}

impl FaugusPaths {
    fn new(path_data_faugus: PathBuf) -> Self {
        let box_art_dir = path_data_faugus.join("covers");
        let banner_dir = path_data_faugus.join("banners");
        let icons_dir = path_data_faugus.join("icons");
        let games_json = path_data_faugus.join("games.json");

        debug_path!("box art directory", box_art_dir);
        debug_path!("icons directory", icons_dir);
        debug_path!("banners directory", banner_dir);
        debug_path!("games.json file", games_json);

        Self {
            box_art_dir,
            banner_dir,
            icons_dir,
            games_json,
        }
    }
}

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

impl ParsableGameData {
    fn try_into_game<F>(self, paths: &FaugusPaths, get_launch_command: F) -> Option<Game>
    where
        F: Fn([&str; 2]) -> Command,
    {
        let ParsableGameData {
            id,
            title,
            runner,
            is_hidden,
            path_game_dir,
            path_icon,
            path_box_art,
        } = self;
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

        let path_box_art =
            path_box_art.or_else(|| get_existing_image_path(&paths.box_art_dir, &id));
        let path_icon = path_icon.or_else(|| get_existing_image_path(&paths.icons_dir, &id));
        let path_hero = get_existing_image_path(&paths.banner_dir, &id);

        trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
        trace!("{LAUNCHER} - Box art for '{title}': {path_box_art:?}");
        trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");
        trace!("{LAUNCHER} - Hero image for '{title}': {path_hero:?}");

        let launch_command = {
            let base_args = ["--game", &id];
            get_launch_command(base_args)
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
    }
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

#[tracing::instrument(level = "trace")]
fn get_parsable_games_json_data(
    path_games_json: &Path,
) -> Result<Vec<ParsableGameData>, GamesParsingError> {
    let file_content = read_to_string(path_games_json).inspect_err(|e| {
        error!("Error with reading games.json file at {path_games_json:?}: {e:#?}");
    })?;

    let mut parsed_games_data: Vec<ParsableGameData> = Vec::new();
    let mut file_content_str: &str = &file_content;

    while let Ok((new_file_content, parsed_data)) = parse_game_from_library(file_content_str) {
        file_content_str = new_file_content;
        parsed_games_data.push(parsed_data)
    }

    Ok(parsed_games_data)
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
    };

    pub fn check_faugus_launcher<F>(
        launcher: impl Launcher,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;
        // One entry hidden, one is Steam - both should be filtered out
        assert_eq!(games.len(), 2);

        assert_cmd(&games[0]);

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
