// PATHS:
// - ~/.local/share/Steam/
// - Flatpak: ~/.var/app/com.valvesoftware.Steam
use std::{
    fs::{File, read_dir, read_to_string},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
};

use nom::{
    AsChar, IResult, Parser,
    bytes::complete::{tag, take_till},
    sequence::delimited,
};
use tracing::{debug, error, trace, warn};
use walkdir::WalkDir;

use super::{get_steam_dir, get_steam_flatpak_dir, get_steam_launch_command};
use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    macros::logs::{debug_fallback_flatpak, debug_path, warn_no_games},
    parsers::parse_value_json,
    utils::{clean_game_title, some_if_dir, some_if_file},
};

struct ParsableManifestData {
    app_id: String,
    title: String,
    install_dir_path: String,
}

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Steam;

// UTILS --------------------------------------------------------------------------------
/// Used for checking if a file name matches the structure for an app manifest file
#[tracing::instrument(level = "trace")]
fn matches_manifest_filename(filename: &str) -> bool {
    let parse_res: IResult<&str, &str> = delimited(
        tag("appmanifest_"),
        take_till(|a| !(a as u8).is_alphanum()),
        tag(".acf"),
    )
    .parse(filename);

    parse_res.is_ok_and(|(remainder, _match)| {
        // Check for a full match (some files end in .*.tmp)
        remainder.is_empty()
    })
}

/// Used for getting the path to the "steamapps" directory, which can be capitalised on some systems.
#[tracing::instrument(level = "trace")]
fn get_path_steamapps_dir(path_parent_dir: &Path) -> PathBuf {
    let path_steamapps_dir = path_parent_dir.join("Steamapps");

    // Use the capitalised version of directory if it exists
    if path_steamapps_dir.is_dir() {
        path_steamapps_dir
    }
    // Otherwise proceed with the default
    else {
        path_parent_dir.join("steamapps")
    }
}

