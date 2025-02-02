use std::{
    fs::{read_dir, read_to_string, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use nom::{
    bytes::complete::{tag, take_till},
    character::is_alphanumeric,
    sequence::delimited,
    IResult,
};
use tracing::{debug, error, trace, warn};
use walkdir::WalkDir;

use super::{get_steam_dir, get_steam_flatpak_dir, get_steam_launch_command};
use crate::{
    data::{Game, GamesParsingError, GamesResult, Launcher, SupportedLaunchers},
    parsers::parse_value_json,
    utils::{clean_game_title, some_if_dir, some_if_file},
};

struct ParsableManifestData {
    app_id: String,
    title: String,
    install_dir_path: String,
}

// UTILS --------------------------------------------------------------------------------
/// Used for checking if a file name matches the structure for an app manifest file
#[tracing::instrument]
fn parse_manifest_filename(filename: &str) -> IResult<&str, &str> {
    delimited(
        tag("appmanifest_"),
        take_till(|a| !is_alphanumeric(a as u8)),
        tag(".acf"),
    )(filename)
}

/// Used for parsing relevant game's data from the given app manifest file's contents
#[tracing::instrument(skip_all)]
fn parse_game_manifest(file_content: &str) -> IResult<&str, ParsableManifestData> {
    // ID
    let (file_content, app_id) = parse_value_json(file_content, "appid")?;

    // TITLE
    let (file_content, title) = parse_value_json(file_content, "name")?;

    // INSTALL_DIR_PATH
    let (file_content, install_dir_path) = parse_value_json(file_content, "installdir")?;

    Ok((
        file_content,
        ParsableManifestData {
            app_id,
            title: clean_game_title(title),
            install_dir_path,
        },
    ))
}

// STEAM LIBRARY ------------------------------------------------------------------------
#[derive(Debug)]
pub struct SteamLibrary<'steamlibrary> {
    path_library: PathBuf,
    path_steam_dir: &'steamlibrary Path,
    is_using_flatpak: bool,
}
impl SteamLibrary<'_> {
    /// Find and return paths of the app manifest files, if they exist
    #[tracing::instrument(skip(self))]
    fn get_manifest_paths(&self) -> Result<Arc<[PathBuf]>, io::Error> {
        Ok(read_dir(self.path_library.join("steamapps"))?
            .flatten()
            .filter_map(|path| {
                let filename_os_str = path.file_name();

                let Some(filename) = filename_os_str.to_str() else {
                    debug!("Could not convert OS string to str: {filename_os_str:?}");
                    return None;
                };

                if parse_manifest_filename(filename).is_err() {
                    trace!("File skipped as it did not match the pattern of a manifest file: {filename}");
                    return None;
                };

                Some(path.path())
            })
            .collect())
    }

    /// Returns a new Game from the given path to a steam app manifest file (`appmanifest_.*.acf`)
    #[tracing::instrument(skip(self))]
    fn get_game(&self, path_app_manifest: &PathBuf) -> Option<Game> {
        let file_content = read_to_string(path_app_manifest)
            .map_err(|e| {
                error!("Error with reading Steam app manifest file at {path_app_manifest:?}:\n{e}");
            })
            .ok()?;

        let (
            _,
            ParsableManifestData {
                app_id,
                title,
                install_dir_path,
            },
        ) = parse_game_manifest(&file_content).ok()?;

        let launch_command = get_steam_launch_command(&app_id, self.is_using_flatpak);

        let path_game_dir = some_if_dir(
            self.path_library
                .join("steamapps/common")
                .join(install_dir_path),
        );

        let path_box_art = {
            let box_art_file_name = "library_600x900.jpg";

            // Old library cache structure
            let mut path = some_if_file(self.path_steam_dir.join(format!(
                "appcache/librarycache/{app_id}_{box_art_file_name}"
            )));

            // In the new structure, the file is not in the root `librarycache` dir, but rather in
            // a dir named after the `app_id`.
            if path.is_none() {
                path = some_if_file(self.path_steam_dir.join(format!(
                    "appcache/librarycache/{app_id}/{box_art_file_name}"
                )));
            }

            // It can also appear in any sub-dir within that `app_id` dir, but we check the
            // above, non-nested path first to save time.
            if path.is_none() {
                path = WalkDir::new(
                    self.path_steam_dir
                        .join(format!("appcache/librarycache/{app_id}")),
                )
                .min_depth(2)
                .max_depth(2)
                .contents_first(true)
                .into_iter()
                .find_map(|res| {
                    let dir_entry = res.ok()?;
                    let file_name = dir_entry.file_name().to_str()?;

                    if file_name == box_art_file_name {
                        Some(dir_entry.path().to_owned())
                    } else {
                        None
                    }
                });
            }

            path
        };

        trace!("Steam - Game directory found for '{title}': {path_game_dir:?}");
        trace!("Steam - Box art found for '{title}': {path_box_art:?}");

        // Skip entries without box art as they are not games (runtimes, redistributables, DLC, etc.),
        // at least as far as I know
        if path_box_art.is_none() {
            trace!("Skipped steam title as no box art exists for it: {title:?}");
            return None;
        }

        Some(Game {
            title,
            launch_command,
            path_box_art,
            path_game_dir,
        })
    }

    /// Get all steam games associated with this library
    #[tracing::instrument]
    pub fn get_all_games(&self) -> Result<Vec<Game>, io::Error> {
        let manifest_paths = self.get_manifest_paths()?;

        if manifest_paths.is_empty() {
            warn!(
                "No app manifest files found for steam library: {:?}",
                self.path_library
            );
        };

        Ok(manifest_paths
            .iter()
            .filter_map(|path| self.get_game(path))
            .collect())
    }
}

