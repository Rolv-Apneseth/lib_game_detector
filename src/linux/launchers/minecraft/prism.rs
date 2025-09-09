// PATHS:
// - ~/.local/share/PrismLauncher/
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
    parsers::{parse_until_key_cfg, parse_value_cfg},
    utils::{get_launch_command, get_launch_command_flatpak, some_if_dir, some_if_file},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::MinecraftPrism;

struct ParsableConfigData {
    path_instances: PathBuf,
}

#[derive(Debug)]
pub struct MinecraftPrism {
    path_root: PathBuf,
    path_config: PathBuf,
    is_using_flatpak: bool,
}

impl MinecraftPrism {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let mut is_using_flatpak = false;
        let mut path_root = path_data.join("PrismLauncher");

        if !path_root.is_dir() {
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            path_root =
                path_home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher");
        }

        let path_config = path_root.join("prismlauncher.cfg");

        debug_path!("root directory", path_root);

        Self {
            path_root,
            path_config,
            is_using_flatpak,
        }
    }

    #[tracing::instrument(level = "trace", skip(file_content))]
    fn parse_prism_config<'a>(
        &self,
        file_content: &'a str,
    ) -> IResult<&'a str, ParsableConfigData> {
        // INSTANCES DIR
        let instances_id = "InstanceDir";
        let (file_content, _) = parse_until_key_cfg(file_content, instances_id)?;
        let (file_content, instances_dir) = parse_value_cfg(file_content, instances_id)?;

        let mut path_instances = PathBuf::from(instances_dir);
        if !path_instances.is_absolute() {
            path_instances = self.path_root.join(path_instances);
        }

        Ok((file_content, ParsableConfigData { path_instances }))
    }
}

impl Launcher for MinecraftPrism {
    fn is_detected(&self) -> bool {
        self.path_config.is_file()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::MinecraftPrism
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let file_content = read_to_string(&self.path_config)?;

        let (_, config_data) = self.parse_prism_config(&file_content)?;
        let ParsableConfigData { path_instances } = config_data;

        if !path_instances.is_dir() {
            error!(
                "{LAUNCHER} - the parsed instances dir does not exist: {:?}",
                path_instances
            );
        }

        let games: Vec<Game> = read_dir(&path_instances)?
            .flatten()
            .filter_map(|dir_entry| {
                let path = dir_entry.path();
                if !path.is_dir() || !path.join("instance.cfg").is_file() {
                    return None;
                }

                Some(dir_entry.file_name().to_str()?.to_owned())
            })
            .map(|title| {
                let launch_command = {
                    let args = ["--launch", &title];
                    if self.is_using_flatpak {
                        get_launch_command_flatpak("org.prismlauncher.PrismLauncher", [], args, [])
                    } else {
                        get_launch_command("prismlauncher", args, [])
                    }
                };
                trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

                let path_game_dir = some_if_dir(path_instances.join(&title));
                let path_icon = get_path_icon(path_game_dir.as_ref());
                // No box art provided
                let path_box_art = None;

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

fn get_path_icon(path_instance: Option<&PathBuf>) -> Option<PathBuf> {
    let path_instance = path_instance?;

    some_if_file(path_instance.join("icon.png"))
        .or_else(|| some_if_file(path_instance.join("minecraft").join("icon.png")))
        .or_else(|| some_if_file(path_instance.join(".minecraft").join("icon.png")))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_minecraft_prism_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftPrism::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let mut games = launcher.get_detected_games()?;
        games.sort_by_key(|a| a.title.clone());

        assert_eq!(games.len(), 3);

        assert_eq!(games[0].title, get_minecraft_title("1.20.6"));
        assert_eq!(games[1].title, get_minecraft_title("All The Forge 10"));
        assert_eq!(games[2].title, get_minecraft_title("The Pixelmon Modpack"));

        assert!(games[0].path_icon.is_none());
        assert!(games[1].path_icon.as_ref().is_some_and(|p| p.is_file()));
        assert!(games[2].path_icon.as_ref().is_some_and(|p| p.is_file()));

        assert!(games.iter().all(|g| g.path_game_dir.is_some()));
        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
