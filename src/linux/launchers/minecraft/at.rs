// PATHS:
// - ~/.local/share/atlauncher/
use std::{
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

use nom::IResult;
use tracing::{error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::minecraft::get_minecraft_title,
    macros::logs::{debug_fallback_flatpak, debug_path, warn_no_games},
    parsers::parse_value_json,
    utils::{get_existing_image_path, get_launch_command, get_launch_command_flatpak, some_if_dir},
};

struct ParsableInstanceConfigData {
    title: String,
}

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::MinecraftAT;

/// Used for parsing relevant instance's data from the given `instance.json` file's contents
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_instance_config(file_content: &str) -> IResult<&str, ParsableInstanceConfigData> {
    // TITLE
    let (file_content, title) = parse_value_json(file_content, "name")?;

    Ok((file_content, ParsableInstanceConfigData { title }))
}

#[derive(Debug)]
pub struct MinecraftAT {
    path_instances: PathBuf,
    is_using_flatpak: bool,
}

impl MinecraftAT {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let mut is_using_flatpak = false;
        let mut path_root = path_data.join("atlauncher");

        if !path_root.is_dir() {
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            path_root = path_home.join(".var/app/com.atlauncher.ATLauncher/data");
        }

        let path_instances = path_root.join("instances");

        debug_path!("root directory", path_root);

        Self {
            path_instances,
            is_using_flatpak,
        }
    }
}

impl Launcher for MinecraftAT {
    fn is_detected(&self) -> bool {
        self.path_instances.is_dir()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let games: Vec<Game> = read_dir(&self.path_instances)?
            .flatten()
            .filter_map(|dir_entry| {
                let dir_path = dir_entry.path();
                let config_path = dir_path.join("instance.json");
                if !config_path.is_file() {
                    return None;
                }

                if let Ok(file_content) = read_to_string(&config_path) {
                    if let Ok((_, parsed_data)) = parse_instance_config(&file_content) {
                        return Some((dir_path, parsed_data));
                    };
                };

                error!("{LAUNCHER} - error parsing instance file at {config_path:?}");
                None
            })
            .map(|(instance_path, ParsableInstanceConfigData { title })| {
                let launch_command = {
                    let args = ["--launch", &title];
                    if self.is_using_flatpak {
                        get_launch_command_flatpak("com.atlauncher.ATLauncher", [], args, [])
                    } else {
                        get_launch_command("atlauncher", args, [])
                    }
                };
                trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

                // No box art provided
                let path_box_art = None;

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
                }
            })
            .collect();

        if games.is_empty() {
            warn_no_games!();
        };

        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_minecraft_at_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftAT::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let mut games = launcher.get_detected_games()?;
        games.sort_by_key(|a| a.title.clone());

        assert_eq!(games.len(), 2);

        assert_eq!(games[0].title, get_minecraft_title("Fabulously Optimized"));
        assert_eq!(games[1].title, get_minecraft_title("Sky Factory"));

        assert!(games[0].path_icon.as_ref().is_some_and(|p| p.is_file()));
        assert!(games[1].path_icon.is_none());

        assert!(games.iter().all(|g| g.path_game_dir.is_some()));
        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
