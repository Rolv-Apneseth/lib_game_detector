pub mod heroic_amazon;
pub mod heroic_epic;
pub mod heroic_gog;

use std::{fs::read_to_string, io, path::Path};

use nom::IResult;

use crate::utils::{
    clean_game_title, parse_double_quoted_value, parse_unquoted_json_value, parse_until_key,
};

/// Useful data about a game which is parsable from a Heroic Games Launcher library file
#[derive(Debug)]
struct ParsableLibraryData {
    app_id: String,
    install_path: String,
    title: String,
}

/// Utility function which parses a single (installed) game from a Heroic Games Launcher library file
fn parse_game_from_library(file_content: &str) -> IResult<&str, ParsableLibraryData> {
    // ID
    let key_id = "app_name";
    let (file_content, _) = parse_until_key(file_content, key_id)?;
    let (file_content, app_id) = parse_double_quoted_value(file_content, key_id)?;

    // Keep checkpoint of file content because `is_installed` comes after the `install_path`
    let file_content_checkpoint = file_content;

    // IS_INSTALLED
    let key_installed = "is_installed";
    let (file_content, _) = parse_until_key(file_content, key_installed)?;
    let (file_content, is_installed) = parse_unquoted_json_value(file_content, key_installed)?;

    // Continue to next game if not installed
    if is_installed == *"false" {
        return parse_game_from_library(file_content);
    }

    // INSTALL_PATH
    let key_path = "install_path";
    let (file_content, _) = parse_until_key(file_content_checkpoint, key_path)?;
    let (file_content, install_path) = parse_double_quoted_value(file_content, key_path)?;

    // TITLE
    let key_title = "title";
    let (file_content, _) = parse_until_key(file_content, key_title)?;
    let (file_content, title) = parse_double_quoted_value(file_content, key_title)?;

    Ok((
        file_content,
        ParsableLibraryData {
            app_id,
            title: clean_game_title(&title),
            install_path,
        },
    ))
}

/// Utitlity function which parses all (installed) games from a given Heroic Games Launcher library file
fn parse_all_games_from_library(
    path_library: &Path,
) -> Result<Vec<ParsableLibraryData>, io::Error> {
    let mut parsed_data = Vec::new();

    let file_content = read_to_string(path_library)?;
    let mut file_content_str: &str = &file_content;

    // Parse individual games from library file until no more are found
    loop {
        let Ok((new_file_content, parsed_game_data)) = parse_game_from_library(file_content_str)
        else {
            break;
        };

        file_content_str = new_file_content;
        parsed_data.push(parsed_game_data);
    }

    Ok(parsed_data)
}
