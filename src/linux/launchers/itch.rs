// PATHS:
// - ~/.config/itch/db/butler.db
// - ~/.var/app/io.itch.itch/config/itch/db/butler.db
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use nom::IResult;
use rusqlite::{OpenFlags, fallible_iterator::FallibleIterator, params};
use tracing::error;

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    macros::logs::{debug_fallback_flatpak, debug_path},
    parsers::parse_value_json,
    utils::clean_game_title,
};

const BUTLER_DB_QUERY: &str = "\
    SELECT g.title, g.url, g.cover_url, il.path as base_path, c.id as caves_id, c.verdict \
    FROM caves c, games g, install_locations il \
    WHERE g.id == c.game_id and il.id == c.install_location_id;\
";

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Itch;

/// Data returned directly by the query to the Butler DB
#[derive(Debug, Clone, PartialEq, Eq)]
struct DbRow {
    pub game_title: String,
    pub _game_url: String,
    pub _game_cover: String,
    pub install_locations_base_path: PathBuf,
    pub caves_id: String,
    pub caves_verdict: String,
}

impl<'stmt> TryFrom<&rusqlite::Row<'stmt>> for DbRow {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            game_title: row.get("title")?,
            _game_url: row.get("url")?,
            _game_cover: row.get("cover_url")?,
            install_locations_base_path: PathBuf::from(row.get::<&str, String>("base_path")?),
            caves_id: row.get("caves_id")?,
            caves_verdict: row.get("verdict")?,
        })
    }
}

/// Formatted, useful data built from [`DbRow`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct DbData {
    title: String,
    path_game_dir: PathBuf,
    path_bin: PathBuf,
    interpreter: Option<String>,
}

