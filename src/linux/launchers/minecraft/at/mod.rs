// PATHS:
// - ~/.local/share/atlauncher/
pub use flatpak::MinecraftATFlatpak;
pub use standard::MinecraftAT;

mod flatpak;
mod standard;

use std::{
    fs::{DirEntry, read_dir, read_to_string},
    path::{Path, PathBuf},
    process::Command,
};

use nom::IResult;
use tracing::{error, trace, warn};

use crate::{
    data::{Game, SupportedLaunchers},
    error::GamesParsingError,
    linux::launchers::minecraft::get_minecraft_title,
    macros::logs::warn_no_games,
    parsers::parse_value_json,
    utils::{get_existing_image_path, some_if_dir},
};

type FnGetCmd = fn(s: [&str; 2]) -> Command;

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::MinecraftAT;

struct ParsableInstanceConfigData {
    title: String,
}

/// Used for parsing relevant instance's data from the given `instance.json` file's contents
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_instance_config(file_content: &str) -> IResult<&str, ParsableInstanceConfigData> {
    // TITLE
    let (file_content, title) = parse_value_json(file_content, "name")?;

    Ok((file_content, ParsableInstanceConfigData { title }))
}

fn try_parse_instance_dir_entry(
    dir_entry: DirEntry,
) -> Option<(PathBuf, ParsableInstanceConfigData)> {
    let dir_path = dir_entry.path();
    let config_path = dir_path.join("instance.json");
    if !config_path.is_file() {
        return None;
    }

    if let Ok(file_content) = read_to_string(&config_path)
        && let Ok((_, parsed_data)) = parse_instance_config(&file_content)
    {
        return Some((dir_path, parsed_data));
    };

    error!("{LAUNCHER} - error parsing instance file at {config_path:?}");
    None
}

fn get_game_from_instance_data(
    instance_path: PathBuf,
    instance_data: ParsableInstanceConfigData,
    get_launch_command: FnGetCmd,
) -> Game {
    let title = instance_data.title;

    let launch_command = {
        // WARNING: issue when passing instance names with spaces
        // https://github.com/ATLauncher/ATLauncher/issues/1053
        let args = ["--launch", &title];
        get_launch_command(args)
    };
    trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

    // AFAICT, no images saved other than the icon
    let (path_box_art, path_hero, path_header) = (None, None, None);

    let path_icon = get_existing_image_path(&instance_path, "instance");
    let path_game_dir = some_if_dir(instance_path);

    trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
    trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");

    Game {
        title: get_minecraft_title(&title),
        launch_command,
        path_box_art,
        path_game_dir,
        path_icon,
        source: LAUNCHER.clone(),
        path_hero,
        path_header,
    }
}

fn get_at_games(
    path_instances: &Path,
    get_launch_command: FnGetCmd,
) -> Result<Vec<Game>, GamesParsingError> {
    let games: Vec<Game> = read_dir(path_instances)?
        .flatten()
        .filter_map(try_parse_instance_dir_entry)
        .map(|(instance_path, instance_data)| {
            get_game_from_instance_data(instance_path, instance_data, get_launch_command)
        })
        .collect();

    if games.is_empty() {
        warn_no_games!();
    };

    Ok(games)
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
        linux::launchers::minecraft::get_minecraft_title,
    };

    pub fn check_prism_launcher<F>(
        launcher: impl Launcher,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        let mut games = launcher.get_detected_games()?;
        games.sort_by_key(|a| a.title.clone());

        assert_eq!(games.len(), 2);

        assert_cmd(&games[0]);

        assert_eq!(games[0].title, get_minecraft_title("Fabulously Optimized"));
        assert_eq!(games[1].title, get_minecraft_title("Sky Factory"));

        assert!(games[0].path_icon.as_ref().is_some_and(|p| p.is_file()));
        assert!(games[1].path_icon.is_none());

        assert!(games.iter().all(|g| g.path_game_dir.is_some()));
        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
