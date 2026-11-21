pub use flatpak::HeroicEpicFlatpak;
pub use standard::HeroicEpic;

mod flatpak;
mod standard;

use std::{
    io::{self},
    path::{Path, PathBuf},
};

use tracing::{error, trace, warn};

use super::ParsableLibraryData;
use crate::{
    data::SupportedLaunchers,
    linux::launchers::heroic::{
        get_heroic_config_path, get_heroic_config_path_flatpak, parse_all_games_from_library_common,
    },
    macros::logs::{debug_path, warn_no_games},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::HeroicGamesEpic;

#[derive(Debug)]
pub struct HeroicEpicPaths {
    legendary_library: PathBuf,
    icons: PathBuf,
}

impl HeroicEpicPaths {
    fn _new(path_heroic_config: &Path) -> Self {
        let legendary_library = path_heroic_config.join("store_cache/legendary_library.json");
        let icons = path_heroic_config.join("icons");

        debug_path!("Legendary library JSON file", legendary_library);
        debug_path!("Icons dir", icons);

        Self {
            legendary_library,
            icons,
        }
    }

    fn new_standard(path_config: &Path) -> Self {
        let path_heroic_config = get_heroic_config_path(path_config);
        Self::_new(&path_heroic_config)
    }

    fn new_flatpak(path_home: &Path) -> Self {
        let path_heroic_config = get_heroic_config_path_flatpak(path_home);
        Self::_new(&path_heroic_config)
    }
}

/// Parse all relevant games' data from `legendary_library.json`
#[tracing::instrument(level = "trace")]
fn parse_legendary_library(
    path_legendary_library: &Path,
) -> Result<Vec<ParsableLibraryData>, io::Error> {
    trace!(
        "{LAUNCHER} - Parsing Legendary library file at {}",
        path_legendary_library.display()
    );

    parse_all_games_from_library_common(path_legendary_library)
        .inspect(|data| {
            if data.is_empty() {
                warn_no_games!();
            };
        })
        .inspect_err(|_| {
            error!(
                "Error parsing the Heroic Games Legendary library file at: {}",
                path_legendary_library.display()
            );
        })
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
    };

    pub fn check_heroic_epic_launcher<F>(
        launcher: impl Launcher,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;
        assert_eq!(games.len(), 2);

        assert_cmd(&games[0]);

        assert_eq!(games[0].title, "Fall Guys");
        assert_eq!(games[1].title, "Rocket League");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_none());
        assert!(games[1].path_box_art.is_some());

        assert!(games.iter().all(|g| g.path_icon.is_none()));

        assert!(games.iter().all(|g| g.path_icon.is_none()));

        Ok(())
    }
}
