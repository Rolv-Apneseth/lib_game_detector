use std::{
    fmt::{self, Debug, Formatter},
    io,
    path::PathBuf,
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex},
};

use thiserror::Error;

/// Data structure which defines all relevant data about any particular game
#[derive(Debug, Clone)]
pub struct Game {
    pub title: String,
    pub path_box_art: Option<PathBuf>,
    pub path_game_dir: Option<PathBuf>,
    pub launch_command: Arc<Mutex<Command>>,
}

pub type GamesSlice = Arc<[Game]>;

/// Custom error type to be used in the custom GamesSlice Result type
#[derive(Error, Debug)]
pub enum GamesParsingError {
    #[error("IO error")]
    Io(#[from] io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Custom Result type for GamesSlice
pub type GamesResult = Result<GamesSlice, GamesParsingError>;

/// Data structure representing a supported games source
#[derive(PartialEq, Eq)]
pub enum SupportedLaunchers {
    Steam,
    HeroicGamesAmazon,
    HeroicGamesEpicGames,
    HeroicGOG,
    Lutris,
    Bottles,
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
                SupportedLaunchers::Bottles => "Bottles",
            }
        )
    }
}

// Game detection is divided up by "launchers" which are just specific sources of games
// e.g. Steam, Heroic Games Launcher, etc.
pub trait Launcher: Send + Debug {
    fn get_detected_games(&self) -> GamesResult;
    fn is_detected(&self) -> bool;
    fn get_launcher_type(&self) -> SupportedLaunchers;
}
pub type LaunchersSlice = Rc<[Arc<dyn Launcher>]>;
pub type GamesPerLauncherSlice = Arc<[(SupportedLaunchers, GamesSlice)]>;

pub trait GamesDetector {
    fn get_detected_launchers(&self) -> LaunchersSlice;
    fn get_all_detected_games(&self) -> Option<GamesSlice>;
    fn get_all_detected_games_with_box_art(&self) -> Option<GamesSlice>;
    fn get_all_detected_games_per_launcher(
        &self,
    ) -> Option<Arc<[(SupportedLaunchers, GamesSlice)]>>;
    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<GamesSlice>;
}
