//! Types and traits used by this crate.

use std::{
    fmt::{self, Debug, Display, Formatter},
    path::PathBuf,
    process::Command,
    sync::Arc,
};

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};

use crate::error::GamesParsingError;

/// Serialize a type into a string using the debug output
#[cfg(feature = "serde")]
fn serialize_debug<S, T>(x: &T, s: S) -> Result<S::Ok, S::Error>
where
    T: Debug,
    S: Serializer,
{
    s.serialize_str(&format!("{x:?}"))
}

/// Data structure which defines all relevant data about any particular game
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Game {
    /// Game title / name.
    pub title: String,
    /// Path to the game's icon (if one was found).
    pub path_icon: Option<PathBuf>,
    /// Path to the game's box art image (if one was found).
    pub path_box_art: Option<PathBuf>,
    /// Path to the game's root directory (if one was found).
    pub path_game_dir: Option<PathBuf>,

    /// Command to launch the game.
    // NOTE: serialized output can be `sh -c "$launch_command"`
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_debug"))]
    pub launch_command: Command,

    /// Game detection source.
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_debug"))]
    pub source: SupportedLaunchers,
}

/// Data structure representing a supported games source
#[derive(Clone, PartialEq, Eq)]
pub enum SupportedLaunchers {
    /// Regular Steam games
    Steam,
    /// Non-Steam games added to Steam manually by the user
    SteamShortcuts,
    /// Lutris games
    Lutris,
    /// Bottles games
    Bottles,
    /// Heroic Games Launcher - Amazon Prime games
    HeroicGamesAmazon,
    /// Heroic Games Launcher - Epic Games Store games
    HeroicGamesEpic,
    /// Heroic Games Launcher - GOG games
    HeroicGamesGOG,
    /// Heroic Games Launcher - games added manually by the user
    HeroicGamesSideload,
    /// Minecraft instances managed by Prism
    MinecraftPrism,
    /// Minecraft instances managed by ATLauncher
    MinecraftAT,
    /// Itch.io games
    Itch,
}

/// Custom Result type for Games
pub type GamesResult = Result<Vec<Game>, GamesParsingError>;

impl Debug for SupportedLaunchers {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SupportedLaunchers::Steam => "Steam",
                SupportedLaunchers::SteamShortcuts => "Steam (shortcuts)",
                SupportedLaunchers::HeroicGamesAmazon =>
                    "Heroic Games Launcher (Amazon Prime Gaming)",
                SupportedLaunchers::HeroicGamesEpic => "Heroic Games Launcher (Epic Games Store)",
                SupportedLaunchers::HeroicGamesGOG => "Heroic Games Launcher (GOG)",
                SupportedLaunchers::HeroicGamesSideload => "Heroic Games Launcher (Sideload)",
                SupportedLaunchers::Lutris => "Lutris",
                SupportedLaunchers::Bottles => "Bottles",
                SupportedLaunchers::MinecraftPrism => "Prism Launcher",
                SupportedLaunchers::MinecraftAT => "ATLauncher",
                SupportedLaunchers::Itch => "Itch",
            }
        )
    }
}

impl Display for SupportedLaunchers {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

// Game detection is divided up by "launchers" which are just specific sources of games
// e.g. Steam, Heroic Games Launcher, etc.
/// Source of games, e.g. Steam, Heroic Games Launcher.
pub trait Launcher: Send + Debug {
    /// Returns the [`SupportedLaunchers`] variant of this launcher.
    fn get_launcher_type(&self) -> SupportedLaunchers;
    /// Returns `true` if this source is detected on the user's system.
    fn is_detected(&self) -> bool;
    /// Get all games detected from this source.
    fn get_detected_games(&self) -> GamesResult;
}
/// Container for [`Launcher`].
pub type Launchers = Vec<Arc<dyn Launcher>>;
/// Container for games divided by their source [`SupportedLaunchers`].
pub type GamesPerLauncher = Vec<(SupportedLaunchers, Vec<Game>)>;

/// Defines methods for a detector which will be used for parsing launchers and games from those
/// launchers.
pub trait GamesDetector {
    /// Returns all detected launchers.
    fn get_detected_launchers(&self) -> Launchers;
    /// Returns all detected games from all detected launchers.
    fn get_all_detected_games(&self) -> Vec<Game>;
    /// Returns all detected games from all detected launchers, which also have detected box art.
    fn get_all_detected_games_with_box_art(&self) -> Vec<Game>;
    /// Returns all detected games divided by their source launchers.
    fn get_all_detected_games_per_launcher(&self) -> GamesPerLauncher;
    /// Returns all detected games from a specific launcher, identified by [`SupportedLaunchers`].
    fn get_all_detected_games_from_specific_launcher(
        &self,
        launcher_type: SupportedLaunchers,
    ) -> Option<Vec<Game>>;
}
