use std::path::{Path, PathBuf};

use super::{DbData, LAUNCHER, get_db_data};
use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    macros::logs::debug_path,
};

#[derive(Debug)]
pub struct ItchFlatpak {
    path_butler_db: PathBuf,
}

impl ItchFlatpak {
    pub fn new(path_home: &Path) -> Self {
        let path_flatpak = path_home.join(".var/app/io.itch.itch");
        let path_config_itch = path_flatpak.join("config/itch");
        let path_butler_db = path_config_itch.join("db").join("butler.db");

        debug_path!("butler DB file", path_butler_db);

        Self { path_butler_db }
    }
}

impl Launcher for ItchFlatpak {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_butler_db.is_file()
    }

    fn get_detected_games(&self) -> GamesResult {
        let games = get_db_data(&self.path_butler_db)?
            .map(DbData::into_game)
            .collect::<Vec<Game>>();

        Ok(games)
    }
}

#[cfg(test)]
mod test {
    use super::{super::test_utils::check_itch_launcher, *};
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

    #[test]
    fn test_itch_launcher() -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = ItchFlatpak::new(&path_file_system_mock);

        check_itch_launcher(launcher, true)
    }
}