// STEAM LAUNCHER -----------------------------------------------------------------------
#[derive(Debug)]
pub struct Steam {
    path_steam_dir: PathBuf,
    is_using_flatpak: bool,
}

impl Steam {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let mut path_steam_dir = get_steam_dir(path_data);
        let mut is_using_flatpak = false;

        if !path_steam_dir.is_dir() {
            is_using_flatpak = true;
            path_steam_dir = get_steam_flatpak_dir(path_home);
        };

        debug!("Steam dir path exists: {}", path_steam_dir.is_dir());

        Steam {
            path_steam_dir,
            is_using_flatpak,
        }
    }

    /// Get all available steam libraries by parsing the `libraryfolders.vdf` file
    #[tracing::instrument]
    pub fn get_steam_libraries(&self) -> Result<Vec<SteamLibrary>, io::Error> {
        let libraries_vdg_path = self.path_steam_dir.join("steamapps/libraryfolders.vdf");

        debug!("Steam libraryfolders.vdf path: {libraries_vdg_path:?}");

        Ok(BufReader::new(File::open(libraries_vdg_path)?)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| {
                parse_value_json(&line, "path")
                    .ok()
                    .map(|(_, library_path)| SteamLibrary {
                        path_library: PathBuf::from(library_path),
                        path_steam_dir: &self.path_steam_dir,
                        is_using_flatpak: self.is_using_flatpak,
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

    #[tracing::instrument(skip(self))]
    fn get_detected_games(&self) -> GamesResult {
        let libraries = self.get_steam_libraries().map_err(|e| {
            error!("Error with parsing steam libraries:\n{e}");
            e
        })?;

        let mut games = libraries
            .iter()
            .filter_map(|library| {
                library
                    .get_all_games()
                    .map_err(|e| { error!(
                        "Error with parsing games from a Steam library.\nLibrary: {library:?}\nError:
                        {e:?}"
                    )})
                    .ok()
            }).peekable();

        if games.peek().is_none() {
            return Err(GamesParsingError::Other(anyhow!(
                "No valid libraries detected."
            )));
        };

        games
            .reduce(|mut acc, e| {
                acc.extend(e.into_iter());
                acc
            })
            .ok_or_else(|| {
                GamesParsingError::Other(anyhow!(
                    "Failed to combine slices from Steam Libraries into a single slice"
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::linux::test_utils::get_mock_file_system_path;

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_steam_launcher(is_testing_flatpak: bool, path_data: &str) {
        let path_files_system_mock = get_mock_file_system_path();
        let launcher = Steam::new(
            &path_files_system_mock,
            &path_files_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        // Minor test to ensure debug formatting for `SupportedLaunchers` works as intended
        assert_eq!(format!("{:?}", launcher.get_launcher_type()), "Steam");

        let games_result = launcher.get_detected_games();

        // Library paths in `libraryfolders.vdf` mock are invalid library paths
        assert!(games_result.is_err());
        if let Err(e) = games_result {
            assert!(matches!(e, GamesParsingError::Other(_)));

            if let GamesParsingError::Other(anyhow_error) = e {
                assert_eq!(anyhow_error.to_string(), "No valid libraries detected.")
            }
        }
    }

    #[test]
    fn test_steam_libraries() -> Result<(), anyhow::Error> {
        let path_file_system_mock = get_mock_file_system_path();
        let path_steam_dir = &path_file_system_mock.join(".local/share/Steam");
        let path_libs_dir = &path_file_system_mock.join("steam_libraries");

        let libraries = [
            SteamLibrary {
                path_library: path_libs_dir.join("1"),
                path_steam_dir,
                is_using_flatpak: false,
            },
            SteamLibrary {
                path_library: path_libs_dir.join("2"),
                path_steam_dir,
                is_using_flatpak: false,
            },
        ];

        let mut games = [libraries[0].get_all_games()?, libraries[1].get_all_games()?];

        assert_eq!(games[0].len(), 3);
        assert_eq!(games[1].len(), 2);

        games[0].sort_by_key(|a| a.title.clone());
        games[1].sort_by_key(|a| a.title.clone());

        assert_eq!(games[0][0].title, "Sid Meier's Civilization V");
        assert_eq!(games[0][1].title, "Unrailed!");
        assert_eq!(games[0][2].title, "Warhammer 40,000: Speed Freeks");
        assert_eq!(games[1][0].title, "Terraria");
        assert_eq!(games[1][1].title, "Timberborn");

        assert!(games[0][2].path_box_art.as_ref().is_some_and(|p| p
            .file_name()
            .is_some_and(|f| f.to_string_lossy() == "library_600x900.jpg")));

        games.into_iter().for_each(|lib| {
            lib.into_iter().for_each(|game| {
                assert!(game.path_game_dir.is_some());
            })
        });

        Ok(())
    }
}
