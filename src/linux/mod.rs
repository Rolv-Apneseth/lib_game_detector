use std::{process::exit, rc::Rc, sync::Arc};

use self::launchers::{
    bottles::Bottles,
    heroic::{heroic_amazon::HeroicAmazon, heroic_epic::HeroicEpic, heroic_gog::HeroicGOG},
    lutris::Lutris,
    minecraft::{at::MinecraftAT, prism::MinecraftPrism},
    steam::Steam,
};
use crate::data::{
    GamesDetector, GamesPerLauncherSlice, GamesSlice, LaunchersSlice, SupportedLaunchers,
};
use directories::BaseDirs;
use tracing::error;

mod launchers;

pub struct GamesDetectorLinux {
    launchers: LaunchersSlice,
}

impl GamesDetectorLinux {
    pub fn new() -> GamesDetectorLinux {
        let Some(user_dirs) = BaseDirs::new() else {
            error!("No valid $HOME directory found for the current user");
            exit(1);
        };

        let launchers = GamesDetectorLinux::get_supported_launchers(&user_dirs);

        GamesDetectorLinux { launchers }
    }

    pub fn get_supported_launchers(base_dirs: &BaseDirs) -> LaunchersSlice {
        let path_home = base_dirs.home_dir();
        let path_config = base_dirs.config_dir();
        let path_cache = base_dirs.cache_dir();
        let path_data = base_dirs.data_dir();

        Rc::new([
            Arc::new(Steam::new(path_home, path_data)),
            Arc::new(HeroicGOG::new(path_home, path_config)),
            Arc::new(HeroicEpic::new(path_home, path_config)),
            Arc::new(HeroicAmazon::new(path_home, path_config)),
            Arc::new(Lutris::new(path_home, path_config, path_cache)),
            Arc::new(Bottles::new(path_home, path_data)),
            Arc::new(MinecraftPrism::new(path_home, path_data)),
            Arc::new(MinecraftAT::new(path_home, path_data)),
        ])
    }
}

impl GamesDetector for GamesDetectorLinux {
    fn get_detected_launchers(&self) -> LaunchersSlice {
        self.launchers
            .iter()
            .filter(|l| l.is_detected())
            .cloned()
            .collect()
    }

    fn get_all_detected_games(&self) -> Option<GamesSlice> {
        self.get_detected_launchers()
            .iter()
            .filter_map(|l| l.get_detected_games().ok())
            .reduce(|acc, e| acc.iter().cloned().chain(e.iter().cloned()).collect())
    }

    fn get_all_detected_games_with_box_art(&self) -> Option<GamesSlice> {
        self.get_all_detected_games().map(|slice| {
            slice
                .iter()
                .filter(|game| game.path_box_art.is_some())
                .cloned()
                .collect()
        })
    }

    fn get_all_detected_games_per_launcher(&self) -> Option<GamesPerLauncherSlice> {
        let categorised_games = self
            .get_detected_launchers()
            .iter()
            .filter_map(|l| match l.get_detected_games() {
                Ok(g) => Some((l.get_launcher_type(), g)),
                Err(_) => {
                    error!("Could not get games for launcher: {l:?}");
                    None
                }
            })
            .collect::<GamesPerLauncherSlice>();

        if categorised_games.is_empty() {
            return None;
        };

        Some(categorised_games)
    }

    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<GamesSlice> {
        self.get_detected_launchers()
            .iter()
            .find(|l| l.get_launcher_type() == launcher_type)
            .and_then(|l| {
                l.get_detected_games()
                    .map_err(|_| {
                        error!(
                            "Launcher detected but there was an error with getting detected games for the launcher: {:?}",
                            l.get_launcher_type()
                        )
                    })
                    .ok()
            })
    }
}

// Test utils
#[cfg(test)]
pub mod test_utils {
    use std::path::PathBuf;

    pub fn get_mock_file_system_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/file_system_mocks/linux")
    }
}