impl DbData {
    /// Build [`DbData`] from [`DbRow`].
    fn from_db_row(row: DbRow) -> Result<Self, nom::Err<nom::error::Error<std::string::String>>> {
        let title = clean_game_title(row.game_title);

        let (_, parsed_verdict) =
            ParsedVerdict::from_verdict_str(&row.caves_verdict).map_err(|e| {
                tracing::error!("{LAUNCHER} - failed to parse verdict for '{title}': {e}");
                tracing::error!("{LAUNCHER} - verdict for '{title}': {}", row.caves_verdict);
                e.to_owned()
            })?;

        let path_game_dir = PathBuf::from(parsed_verdict.game_dir);
        let path_bin = path_game_dir.join(parsed_verdict.bin);

        Ok(Self {
            title,
            path_game_dir,
            path_bin,
            interpreter: parsed_verdict.interpreter,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVerdict {
    game_dir: String,
    bin: String,
    interpreter: Option<String>,
}

impl ParsedVerdict {
    fn from_verdict_str(verdict: &str) -> IResult<&str, Self> {
        tracing::trace!("{LAUNCHER} - parsing game verdict: {verdict}",);

        // GAME DIR
        let key_game_dir = "basePath";
        let (verdict, path_game_dir) = parse_value_json(verdict, key_game_dir)?;

        // PATH
        let key_bin = "path";
        let (verdict, path_bin) = parse_value_json(verdict, key_bin)?;

        // INTERPRETER
        let key_interpreter = "interpreter";
        let (verdict, interpreter) = match parse_value_json(verdict, key_interpreter) {
            Ok((v, i)) => (v, Some(i)),
            Err(_) => (verdict, None),
        };

        Ok((
            verdict,
            Self {
                game_dir: path_game_dir,
                bin: path_bin,
                interpreter,
            },
        ))
    }
}

#[derive(Debug)]
pub struct Itch {
    path_butler_db: PathBuf,
    #[allow(dead_code)]
    is_using_flatpak: bool,
}

impl Itch {
    pub fn new(path_home: &Path, path_config: &Path) -> Self {
        let mut path_config_itch = path_config.join("itch");
        let mut is_using_flatpak = false;

        if !path_config_itch.is_dir() {
            is_using_flatpak = true;
            debug_fallback_flatpak!();

            let path_flatpak = path_home.join(".var/app/io.itch.itch");
            path_config_itch = path_flatpak.join("config/itch");
        }

        let path_butler_db = path_config_itch.join("db").join("butler.db");

        debug_path!("butler DB file", path_butler_db);

        Self {
            path_butler_db,
            is_using_flatpak,
        }
    }
}

impl Launcher for Itch {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_butler_db.is_file()
    }

    fn get_detected_games(&self) -> GamesResult {
        let conn = rusqlite::Connection::open_with_flags(
            self.path_butler_db.as_path(),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .inspect_err(|e| error!("{LAUNCHER} - failed to open the butler DB: {e}"))?;

        let mut stmt = conn
            .prepare(BUTLER_DB_QUERY)
            .inspect_err(|e| error!("{LAUNCHER} - failed to prepare DB query: {e}"))?;

        let db_rows = stmt
            .query(params![])
            .inspect_err(|e| error!("{LAUNCHER} - failed to execute DB query: {e}"))?
            .map(|r| DbRow::try_from(r))
            .collect::<Vec<DbRow>>()?;

        let db_data = db_rows
            .into_iter()
            .filter_map(|r| DbData::from_db_row(r).ok());

        let games = db_data
            .map(
                |DbData {
                     title,
                     path_game_dir,
                     path_bin,
                     interpreter,
                 }| {
                    // TODO: itch CLI to launch game using cave ID, if the following PR gets
                    // merged: <https://github.com/itchio/itch/pull/3069>
                    let launch_command = if let Some(interpreter) = interpreter {
                        let mut cmd = Command::new(interpreter);
                        cmd.arg(path_bin);
                        cmd
                    } else {
                        Command::new(path_bin)
                    };

                    // TODO: use `some_if_dir` and `some_if_file` when there is a better testing
                    // setup. Don't want to edit the test DB files to point to paths that exist.

                    Game {
                        title,
                        path_icon: None,
                        path_box_art: None,
                        path_game_dir: Some(path_game_dir),
                        launch_command,
                        source: LAUNCHER,
                    }
                },
            )
            .collect::<Vec<Game>>();

        Ok(games)
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test_case(
        "{\"basePath\":\"/media/main/Games/ultrakill-prelude\",\"totalSize\":189548486,\"candidates\":[{\"path\":\"Linux Test Build.x86_64\",\"depth\":1,\"flavor\":\"linux\",\"arch\":\"amd64\",\"size\":29327440}]}",
        ParsedVerdict {
            game_dir: "/media/main/Games/ultrakill-prelude".into(),
            bin: "Linux Test Build.x86_64".into(),
            interpreter: None,
        }
    )]
    #[test_case(
        "{\"basePath\":\"/media/main/Games/aottg2\",\"totalSize\":2403829342,\"candidates\":[{\"path\":\"Aottg2Linux/Aottg2Linux.x86_64\",\"depth\":2,\"flavor\":\"linux\",\"arch\":\"amd64\",\"size\":14720}]}",
        ParsedVerdict {
            game_dir: "/media/main/Games/aottg2".into(),
            bin: "Aottg2Linux/Aottg2Linux.x86_64".into(),
            interpreter: None,
        }
    )]
    #[test_case(
        "{\"basePath\":\"/home/alex/.local/share/itch/burrows\",\"totalSize\":1172312431,\"candidates\":[{\"path\":\"Burrows-0.17-pc/Burrows.sh\",\"depth\":2,\"flavor\":\"script\",\"size\":1663,\"scriptInfo\":{\"interpreter\":\"/bin/sh\"}}]}",
        ParsedVerdict {
            game_dir: "/home/alex/.local/share/itch/burrows".into(),
            bin: "Burrows-0.17-pc/Burrows.sh".into(),
            interpreter: Some("/bin/sh".into()),
        }
    )]
    #[test_case(
        "{\"basePath\":\"/home/alex/.local/share/itch/lautomne\",\"totalSize\":1063024341,\"candidates\":[{\"path\":\"lautomne-.4-pc/lautomne.sh\",\"depth\":2,\"flavor\":\"script\",\"size\":1660,\"scriptInfo\":{\"interpreter\":\"/bin/sh\"}}]}",
        ParsedVerdict {
            game_dir: "/home/alex/.local/share/itch/lautomne".into(),
            bin: "lautomne-.4-pc/lautomne.sh".into(),
            interpreter: Some("/bin/sh".into()),
        }
    )]
    fn parse_verdict_str(verdict: &str, expected: ParsedVerdict) {
        assert_eq!(
            ParsedVerdict::from_verdict_str(verdict).unwrap().1,
            expected
        );
    }

    #[test_case(false, ".config"; "standard")]
    #[test_case(true, "invalid/path"; "flatpak")]
    fn test_itch_launcher(
        is_testing_flatpak: bool,
        path_config: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Itch::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_config),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let mut games = launcher.get_detected_games()?;
        games.sort_by_key(|a| a.title.clone());

        assert_eq!(games.len(), 3);

        assert_eq!(games[0].title, "AoTTG2 - Attack on Titan Tribute Game 2");
        assert_eq!(games[1].title, "EMUUROM");
        assert_eq!(games[2].title, "ULTRAKILL Prelude");

        for game in &games {
            assert!(!game.title.is_empty());
            assert!(game.path_icon.is_none());
            assert!(game.path_box_art.is_none());
            assert!(game.path_game_dir.is_some());
        }

        Ok(())
    }
}
