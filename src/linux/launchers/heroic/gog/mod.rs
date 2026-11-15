pub use flatpak::HeroicGOGFlatpak;
pub use standard::HeroicGOG;

mod flatpak;
mod standard;

use std::{
    io::{self},
    path::{Path, PathBuf},
    process::Command,
};

use nom::IResult;
use tracing::{error, trace, warn};

use crate::{
    data::{Game, SupportedLaunchers},
    linux::launchers::heroic::{
        get_heroic_config_path, get_heroic_config_path_flatpak, parse_all_games_from_library,
    },
    macros::logs::{debug_path, warn_no_games},
    parsers::parse_value_json,
    utils::{clean_game_title, some_if_dir, some_if_file},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::HeroicGamesGOG;

#[derive(Debug)]
struct ParsableGOGInstalledData {
    app_id: String,
    install_path: String,
    title: String,
}

impl ParsableGOGInstalledData {
    fn into_game<F>(self, path_icons: &Path, get_launch_command: F) -> Game
    where
        F: Fn(&str, &str) -> Command,
    {
        let Self {
            app_id,
            install_path,
            title,
        } = self;

        let launch_command = get_launch_command("gog", &app_id);
        trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

        let path_game_dir = some_if_dir(PathBuf::from(install_path));
        let path_icon = some_if_file(path_icons.join(format!("{app_id}.png")));

        trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
        trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");

        Game {
            title,
            launch_command,
            path_game_dir,
            path_icon,
            path_box_art: None,
            source: LAUNCHER.clone(),
            path_hero: None,
            path_header: None,
        }
    }
}

#[derive(Debug)]
struct HeroicGogPaths {
    gog_installed_games: PathBuf,
    icons: PathBuf,
}

impl HeroicGogPaths {
    fn _new(path_heroic_config: &Path) -> Self {
        let gog_installed_games = path_heroic_config.join("gog_store/installed.json");
        let icons = path_heroic_config.join("icons");

        debug_path!("GOG installed games JSON file", gog_installed_games);
        debug_path!("Icons dir", icons);

        Self {
            gog_installed_games,
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

/// Utility function which parses a single game from the Heroic Games GOG store `installed.json` file
///
/// Unfortunately a separate parser function is needed for GOG's `gog_store/installed.json` file because:
/// 1. `store_cache/gog_library.json` has `is_installed` as always false
/// 2. `gog_store/library.json` is empty for some reason
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_gog_installed(file_content: &str) -> IResult<&str, ParsableGOGInstalledData> {
    // INSTALL_PATH
    let (file_content, install_path) = parse_value_json(file_content, "install_path")?;

    // ID
    let (file_content, app_id) = parse_value_json(file_content, "appName")?;

    // TITLE
    let Some(title) = install_path
        .rsplit_once('/')
        .map(|split_path| clean_game_title(split_path.1))
    else {
        return parse_game_from_gog_installed(file_content);
    };

    Ok((
        file_content,
        ParsableGOGInstalledData {
            app_id,
            title: clean_game_title(title),
            install_path,
        },
    ))
}

/// Parse all relevant games' data from GOG's `installed.json`
#[tracing::instrument]
fn parse_gog_installed(
    path_gog_installed_games: &Path,
) -> Result<Vec<ParsableGOGInstalledData>, io::Error> {
    trace!(
        "Parsing Heroic Launcher GOG installed games file at {}",
        path_gog_installed_games.display()
    );

    parse_all_games_from_library::<ParsableGOGInstalledData>(
        path_gog_installed_games,
        parse_game_from_gog_installed,
    )
    .inspect(|data| {
        if data.is_empty() {
            warn_no_games!();
        };
    })
    .inspect_err(|_| {
        error!(
            "Error parsing the Heroic Games Legendary library file at: {}",
            path_gog_installed_games.display()
        );
    })
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
    };

    pub fn check_heroic_gog_launcher<F>(
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

        assert_eq!(games[0].title, "home");
        assert_eq!(games[1].title, "Bread & Fred Demo");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());

        assert!(games[0].path_icon.is_none());
        assert!(games[1].path_icon.is_some());

        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
