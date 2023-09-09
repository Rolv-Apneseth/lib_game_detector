use std::process::exit;

use crate::{
    data::{Game, GamesDetector, Launcher, SupportedLaunchers},
    linux::launchers::{heroic_amazon::HeroicAmazon, heroic_epic::HeroicEpic},
    linux::launchers::{heroic_gog::HeroicGOG, lutris::Lutris},
};
use directories::BaseDirs;
use log::error;

use self::launchers::steam::Steam;

mod launchers;

pub struct GamesDetectorLinux {
    launchers: Vec<Box<dyn Launcher>>,
}

impl GamesDetectorLinux {
    pub fn new() -> GamesDetectorLinux {
        let Some(user_dirs) = BaseDirs::new() else {
            error!("No valid $HOME directory found for the current user");
            exit(1);
        };

        let launchers = GamesDetectorLinux::get_launchers(&user_dirs);

        GamesDetectorLinux { launchers }
    }

    pub fn get_launchers(base_dirs: &BaseDirs) -> Vec<Box<dyn Launcher>> {
        let path_home = base_dirs.home_dir();
        let path_config = base_dirs.config_dir();
        let path_cache = base_dirs.cache_dir();

        let path_heroic_config = path_config.join("heroic");

        vec![
            Box::new(Steam::new(path_home)),
            Box::new(HeroicGOG::new(&path_heroic_config)),
            Box::new(HeroicEpic::new(&path_heroic_config)),
            Box::new(HeroicAmazon::new(&path_heroic_config)),
            Box::new(Lutris::new(path_config, path_cache)),
        ]
    }
}

impl GamesDetector for GamesDetectorLinux {
    fn get_detected_launchers(&self) -> Vec<&Box<dyn Launcher>> {
        self.launchers.iter().filter(|l| l.is_detected()).collect()
    }

    fn get_all_detected_games(&self) -> Option<Vec<Game>> {
        self.get_detected_launchers()
            .iter()
            .filter_map(|l| l.get_detected_games().ok())
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
    }

    fn get_all_detected_games_per_launcher(&self) -> Option<Vec<(SupportedLaunchers, Vec<Game>)>> {
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
            .collect::<Vec<(SupportedLaunchers, Vec<Game>)>>();

        if categorised_games.is_empty() {
            return None;
        };

        Some(categorised_games)
    }

    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<Vec<Game>> {
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
