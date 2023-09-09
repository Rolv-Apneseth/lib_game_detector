use log::{debug, error, trace, warn};
use nom::{
    bytes::complete::{tag, take_till},
    character::is_alphanumeric,
    sequence::delimited,
    IResult,
};
use std::{
    fs::{read_dir, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use crate::{
    data::{Game, Launcher, SupportedLaunchers},
    utils::{clean_game_title, parse_double_quoted_value, some_if_dir, some_if_file},
};

struct ParsableManifestData {
    appid: String,
    title: String,
    game_dir: String,
}

pub struct Steam {
    path_steam_dir: PathBuf,
}

impl Steam {
    pub fn new(path_home: &Path) -> Self {
        let path_steam_dir = path_home.join(".local/share/Steam");
        debug!("Steam dir path exists: {}", path_steam_dir.is_dir());

        Steam { path_steam_dir }
    }

    /// Get all available steam libraries by parsing the `libraryfolders.vdf` file
    pub fn get_steam_libraries(&self) -> Result<Vec<SteamLibrary>, io::Error> {
        let libraries_vdg_path = self.path_steam_dir.join("steamapps/libraryfolders.vdf");

        debug!("Steam libraryfolders.vdf path: {libraries_vdg_path:?}");

        Ok(BufReader::new(File::open(libraries_vdg_path)?)
            .lines()
            .flatten()
            .filter_map(|line| {
                parse_double_quoted_value(&line, "path")
                    .ok()
                    .map(|(_, library_path)| SteamLibrary {
                        path_library: PathBuf::from(library_path),
                        path_steam_dir: &self.path_steam_dir,
                    })
            })
            .collect())
    }
}

impl Launcher for Steam {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        SupportedLaunchers::Steam
    }

    fn is_detected(&self) -> bool {
        self.path_steam_dir.is_dir()
    }

    fn get_detected_games(&self) -> Result<Vec<Game>, ()> {
        let mut steam_games = Vec::new();

        let libraries = self
            .get_steam_libraries()
            .map_err(|e| error!("Error with parsing steam libraries:\n{e}"))?;

        for library in libraries {
            steam_games.append(&mut library.get_all_games().map_err(|e| {
                error!(
                "Error with parsing games from a steam library.\nLibrary: {library:?}\nError: {e:?}"
            )
            })?);
        }

        if steam_games.is_empty() {
            warn!("No games found for any steam library")
        }

        Ok(steam_games)
    }
}

// STEAM LIBRARY ------------------------------------------------------------------------
#[derive(Debug)]
pub struct SteamLibrary<'steamlibrary> {
    path_library: PathBuf,
    path_steam_dir: &'steamlibrary Path,
}
impl<'steamlibrary> SteamLibrary<'steamlibrary> {
    /// Used for checking if a file name matches the structure for an app manifest file
    fn parse_manifest_filename<'b>(&self, filename: &'b str) -> IResult<&'b str, &'b str> {
        delimited(
            tag("appmanifest_"),
            take_till(|a| !is_alphanumeric(a as u8)),
            tag(".acf"),
        )(filename)
    }

    /// Find and return paths of the app manifest files, if they exist
    fn get_manifest_paths(&self) -> Result<Vec<PathBuf>, io::Error> {
        Ok(read_dir(self.path_library.join("steamapps"))?
            .flatten()
            .filter_map(|path| {
                let filename_os_str = path.file_name();

                let Some(filename) = filename_os_str.to_str() else {
                debug!("Could not convert OS string to str: {filename_os_str:?}");
                return None;
            };

                if self.parse_manifest_filename(filename).is_err() {
                    trace!(
                    "File skipped as it did not match the pattern of a manifest file: {filename}"
                );
                    return None;
                };

                Some(path.path())
            })
            .collect())
    }

    /// Get all steam games associated with this library
    pub fn get_all_games(&self) -> Result<Vec<Game>, io::Error> {
        let manifest_paths = self.get_manifest_paths()?;
        if manifest_paths.is_empty() {
            warn!(
                "No app manifest files found for steam library: {:?}",
                self.path_library
            );
        };

        Ok(manifest_paths
            .into_iter()
            .filter_map(|path| self.get_game(path))
            .collect())
    }

    /// Returns a new Game from the given path to a steam app manifest file (`appmanifest_.*.acf`)
    fn get_game(&self, path_app_manifest: PathBuf) -> Option<Game> {
        let ParsableManifestData {
            appid,
            title,
            game_dir,
        } = self.parse_game_manifest(path_app_manifest)?;

        let path_game_dir = some_if_dir(
            self.path_library
                .join("steamapps")
                .join("common")
                .join(game_dir),
        );
        trace!("Steam - Game directory found for '{title}': {path_game_dir:?}");

        let launch_command = format!("steam steam://rungameid/{appid}");

        let path_box_art = some_if_file(
            self.path_steam_dir
                .join(format!("appcache/librarycache/{appid}_library_600x900.jpg")),
        );

        // Skip entries without box art as they are not games (runtimes, redistributables, etc.)
        if path_box_art.is_none() {
            debug!("Skipped steam title as no box art exists for it: {title:?}");
            return None;
        }

        Some(Game {
            title,
            launch_command,
            path_box_art,
            path_game_dir,
        })
    }

    /// Parse data from the app manifest file
    fn parse_game_manifest(&self, path_app_manifest: PathBuf) -> Option<ParsableManifestData> {
        let manifest_file = File::open(&path_app_manifest)
            .map_err(|e| {
                error!("Error with reading app manifest file at {path_app_manifest:?}:\n{e}");
            })
            .ok()?;

        let mut opt_appid = None;
        let mut opt_title = None;
        let mut opt_game_dir = None;

        for line in BufReader::new(manifest_file).lines().flatten() {
            if opt_appid.is_none() {
                opt_appid = parse_double_quoted_value(&line, "appid")
                    .map(|(_, t)| t)
                    .ok();
            } else if opt_title.is_none() {
                opt_title = parse_double_quoted_value(&line, "name")
                    .map(|(_, t)| t)
                    .ok();
            } else if opt_game_dir.is_none() {
                opt_game_dir = parse_double_quoted_value(&line, "installdir")
                    .map(|(_, t)| t)
                    .ok();
            } else {
                break;
            }
        }

        let Some(appid) = opt_appid else {
            debug!("No appid could be parsed from app manifest file at: {path_app_manifest:?}");
            return None;
        };
        let Some(title) = opt_title else {
            debug!("No title could be parsed from app manifest file at: {path_app_manifest:?}");
            return None;
        };
        let Some(game_dir) = opt_game_dir else {
            debug!("No install directory could be parsed from app manifest file at: {path_app_manifest:?}");
            return None;
        };

        Some(ParsableManifestData {
            appid,
            title: clean_game_title(&title),
            game_dir,
        })
    }
}