/// Used for parsing relevant game's data from the given app manifest file's contents
#[tracing::instrument(level = "trace", skip(file_content))]
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
    #[tracing::instrument(level = "trace")]
    fn get_manifest_paths(&self) -> Result<Arc<[PathBuf]>, io::Error> {
        let all_paths = read_dir(get_path_steamapps_dir(&self.path_library))
            .inspect_err(|e| {
                error!(
                    "{LAUNCHER} - failed to read library directory at {:?}: {e}",
                    self.path_library
                )
            })?
            .flatten();

        let manifest_paths = all_paths
            .filter_map(|path| {
                let filename_os_str = path.file_name();

                let Some(filename) = filename_os_str.to_str() else {
                    debug!("{LAUNCHER} - Could not convert OS string to str: {filename_os_str:?}");
                    return None;
                };

                if !matches_manifest_filename(filename) {
                    trace!("{LAUNCHER} - File skipped as it did not match the pattern of a manifest file: {filename}");
                    return None;
                };

                Some(path.path())
            })
            .collect();

        Ok(manifest_paths)
    }

    /// Get the box art for a specific game, checking several different potential locations.
    #[tracing::instrument(level = "trace")]
    fn get_images(&self, app_id: &str) -> (Option<PathBuf>, Option<PathBuf>) {
        const FILENAME_1: &str = "library_600x900.jpg";
        const FILENAME_2: &str = "library_capsule.jpg";
        const ICON_FILENAME_LEN: usize = 44;

        // Old library cache structure
        let mut path_box_art = some_if_file(
            self.path_steam_dir
                .join(format!("appcache/librarycache/{app_id}_{FILENAME_1}")),
        );
        let mut path_icon = some_if_file(
            self.path_steam_dir
                .join(format!("appcache/librarycache/{app_id}_icon.jpg")),
        );

        // In newer structures, icons and box art can appear in any sub-dir within the `app_id` dir
        for res in WalkDir::new(
            self.path_steam_dir
                .join(format!("appcache/librarycache/{app_id}")),
        )
        .min_depth(1)
        .max_depth(2)
        .contents_first(true)
        {
            let Ok(dir_entry) = res else {
                continue;
            };

            let Some(filename) = dir_entry.file_name().to_str() else {
                continue;
            };

            if filename == FILENAME_1 || filename == FILENAME_2 {
                path_box_art = Some(dir_entry.path().to_owned());
            }
            // Not sure how else to parse these, as I can't find them mentioned anywhere.
            // The filenames look like: a4c7a8cce43d797c275aaf601d6855b90ba87769.jpg
            else if filename.len() == ICON_FILENAME_LEN && filename.ends_with(".jpg") {
                path_icon = Some(dir_entry.path().to_owned());
            }
        }

        (path_box_art, path_icon)
    }

    /// Returns a new Game from the given path to a steam app manifest file (`appmanifest_.*.acf`)
    #[tracing::instrument(level = "trace")]
    fn get_game(&self, path_app_manifest: &PathBuf) -> Option<Game> {
        let file_content = read_to_string(path_app_manifest)
            .map_err(|e| {
                error!("{LAUNCHER} - Error with reading Steam app manifest file at {path_app_manifest:?}:\n{e}");
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

        let (path_box_art, path_icon) = self.get_images(&app_id);

        trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
        trace!("{LAUNCHER} - Box art for '{title}': {path_box_art:?}");
        trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");

        // Skip entries without box art as they are not games (runtimes, redistributables, DLC, etc.),
        // at least as far as I know
        if path_box_art.is_none() {
            trace!("{LAUNCHER} - Skipped steam title as no box art exists for it: {title:?}");
            return None;
        }

        Some(Game {
            title,
            launch_command,
            path_box_art,
            path_game_dir,
            path_icon,
            source: LAUNCHER.clone(),
        })
    }

    /// Get all steam games associated with this library
    #[tracing::instrument(level = "trace")]
    pub fn get_all_games(&self) -> Result<Vec<Game>, io::Error> {
        let manifest_paths = self.get_manifest_paths()?;

        if manifest_paths.is_empty() {
            warn!(
                "{LAUNCHER} - No app manifest files found for steam library: {:?}",
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
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            path_steam_dir = get_steam_flatpak_dir(path_home);
        };

        debug_path!("main Steam directory", path_steam_dir);

        Steam {
            path_steam_dir,
            is_using_flatpak,
        }
    }

    /// Get all available steam libraries by parsing the `libraryfolders.vdf` file
    #[tracing::instrument(level = "trace")]
    pub fn get_steam_libraries(&self) -> Result<Vec<SteamLibrary<'_>>, io::Error> {
        let libraries_vdg_path =
            get_path_steamapps_dir(&self.path_steam_dir).join("libraryfolders.vdf");

        debug_path!("libraryfolders.vdf", libraries_vdg_path);

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
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_steam_dir.is_dir()
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let libraries = self.get_steam_libraries().map_err(|e| {
            error!("{LAUNCHER} - Error with parsing steam libraries:\n{e}");
            e
        })?;

        if libraries.is_empty() {
            warn!("{LAUNCHER} - No valid libraries");
            return Ok(Default::default());
        }

        debug!("{LAUNCHER} - libraries detected: {:?}", libraries);

        let games = libraries
            .into_iter()
            .filter_map(|l| {
                let games = l.get_all_games().ok()?;

                trace!(
                    "{LAUNCHER} - games for library at {:?}: {:?}",
                    l.path_library, games
                );

                Some(games)
            })
            .collect::<Vec<_>>();

        if games.is_empty() {
            warn_no_games!();
        }

        Ok(games.into_iter().flatten().collect())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;
    use crate::{error::GamesParsingError, linux::test_utils::get_mock_file_system_path};

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
            assert!(matches!(e, GamesParsingError::Io(_)));

            if let GamesParsingError::Other(anyhow_error) = e {
                assert_eq!(anyhow_error.to_string(), "No valid libraries detected.")
            }
        }
    }

    #[test]
    fn test_steam_libraries() -> Result<(), GamesParsingError> {
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
        assert_eq!(games[1].len(), 3);

        games[0].sort_by_key(|a| a.title.clone());
        games[1].sort_by_key(|a| a.title.clone());

        assert_eq!(games[0][0].title, "Sid Meier's Civilization V");
        assert_eq!(games[0][1].title, "Unrailed!");
        assert_eq!(games[0][2].title, "Warhammer 40,000: Speed Freeks");
        assert_eq!(games[1][0].title, "Marvel Rivals");
        assert_eq!(games[1][1].title, "Terraria");
        assert_eq!(games[1][2].title, "Timberborn");

        assert!(games[0][0].path_icon.is_some());
        assert!(games[0][1].path_icon.is_none());
        assert!(games[0][2].path_icon.is_some());
        assert!(games[1][0].path_icon.is_none());
        assert!(games[1][1].path_icon.is_none());
        assert!(games[1][2].path_icon.is_none());

        assert!(games[0][2].path_box_art.as_ref().is_some_and(|p| {
            p.file_name()
                .is_some_and(|f| f.to_string_lossy() == "library_600x900.jpg")
        }));

        games.into_iter().for_each(|lib| {
            lib.into_iter().for_each(|game| {
                assert!(game.path_game_dir.is_some());
            })
        });

        Ok(())
    }
}
