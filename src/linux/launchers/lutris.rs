// PATHS:
// - ~/.local/share/lutris/
// - ~/.config/lutris/
// - ~/.cache/lutris/
// - Flatpak: ~/.var/app/net.lutris.Lutris
use std::path::{Path, PathBuf};

use rusqlite::{OpenFlags, fallible_iterator::FallibleIterator, params};
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    error::GamesParsingError,
    macros::logs::{debug_fallback_flatpak, debug_path, warn_no_games},
    utils::{
        clean_game_title, get_existing_image_path, get_launch_command, get_launch_command_flatpak,
        some_if_dir,
    },
};

// DB DATA --------------------------------------------------------------------------------
const PGA_DB_QUERY: &str =
    "SELECT id, name, slug, installer_slug, parent_slug, directory, playtime FROM games;";

/// Data returned directly by the query to the pga.db
#[derive(Debug, Clone)]
struct DbRow {
    run_id: String,
    title: String,
    slug: String,
    installer_slug: Option<String>,
    game_dir: String,
    _parent_slug: Option<String>,
    _playtime: Option<f64>,
}

impl<'stmt> TryFrom<&rusqlite::Row<'stmt>> for DbRow {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            run_id: row.get::<&str, i32>("id")?.to_string(),
            title: row.get("name")?,
            slug: row.get("slug")?,
            installer_slug: row.get("installer_slug")?,
            _parent_slug: row.get("parent_slug")?,
            game_dir: row.get("directory")?,
            _playtime: row.get("playtime")?,
        })
    }
}

// LUTRIS LAUNCHER -------------------------------------------------------------------------
const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Lutris;

#[derive(Debug)]
pub struct Lutris {
    path_pga_db: PathBuf,
    path_box_art_dir: PathBuf,
    path_icons_dir: PathBuf,
    is_using_flatpak: bool,
}

impl Lutris {
    pub fn new(path_home: &Path, path_config: &Path, path_cache: &Path, path_data: &Path) -> Self {
        let path_config_lutris = path_config.join("lutris");
        let path_cache_lutris = path_cache.join("lutris");
        let path_data_lutris = path_data.join("lutris");

        let mut path_box_art_dir = path_data_lutris.join("coverart");
        let mut path_pga_db = path_data_lutris.join("pga.db");
        let mut path_icons_dir = path_data_lutris.join("icons/hicolor/128x128/apps");

        // Flatpak fallback only if multiple dirs don't exist
        let mut is_using_flatpak = false;
        if !path_config_lutris.is_dir()
            && (!path_cache_lutris.is_dir() || !path_data_lutris.is_dir())
        {
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            let path_flatpak = path_home.join(".var/app/net.lutris.Lutris/data");
            path_icons_dir = path_flatpak.join("icons/hicolor/128x128/apps");
            path_box_art_dir = path_flatpak.join("lutris/coverart");
            path_pga_db = path_flatpak.join("lutris/pga.db")
        }

        // Potential fallbacks for cover art dir
        if path_config_lutris.is_dir() && !path_box_art_dir.is_dir() {
            debug!(
                "{LAUNCHER} - box art directory not found at {path_box_art_dir:?}, using .config fallback"
            );
            path_box_art_dir = path_config_lutris.join("coverart");
        }
        if path_cache_lutris.is_dir() && !path_box_art_dir.is_dir() {
            debug!(
                "{LAUNCHER} - box art directory not found at {path_box_art_dir:?}, using .cache fallback"
            );
            path_box_art_dir = path_cache_lutris.join("coverart");
        }

        debug_path!("box art directory", path_box_art_dir);
        debug_path!("icons directory", path_icons_dir);
        debug_path!("pga.db file", path_pga_db);

        Lutris {
            path_box_art_dir,
            path_icons_dir,
            path_pga_db,
            is_using_flatpak,
        }
    }

