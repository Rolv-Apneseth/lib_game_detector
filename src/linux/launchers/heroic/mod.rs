pub mod amazon;
pub mod epic;
pub mod gog;
pub mod sideload;

use std::{
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
    process::Command,
};

use nom::IResult;
use tracing::debug;

use crate::{
    parsers::{parse_value_json, parse_value_json_unquoted},
    utils::{clean_game_title, get_launch_command, get_launch_command_flatpak},
};

/// Useful data about a game which is parseable from a Heroic Games Launcher library file
#[derive(Debug)]
struct ParsableLibraryData {
    app_id: String,
    install_path: String,
    title: String,
}

/// Parses a single (installed) game from a Heroic Games Launcher library file
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_library_common(file_content: &str) -> IResult<&str, ParsableLibraryData> {
    // ID
    let (file_content, app_id) = parse_value_json(file_content, "app_name")?;

    // Keep checkpoint of file content because `is_installed` comes after the `install_path`
    // and `title` may come before install info
    let file_content_checkpoint = file_content;

    // IS_INSTALLED
    let (file_content, is_installed) = parse_value_json_unquoted(file_content, "is_installed")?;

    // Continue to next game if not installed
    if is_installed == *"false" {
        return parse_game_from_library_common(file_content);
    }

    // INSTALL_PATH
    let (file_content, install_path) = parse_value_json(file_content_checkpoint, "install_path")?;

    // TITLE
    let (file_content, title) = parse_value_json(file_content, "title")?;

    Ok((
        file_content,
        ParsableLibraryData {
            app_id,
            title: clean_game_title(title),
            install_path,
        },
    ))
}

/// Parses all (installed) games from a given Heroic Games Launcher library file
#[tracing::instrument]
fn parse_all_games_from_library<T>(
    path_library: &Path,
    parse_fn: fn(file_content: &str) -> IResult<&str, T>,
) -> Result<Vec<T>, io::Error> {
    let mut parsed_data = Vec::new();

    let file_content = read_to_string(path_library)?;
    let mut file_content_str: &str = &file_content;

    // Parse individual games from library file until no more are found
    loop {
        let Ok((new_file_content, parsed_game_data)) = parse_fn(file_content_str) else {
            break;
        };

        file_content_str = new_file_content;
        parsed_data.push(parsed_game_data);
    }

    Ok(parsed_data)
}

#[tracing::instrument]
fn parse_all_games_from_library_common(path: &Path) -> Result<Vec<ParsableLibraryData>, io::Error> {
    parse_all_games_from_library::<ParsableLibraryData>(path, parse_game_from_library_common)
}

/// Get path to the Heroic Games Launcher config dir, falling back to the flatpak version if necessary
fn get_heroic_config_path(path_home: &Path, path_config: &Path) -> (PathBuf, bool) {
    let mut is_using_flatpak = false;
    let mut path_heroic_config = path_config.join("heroic");

    if !path_heroic_config.is_dir() {
        debug!("Heroic - Attempting to fall back to flatpak");

        is_using_flatpak = true;
        path_heroic_config = path_home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic");
    }

    (path_heroic_config, is_using_flatpak)
}

/// Get launch command for game from any of the Heroic Games Launcher sources
fn get_launch_command_for_heroic_source(
    source: &str,
    app_id: &str,
    is_using_flatpak: bool,
) -> Command {
    let game_run_arg = format!("heroic://launch/{source}/{app_id}");
    let args = [game_run_arg.as_str()];

    if is_using_flatpak {
        get_launch_command_flatpak("com.heroicgameslauncher.hgl", [], args, [])
    } else {
        get_launch_command("xdg-open", args, [])
    }
}
