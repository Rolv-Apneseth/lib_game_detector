use std::{
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

use nom::IResult;
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::minecraft::get_minecraft_title,
    parsers::parse_value_json,
    utils::{get_launch_command, get_launch_command_flatpak, some_if_dir},
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
            debug!("{LAUNCHER} - Attempting to fall back to flatpak directory");
            is_using_flatpak = true;
            path_root = path_home.join(".var/app/com.atlauncher.ATLauncher/data");
        }

        let path_instances = path_root.join("instances");

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
                let config_path = dir_entry.path().join("instance.json");
                if !config_path.is_file() {
                    return None;
                }

                if let Ok(file_content) = read_to_string(&config_path) {
                    if let Ok((_, parsed_data)) = parse_instance_config(&file_content) {
                        return Some((dir_entry.path(), parsed_data));
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

                let path_game_dir = some_if_dir(instance_path);

                // No box art provided
                let path_box_art = None;

                trace!("{LAUNCHER} - Game directory found for '{title}': {path_game_dir:?}");
                trace!("{LAUNCHER} - Box art found for '{title}': {path_box_art:?}");

                Game {
                    title: get_minecraft_title(&title),
                    launch_command,
                    path_box_art,
                    path_game_dir,
                }
            })
            .collect();

        if games.is_empty() {
            warn!("{LAUNCHER} - No games found");
        };

        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::linux::test_utils::get_mock_file_system_path;

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_minecraft_at_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), anyhow::Error> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftAT::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;

        dbg!(&games);
        assert_eq!(games.len(), 2);

        assert!(games
            .iter()
            .any(|g| g.title == get_minecraft_title("Sky Factory")));
        assert!(games
            .iter()
            .any(|g| g.title == get_minecraft_title("Fabulously Optimized")));

        assert!(games.iter().all(|g| g.path_game_dir.is_some()));
        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
