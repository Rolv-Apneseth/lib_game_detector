use std::sync::Arc;

use tracing::error;

use self::launchers::{
    bottles::Bottles,
    heroic::{heroic_amazon::HeroicAmazon, heroic_epic::HeroicEpic, heroic_gog::HeroicGOG},
    lutris::Lutris,
    minecraft::{at::MinecraftAT, prism::MinecraftPrism},
    steam::{Steam, SteamShortcuts},
};
use crate::data::{Game, GamesDetector, GamesPerLauncher, Launchers, SupportedLaunchers};
use dirs::{cache_dir, config_dir, data_dir, home_dir};

mod launchers;

pub struct GamesDetectorLinux {
    launchers: Launchers,
}

impl GamesDetectorLinux {
    pub fn new() -> GamesDetectorLinux {
        let launchers = GamesDetectorLinux::get_supported_launchers();
        GamesDetectorLinux { launchers }
    }

    pub fn get_supported_launchers() -> Launchers {
        let path_home = home_dir().expect("Failed to find the user's home directory");
        let path_config = config_dir().expect("Failed to find the user's config directory");
        let path_cache = cache_dir().expect("Failed to find the user's cache directory");
        let path_data = data_dir().expect("Failed to find the user's data directory");

        vec![
            Arc::new(Steam::new(&path_home, &path_data)),
            Arc::new(SteamShortcuts::new(&path_home, &path_data)),
            Arc::new(HeroicGOG::new(&path_home, &path_config)),
            Arc::new(HeroicEpic::new(&path_home, &path_config)),
            Arc::new(HeroicAmazon::new(&path_home, &path_config)),
            Arc::new(Lutris::new(
                &path_home,
                &path_config,
                &path_cache,
                &path_data,
            )),
            Arc::new(Bottles::new(&path_home, &path_data)),
            Arc::new(MinecraftPrism::new(&path_home, &path_data)),
            Arc::new(MinecraftAT::new(&path_home, &path_data)),
        ]
    }
}

impl GamesDetector for GamesDetectorLinux {
    fn get_detected_launchers(&self) -> Launchers {
        self.launchers
            .iter()
            .filter(|l| l.is_detected())
            .cloned()
            .collect()
    }

    fn get_all_detected_games(&self) -> Vec<Game> {
        self.get_detected_launchers()
            .iter()
            .filter_map(|l| l.get_detected_games().ok())
            .fold(vec![], |mut acc, g| {
                acc.extend(g);
                acc
            })
    }

    fn get_all_detected_games_with_box_art(&self) -> Vec<Game> {
        self.get_all_detected_games()
            .into_iter()
            .filter(|game| game.path_box_art.is_some())
            .collect()
    }

    fn get_all_detected_games_per_launcher(&self) -> GamesPerLauncher {
        self.get_detected_launchers()
            .into_iter()
            .filter_map(|l| match l.get_detected_games() {
                Ok(g) => Some((l.get_launcher_type(), g)),
                Err(_) => {
                    error!("Could not get games for launcher: {l:?}");
                    None
                }
            })
            .collect::<GamesPerLauncher>()
    }

    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<Vec<Game>> {
        self.get_detected_launchers()
            .into_iter()
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
