use std::{
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use nom::IResult;
use tracing::{error, trace};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    linux::launchers::minecraft::get_minecraft_title,
    parsers::{parse_until_key_cfg, parse_value_cfg},
    utils::{get_launch_command, get_launch_command_flatpak, some_if_dir},
};

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
            trace!("Minecraft (Prism) - Attempting to fall back to flatpak directory");
            is_using_flatpak = true;
            path_root =
                path_home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher");
        }

        let path_config = path_root.join("prismlauncher.cfg");

        Self {
            path_root,
            path_config,
            is_using_flatpak,
        }
    }

    #[tracing::instrument(skip_all)]
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

        return Ok((file_content, ParsableConfigData { path_instances }));
    }
}

impl Launcher for MinecraftPrism {
    fn is_detected(&self) -> bool {
        self.path_config.is_file()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::MinecraftPrism
    }

    #[tracing::instrument(skip(self))]
    fn get_detected_games(&self) -> GamesResult {
        let file_content = read_to_string(&self.path_config)?;

        let (_, config_data) = self.parse_prism_config(&file_content).map_err(|_| {
            anyhow!(
                "Couldn't parse Prism launcher config at {:?}",
                self.path_config
            )
        })?;
        let ParsableConfigData { path_instances } = config_data;

        if !path_instances.is_dir() {
            error!(
                "Minecraft (Prism) - the parsed instances dir does not exist: {:?}",
                path_instances
            );
        }

        Ok(read_dir(&path_instances)?
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
                trace!("Minecraft (Prism) - launch command for '{title}': {launch_command:?}");

                let path_game_dir = some_if_dir(path_instances.join(&title));
                // No box art provided
                let path_box_art = None;

                trace!("Minecraft (Prism) - Game directory found for '{title}': {path_game_dir:?}");
                trace!("Minecraft (Prism) - Box art found for '{title}': {path_box_art:?}");

                Game {
                    title: get_minecraft_title(&title),
                    launch_command,
                    path_box_art,
                    path_game_dir,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::linux::test_utils::get_mock_file_system_path;

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_minecraft_prism_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), anyhow::Error> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftPrism::new(
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
            .any(|g| g.title == get_minecraft_title("All The Forge 10")));
        assert!(games
            .iter()
            .any(|g| g.title == get_minecraft_title("The Pixelmon Modpack")));

        assert!(games.iter().all(|g| g.path_game_dir.is_some()));
        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
