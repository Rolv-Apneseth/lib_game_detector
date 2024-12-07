use std::{
    fmt::{self, Debug, Formatter},
    io,
    path::PathBuf,
    process::Command,
    sync::Arc,
};

use thiserror::Error;

/// Data structure which defines all relevant data about any particular game
#[derive(Debug)]
pub struct Game {
    pub title: String,
    pub path_box_art: Option<PathBuf>,
    pub path_game_dir: Option<PathBuf>,
    pub launch_command: Command,
}

/// Custom error type to be used in the custom `Games` Result type
#[derive(Error, Debug)]
pub enum GamesParsingError {
    #[error("IO error")]
    Io(#[from] io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Custom Result type for Games
pub type GamesResult = Result<Vec<Game>, GamesParsingError>;

/// Data structure representing a supported games source
#[derive(PartialEq, Eq)]
pub enum SupportedLaunchers {
    Steam,
    SteamShortcuts,
    HeroicGamesAmazon,
    HeroicGamesEpicGames,
    HeroicGOG,
    Lutris,
    Bottles,
    MinecraftPrism,
    MinecraftAT,
}
impl Debug for SupportedLaunchers {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SupportedLaunchers::Steam => "Steam",
                SupportedLaunchers::SteamShortcuts => "Steam Shortcuts",
                SupportedLaunchers::HeroicGamesAmazon =>
                    "Heroic Games Launcher - Amazon Prime Gaming",
                SupportedLaunchers::HeroicGamesEpicGames =>
                    "Heroic Games Launcher - Epic Games Store",
                SupportedLaunchers::HeroicGOG => "Heroic Games Launcher - GOG",
                SupportedLaunchers::Lutris => "Lutris",
                SupportedLaunchers::Bottles => "Bottles",
                SupportedLaunchers::MinecraftPrism => "Minecraft - PrismLauncher",
                SupportedLaunchers::MinecraftAT => "Minecraft - ATLauncher",
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
pub type Launchers = Vec<Arc<dyn Launcher>>;
pub type GamesPerLauncher = Vec<(SupportedLaunchers, Vec<Game>)>;

pub trait GamesDetector {
    fn get_detected_launchers(&self) -> Launchers;
    fn get_all_detected_games(&self) -> Vec<Game>;
    fn get_all_detected_games_with_box_art(&self) -> Vec<Game>;
    fn get_all_detected_games_per_launcher(&self) -> GamesPerLauncher;
    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<Vec<Game>>;
}
