use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tracing::warn;

use super::{LAUNCHER, get_prism_games};
use crate::{
    data::{GamesResult, Launcher, SupportedLaunchers},
    macros::logs::debug_path,
    utils::get_launch_command,
};

fn get_cmd(args: [&str; 2]) -> Command {
    get_launch_command("prismlauncher", args, [])
}

#[derive(Debug)]
pub struct MinecraftPrism {
    path_root: PathBuf,
    path_config: PathBuf,
}

impl MinecraftPrism {
    pub fn new(path_data: &Path) -> Self {
        let path_root = path_data.join("PrismLauncher");
        let path_config = path_root.join("prismlauncher.cfg");

        debug_path!("root directory", path_root);
        debug_path!("prismlauncher.cfg", path_config);

        Self {
            path_root,
            path_config,
        }
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
        get_prism_games(&self.path_root, &self.path_config, get_cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_prism_launcher, *};
    use crate::{
        data::Game,
        error::GamesParsingError,
        linux::test_utils::{assert_cmd, get_mock_file_system_path},
    };

    #[test]
    fn test_minecraft_prism_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftPrism::new(&path_file_system_mock.join(".local/share"));

        check_prism_launcher(launcher, |g: &Game| assert_cmd(g, "prismlauncher"))
    }
}
