// PATHS:
// - ~/.local/share/lutris/
// - ~/.config/lutris/
// - ~/.cache/lutris/
// - Flatpak: ~/.var/app/net.lutris.Lutris
pub use flatpak::LutrisFlatpak;
pub use standard::Lutris;

mod flatpak;
mod standard;

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use rusqlite::{OpenFlags, fallible_iterator::FallibleIterator, params};
use tracing::{debug, error, trace};

use crate::{
    data::{Game, SupportedLaunchers},
    error::GamesParsingError,
    macros::logs::debug_path,
    utils::{clean_game_title, get_existing_image_path, some_if_dir},
};

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Lutris;

// DB DATA --------------------------------------------------------------------------------
const PGA_DB_QUERY: &str = "\
    SELECT id, name, slug, installer_slug, parent_slug, directory, playtime, installed, runner \
    FROM games;\
";

/// Data returned directly by the query to the pga.db
#[derive(Debug, Clone)]
struct DbRow {
    run_id: String,
    runner: Option<String>,
    title: String,
    slug: String,
    installer_slug: Option<String>,
    game_dir: Option<String>,
    installed: bool,
    _parent_slug: Option<String>,
    _playtime: Option<f64>,
}

impl<'stmt> TryFrom<&rusqlite::Row<'stmt>> for DbRow {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            run_id: row.get::<&str, i32>("id")?.to_string(),
            runner: row.get("runner")?,
            title: row.get("name")?,
            slug: row.get("slug")?,
            installer_slug: row.get("installer_slug")?,
            _parent_slug: row.get("parent_slug")?,
            game_dir: row.get("directory")?,
            _playtime: row.get("playtime")?,
            installed: row.get("installed")?,
        })
    }
}

impl DbRow {
    fn into_game<F>(self, paths: &LutrisPaths, get_launch_command: F) -> Game
    where
        F: Fn(&str, [(&str, &str); 1]) -> Command,
    {
        let DbRow {
            game_dir,
            run_id,
            title,
            slug,
            installer_slug,
            ..
        } = self;

        let launch_command = {
            let env_vars = [("LUTRIS_SKIP_INIT", "1")];
            let game_run_arg = format!("lutris:rungameid/{run_id}");
            get_launch_command(game_run_arg.as_str(), env_vars)
        };

        trace!("{LAUNCHER} - launch_command: {launch_command:?}");

        let (path_box_art, path_icon, path_hero) = {
            let (mut box_art, mut icon, mut banner) = (None, None, None);
            // First, check if a file name using the slug exists
            if let Some(s) = installer_slug.as_ref() {
                box_art = get_existing_image_path(&paths.box_art_dir, s);
                banner = get_existing_image_path(&paths.banner_dir, s);
                icon = get_existing_image_path(&paths.icons_dir, format!("lutris_{s}"));
            }
            // Otherwise, fallback to using the slug
            (
                box_art.or_else(|| get_existing_image_path(&paths.box_art_dir, &slug)),
                icon.or_else(|| {
                    get_existing_image_path(&paths.icons_dir, format!("lutris_{slug}"))
                }),
                banner.or_else(|| get_existing_image_path(&paths.banner_dir, slug)),
            )
        };

        let mut path_game_dir = None;
        if let Some(gd) = game_dir {
            path_game_dir = some_if_dir(PathBuf::from(gd));
        }

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
            path_hero,
            path_header: None,
        }
    }
}

#[derive(Debug, Clone)]
struct LutrisPaths {
    pga_db: PathBuf,
    box_art_dir: PathBuf,
    banner_dir: PathBuf,
    icons_dir: PathBuf,
}

impl LutrisPaths {
    fn new(path_data: &Path, path_config: &Path, path_cache: &Path) -> Self {
        let path_data_lutris = path_data.join("lutris");
        let path_config_lutris = path_config.join("lutris");
        let path_cache_lutris = path_cache.join("lutris");

        let mut path_box_art_dir = path_data_lutris.join("coverart");
        let mut path_banner_dir = path_data_lutris.join("banners");
        let mut path_pga_db = path_data_lutris.join("pga.db");

        let mut path_icons_dir = path_data_lutris.join("icons/hicolor/128x128/apps");
        if !path_icons_dir.is_dir() {
            path_icons_dir = path_data.join("icons/hicolor/128x128/apps");
        }

        // Potential fallbacks for Lutris data dir contents
        if path_config_lutris.is_dir() && !path_pga_db.is_file() {
            debug!("{LAUNCHER} - pga.db not found at {path_pga_db:?}, using config dir fallback");
            path_box_art_dir = path_config_lutris.join("coverart");
            path_banner_dir = path_config_lutris.join("banners");
            path_pga_db = path_config_lutris.join("pga.db");
        }
        if path_cache_lutris.is_dir() && !path_pga_db.is_file() {
            debug!("{LAUNCHER} - pga.db not found at {path_pga_db:?}, using cache dir fallback");
            path_box_art_dir = path_cache_lutris.join("coverart");
            path_banner_dir = path_cache_lutris.join("banners");
            path_pga_db = path_cache_lutris.join("pga.db");
        }

        debug_path!("pga.db file", path_pga_db);
        debug_path!("box art directory", path_box_art_dir);
        debug_path!("banner directory", path_banner_dir);
        debug_path!("icons directory", path_icons_dir);

        Self {
            pga_db: path_pga_db,
            box_art_dir: path_box_art_dir,
            banner_dir: path_banner_dir,
            icons_dir: path_icons_dir,
        }
    }
}

fn get_db_data(path_pga_db: &Path) -> Result<Vec<DbRow>, GamesParsingError> {
    debug!("reading butler DB file at {}", path_pga_db.display());

    let conn = rusqlite::Connection::open_with_flags(path_pga_db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .inspect_err(|e| error!("{LAUNCHER} - failed to open pga.db: {e:#?}"))?;

    let mut stmt = conn
        .prepare(PGA_DB_QUERY)
        .inspect_err(|e| error!("{LAUNCHER} - failed to prepare DB query: {e:#?}"))?;

    stmt.query(params![])
        .inspect_err(|e| error!("{LAUNCHER} - failed to execute DB query: {e:#?}"))?
        .map(|r| DbRow::try_from(r))
        .filter(|r| Ok(r.installed))
        // WARN: Ignore Steam entries entirely.
        //       This is more of an opinion, as these lead to duplicate entries,
        //       and do not inherently belong to Lutris anyway (will still just
        //       launch through Steam). Feel free to start a discussion if you disagree.
        //       See: https://github.com/Rolv-Apneseth/lib_game_detector/issues/53
        .filter(|r| Ok(r.runner.as_ref().is_none_or(|s| s != "steam")))
        .collect::<Vec<DbRow>>()
        .map_err(|e| {
            error!("{LAUNCHER} - failed to convert DB rows: {e:#?}");
            e.into()
        })
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
    };

    pub fn check_lutris_launcher<F>(
        launcher: impl Launcher,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        assert!(launcher.is_detected());

        let mut games = launcher.get_detected_games()?;
        games.sort_unstable_by_key(|g| g.title.clone());

        assert_eq!(games.len(), 6);

        assert_cmd(&games[0]);

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
