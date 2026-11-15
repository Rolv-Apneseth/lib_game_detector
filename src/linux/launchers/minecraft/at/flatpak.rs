use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tracing::warn;

use super::{LAUNCHER, get_at_games};
use crate::{
    data::{GamesResult, Launcher},
    macros::logs::debug_path,
    utils::get_launch_command_flatpak,
};

fn get_cmd(args: [&str; 2]) -> Command {
    get_launch_command_flatpak("com.atlauncher.ATLauncher", [], args, [])
}

#[derive(Debug)]
pub struct MinecraftATFlatpak {
    path_instances: PathBuf,
}

impl MinecraftATFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let path_root = path_home.join(".var/app/com.atlauncher.ATLauncher/data");
        let path_instances = path_root.join("instances");

        debug_path!("root directory", path_root);
        debug_path!("instances directory", path_instances);

        Self { path_instances }
    }
}

impl Launcher for MinecraftATFlatpak {
    fn is_detected(&self) -> bool {
        self.path_instances.is_dir()
    }

    fn get_launcher_type(&self) -> crate::data::SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        get_at_games(&self.path_instances, get_cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_utils::check_prism_launcher as check_at_launcher, *};
    use crate::{
        error::GamesParsingError,
        linux::test_utils::{assert_cmd_flatpak, get_mock_file_system_path},
    };

    #[test]
    fn test_minecraft_at_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = MinecraftATFlatpak::new(&path_file_system_mock);

        check_at_launcher(launcher, assert_cmd_flatpak)
    }
}
