use std::{
    fmt::{self, Debug, Formatter},
    path::PathBuf,
};

/// Data structure which defines all relevant data about any particular game
#[derive(Debug)]
pub struct Game {
    pub title: String,
    pub launch_command: String,
    pub path_box_art: Option<PathBuf>,
    pub path_game_dir: Option<PathBuf>,
}

#[derive(PartialEq, Eq)]
pub enum SupportedLaunchers {
    Steam,
    HeroicGamesAmazon,
    HeroicGamesEpicGames,
    HeroicGOG,
    Lutris,
}

impl Debug for SupportedLaunchers {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SupportedLaunchers::Steam => "Steam",
                SupportedLaunchers::HeroicGamesAmazon =>
                    "Heroic Games Launcher - Amazon Prime Gaming",
                SupportedLaunchers::HeroicGamesEpicGames =>
                    "Heroic Games Launcher - Epic Games Store",
                SupportedLaunchers::HeroicGOG => "Heroic Games Launcher - GOG",
                SupportedLaunchers::Lutris => "Lutris",
            }
        )
    }
}

// Game detection is divided up by "launchers" e.g. Steam games, Heroic games, etc.
pub trait Launcher {
    fn get_detected_games(&self) -> Result<Vec<Game>, ()>;
    fn is_detected(&self) -> bool;
    fn get_launcher_type(&self) -> SupportedLaunchers;
}

impl Debug for dyn Launcher {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Launcher: \n\t{:?}", self.get_launcher_type())
    }
}

pub trait GamesDetector {
    fn get_detected_launchers(&self) -> Vec<&Box<dyn Launcher>>;
    fn get_all_detected_games(&self) -> Option<Vec<Game>>;
    fn get_all_detected_games_per_launcher(&self) -> Option<Vec<(SupportedLaunchers, Vec<Game>)>>;
    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<Vec<Game>>;
}
