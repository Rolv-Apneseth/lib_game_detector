use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tracing::warn;

use super::{LAUNCHER, get_prism_games};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    macros::logs::debug_path,
    utils::get_launch_command_flatpak,
};

fn get_cmd(args: [&str; 2]) -> Command {
    get_launch_command_flatpak("org.prismlauncher.PrismLauncher", [], args, [])
}

#[derive(Debug)]
pub struct MinecraftPrismFlatpak {
    path_root: PathBuf,
    path_config: PathBuf,
}

impl MinecraftPrismFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let path_root =
            path_home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher");
        let path_config = path_root.join("prismlauncher.cfg");

        debug_path!("root directory", path_root);
        debug_path!("prismlauncher.cfg", path_config);

        Self {
            path_root,
            path_config,
        }
    }
}

impl Launcher for MinecraftPrismFlatpak {
    fn is_detected(&self) -> bool {
        self.path_config.is_file()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        SupportedLaunchers::MinecraftPrism
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        get_prism_games(&self.path_root, &self.path_config, get_cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_prism_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_minecraft_prism_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftPrismFlatpak::new(&path_file_system_mock);

        check_prism_launcher(launcher, assert_cmd_flatpak)
    }
}