    fn get_db_data(&self) -> Result<Vec<DbRow>, GamesParsingError> {
        let conn = rusqlite::Connection::open_with_flags(
            self.path_pga_db.as_path(),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .inspect_err(|e| error!("{LAUNCHER} - failed to open pga.db: {e}"))?;

        let mut stmt = conn
            .prepare(PGA_DB_QUERY)
            .inspect_err(|e| error!("{LAUNCHER} - failed to prepare DB query: {e}"))?;

        stmt.query(params![])
            .inspect_err(|e| error!("{LAUNCHER} - failed to execute DB query: {e}"))?
            .map(|r| DbRow::try_from(r))
            .collect::<Vec<DbRow>>()
            .inspect(|rows| {
                if rows.is_empty() {
                    warn!(
                        "{LAUNCHER} - No games were parsed from the butler DB file at {:?}",
                        self.path_pga_db
                    )
                };
            })
            .map_err(|e| {
                error!("{LAUNCHER} - failed to convert DB rows: {e}");
                e.into()
            })
    }
}

impl Launcher for Lutris {
    fn is_detected(&self) -> bool {
        self.path_pga_db.is_file() && self.path_box_art_dir.is_dir()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.get_db_data()?;

        if parsed_data.is_empty() {
            warn_no_games!();
        }

        Ok(parsed_data
            .iter()
            .map(
                |DbRow {
                     game_dir,
                     run_id,
                     title,
                     slug,
                     installer_slug,
                     ..
                 }| {
                    let launch_command = {
                        let env_vars = [("LUTRIS_SKIP_INIT", "1")];
                        let game_run_arg = format!("lutris:rungameid/{run_id}");
                        let args = [game_run_arg.as_str()];
                        if self.is_using_flatpak {
                            get_launch_command_flatpak("net.lutrsi.Lutris", [], args, env_vars)
                        } else {
                            get_launch_command("lutris", args, env_vars)
                        }
                    };

                    trace!("{LAUNCHER} - launch_command: {launch_command:?}");

                    let (path_box_art, path_icon) = {
                        let (mut box_art, mut icon) = (None, None);
                        // First, check if a file name using the slug exists
                        if let Some(s) = installer_slug {
                            box_art = get_existing_image_path(&self.path_box_art_dir, s);
                            icon = get_existing_image_path(
                                &self.path_icons_dir,
                                format!("lutris_{s}"),
                            );
                        }
                        // Otherwise, fallback to using the slug
                        (
                            box_art
                                .or_else(|| get_existing_image_path(&self.path_box_art_dir, slug)),
                            icon.or_else(|| {
                                get_existing_image_path(
                                    &self.path_icons_dir,
                                    format!("lutris_{slug}"),
                                )
                            }),
                        )
                    };

                    let path_game_dir = some_if_dir(PathBuf::from(game_dir));

                    trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
                    trace!("{LAUNCHER} - Box art for '{title}': {path_box_art:?}");
                    trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");

                    Game {
                        title: clean_game_title(title),
                        launch_command,
                        path_box_art,
                        path_game_dir,
                        path_icon,
                        source: LAUNCHER.clone(),
                    }
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case(false, ".config", ".cache"; "standard")]
    #[test_case(false, "invalid/path", ".cache"; "fallback")]
    #[test_case(true, "invalid/path", "invalid/path"; "flatpak")]
    fn test_lutris_launcher(
        is_testing_flatpak: bool,
        path_config: &str,
        path_cache: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Lutris::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_config),
            &path_file_system_mock.join(path_cache),
            &path_file_system_mock.join(".local/share"),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let mut games = launcher.get_detected_games()?;
        games.sort_unstable_by_key(|g| g.title.clone());

        assert_eq!(games.len(), 6);

        assert_eq!(games[0].title, "Battle.net");
        assert_eq!(games[1].title, "Epic Games Store");
        assert_eq!(games[2].title, "Hearthstone");
        assert_eq!(games[3].title, "Heroes of the Storm");
        assert_eq!(games[4].title, "Warcraft III");
        assert_eq!(games[5].title, "Warcraft III - Frozen Throne");

        // TODO: when revamping testing setup - initialise DB with given settings
        //       and verify other fields are parsed correctly here.

        Ok(())
    }
}
