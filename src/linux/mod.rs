use std::sync::Arc;

use dirs::{cache_dir, config_dir, data_dir, home_dir};
use launchers::heroic::sideload::HeroicSideloadFlatpak;
use tracing::error;

use self::launchers::{
    bottles::Bottles,
    heroic::{amazon::HeroicAmazon, epic::HeroicEpic, gog::HeroicGOG},
    itch::Itch,
    lutris::Lutris,
    minecraft::{at::MinecraftAT, prism::MinecraftPrism},
    steam::{Steam, SteamShortcuts},
};
use crate::{
    data::{Game, GamesDetector, GamesPerLauncher, Launchers, SupportedLaunchers},
    linux::launchers::{
        bottles::BottlesFlatpak,
        faugus::{Faugus, FaugusFlatpak},
        heroic::{
            amazon::HeroicAmazonFlatpak, epic::HeroicEpicFlatpak, gog::HeroicGOGFlatpak,
            sideload::HeroicSideload,
        },
        itch::ItchFlatpak,
        lutris::LutrisFlatpak,
        minecraft::{at::MinecraftATFlatpak, prism::MinecraftPrismFlatpak},
        steam::{SteamFlatpak, SteamShortcutsFlatpak},
    },
};

mod launchers;

#[derive(Debug)]
pub struct GamesDetectorLinux {
    launchers: Launchers,
}

impl Default for GamesDetectorLinux {
    fn default() -> Self {
        let launchers = GamesDetectorLinux::get_supported_launchers();
        GamesDetectorLinux { launchers }
    }
}

impl GamesDetectorLinux {
    pub fn get_supported_launchers() -> Launchers {
        let path_home = home_dir().expect("Failed to find the user's home directory");
        let path_config = config_dir().expect("Failed to find the user's config directory");
        let path_cache = cache_dir().expect("Failed to find the user's cache directory");
        let path_data = data_dir().expect("Failed to find the user's data directory");

        vec![
            Arc::new(Steam::new(&path_home, &path_data)),
            Arc::new(SteamFlatpak::new(&path_home)),
            Arc::new(SteamShortcuts::new(&path_home, &path_data)),
            Arc::new(SteamShortcutsFlatpak::new(&path_home)),
            Arc::new(HeroicGOG::new(&path_config)),
            Arc::new(HeroicGOGFlatpak::new(&path_home)),
            Arc::new(HeroicEpic::new(&path_config)),
            Arc::new(HeroicEpicFlatpak::new(&path_home)),
            Arc::new(HeroicAmazon::new(&path_config)),
            Arc::new(HeroicAmazonFlatpak::new(&path_home)),
            Arc::new(HeroicSideload::new(&path_config)),
            Arc::new(HeroicSideloadFlatpak::new(&path_home)),
            Arc::new(Lutris::new(&path_data, &path_config, &path_cache)),
            Arc::new(LutrisFlatpak::new(&path_home, &path_config, &path_cache)),
            Arc::new(Bottles::new(&path_data)),
            Arc::new(BottlesFlatpak::new(&path_home)),
            Arc::new(MinecraftPrism::new(&path_data)),
            Arc::new(MinecraftPrismFlatpak::new(&path_home)),
            Arc::new(MinecraftAT::new(&path_data)),
            Arc::new(MinecraftATFlatpak::new(&path_home)),
            Arc::new(Itch::new(&path_config)),
            Arc::new(ItchFlatpak::new(&path_home)),
            Arc::new(Faugus::new(&path_data)),
            Arc::new(FaugusFlatpak::new(&path_home)),
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
            .flatten()
            .collect()
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
            .filter_map(|l| {
                if let Ok(g) = l.get_detected_games() {
                    Some((l.get_launcher_type(), g))
                } else {
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
        let launchers = {
            let mut l = self.get_detected_launchers();
            l.retain(|l| l.get_launcher_type() == launcher_type);
            if l.is_empty() { return None } else { l }
        };

        Some(launchers
            .into_iter()
            .filter_map(|launcher| {
                launcher.get_detected_games()
                    .map_err(|e| {
                        #[cfg(debug_assertions)]
                        dbg!(&e);
                        error!(
                            "Launcher detected but there was an error with getting detected games for the launcher: {e:#?}"
                        )
                    })
                    .ok()
            })
            .flatten()
            .collect())
    }
}

// Test utils
#[cfg(test)]
pub mod test_utils {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use crate::data::Game;

    pub fn get_mock_file_system_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/file_system_mocks/linux")
    }

    pub fn assert_cmd(game: &Game, cmd: &str) {
        assert_eq!(game.launch_command.get_program().to_str(), Some(cmd))
    }

    pub fn assert_cmd_flatpak(game: &Game) {
        assert_cmd(game, "flatpak");
    }

    pub fn assert_cmd_xdg(game: &Game) {
        assert_cmd(game, "xdg-open");
    }
}
