use std::{
    fs::{read, read_dir, read_to_string},
    mem,
    path::{Path, PathBuf},
};

use nom::{
    bytes::complete::{take_till, take_until},
    character::complete::char,
    sequence::delimited,
    IResult,
};
use steam_shortcuts_util::parse_shortcuts;
use tracing::{error, trace, warn};

use super::{get_steam_dir, get_steam_flatpak_dir, get_steam_launch_command};
use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    error::GamesParsingError,
    macros::logs::{debug_fallback_flatpak, debug_path, warn_no_games},
    parsers::{parse_between_double_quotes, parse_not_double_quote},
    utils::{clean_game_title, get_existing_image_path},
};

/// Data parseable from a Steam user's `shortcuts.vdf`
#[derive(Debug, Clone, Default)]
pub struct ParsableShortcutData {
    box_art_id: String,
    title: String,
}

/// Data parseable from a Steam user's `screenshots.vdf`
#[derive(Debug, Clone, Default)]
pub struct ParsableScreenshotData {
    title: String,
    app_id: String,
}

/// Combined parsed data for a Steam shortcut / non-Steam game
#[derive(Debug, Clone)]
pub struct ParsableDataCombined {
    title: String,
    app_id: String,
    path_box_art: Option<PathBuf>,
}
impl ParsableDataCombined {
    fn combine(
        path_box_art_dir: &Path,
        shortcut_data: ParsableShortcutData,
        screenshot_data: ParsableScreenshotData,
    ) -> Self {
        // Regular Steam shortcut images have an extra "p" at the end of the image file names,
        // whereas the flathub Steam ones don't.
        let path_box_art =
            get_existing_image_path(path_box_art_dir, format!("{}p", shortcut_data.box_art_id))
                .or_else(|| get_existing_image_path(path_box_art_dir, &shortcut_data.box_art_id));

        ParsableDataCombined {
            title: shortcut_data.title,
            app_id: screenshot_data.app_id.clone(),
            path_box_art,
        }
    }
}

/// Paths to the files required for parsing all Steam shortcut data
#[derive(Debug)]
pub struct UserDataFiles {
    path_shortcuts: PathBuf,
    path_screenshots: PathBuf,
    path_box_art_dir: PathBuf,
}

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::SteamShortcuts;

// UTILS -----------------------------------------------------------------------------------------
#[tracing::instrument(level = "trace")]
fn find_userdata_files(
    path_steam_userdata_dir: &Path,
) -> Result<Vec<UserDataFiles>, GamesParsingError> {
    Ok(read_dir(path_steam_userdata_dir)?
        .flatten()
        .filter_map(|p| {
            if !p.file_type().is_ok_and(|f| f.is_dir()) {
                return None;
            }

            let p = p.path();
            let path_config = p.join("config");

            let path_screenshots = p.join("760").join("screenshots.vdf");
            if !path_screenshots.is_file() {
                trace!(
                    "{LAUNCHER} - Couldn't find Steam user screenshots file at {path_screenshots:?}"
                );
                return None;
            }

            let path_shortcuts = path_config.join("shortcuts.vdf");
            if !path_shortcuts.is_file() {
                trace!(
                    "{LAUNCHER} - Couldn't find Steam user shortcuts file at {path_shortcuts:?}"
                );
                return None;
            }

            let path_box_art_dir = path_config.join("grid");
            if !path_box_art_dir.is_dir() {
                trace!(
                    "{LAUNCHER} - Couldn't find Steam user shortcuts box art directory at {path_shortcuts:?}"
                );
                return None;
            }

            Some(UserDataFiles {
                path_shortcuts,
                path_screenshots,
                path_box_art_dir,
            })
        })
        .collect())
}

#[tracing::instrument(level = "trace")]
fn get_parsable_shortcuts_data(
    path_shortcuts: &Path,
) -> Result<Vec<ParsableShortcutData>, GamesParsingError> {
    let content = read(path_shortcuts)?;
    let shortcuts = parse_shortcuts(content.as_slice()).map_err(GamesParsingError::Other)?;

    Ok(shortcuts
        .into_iter()
        .map(|s| ParsableShortcutData {
            title: s.app_name.to_string(),
            box_art_id: s.app_id.to_string(),
        })
        .collect())
}

#[tracing::instrument(level = "trace")]
fn get_parsable_screenshots_data(
    path_screenshots: &Path,
) -> Result<Vec<ParsableScreenshotData>, GamesParsingError> {
    let file_content = &read_to_string(path_screenshots)?;

    Ok(parse_screenshots_vdf(file_content, path_screenshots)?.1)
}

#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_screenshots_vdf<'a>(
    file_content: &'a str,
    file_path: &Path,
) -> IResult<&'a str, Vec<ParsableScreenshotData>> {
    let mut data = vec![];

    // Parse until "shortcutnames" and grab the next block contained by `{}`
    let (file_content, _) = take_until("\"shortcutnames\"")(file_content)?;
    let (file_content, _) = take_till(|c| c == '{')(file_content)?;
    let (file_content, mut block) =
        delimited(char('{'), take_till(|c| c == '}'), char('}'))(file_content)?;

    // Remove trailing whitespace so the below while block condition fails before running with the empty line
    block = block.trim_end();

    while let Ok((file_content, _)) = parse_not_double_quote(block) {
        // APP ID
        let (file_content, app_id) = parse_between_double_quotes(file_content)?;

        let (file_content, _) = parse_not_double_quote(file_content)?;

        // TITLE
        let (file_content, title) = parse_between_double_quotes(file_content)?;

        data.push(ParsableScreenshotData {
            title: title.to_string(),
            app_id: app_id.to_string(),
        });

        block = file_content;
    }

    Ok((file_content, data))
}

// STEAM SHORTCUTS / NON-STEAM GAMES ---------------------------------------------------------------
#[derive(Debug)]
pub struct SteamShortcuts {
    path_steam_userdata_dir: PathBuf,
    is_using_flatpak: bool,
}

impl SteamShortcuts {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let mut path_steam_userdata_dir = get_steam_dir(path_data).join("userdata");
        let mut is_using_flatpak = false;

        if !path_steam_userdata_dir.is_dir() {
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            path_steam_userdata_dir = get_steam_flatpak_dir(path_home).join("userdata");
        };

        debug_path!("userdata dir path", path_steam_userdata_dir);

        Self {
            path_steam_userdata_dir,
            is_using_flatpak,
        }
    }

    #[tracing::instrument(level = "trace")]
    fn parse_combined_data(&self) -> Result<Option<Vec<ParsableDataCombined>>, GamesParsingError> {
        let shortcut_files = find_userdata_files(&self.path_steam_userdata_dir)?;

        // TODO: find way to know what user is logged in so we can choose the correct file
        let Some(UserDataFiles {
            path_shortcuts,
            path_screenshots,
            path_box_art_dir,
        }) = shortcut_files.into_iter().next()
        else {
            // One of the paths could not be found, no shortcuts available for the user
            return Ok(None);
        };

        let shortcuts_data = get_parsable_shortcuts_data(&path_shortcuts)?;
        let mut screenshots_data = get_parsable_screenshots_data(&path_screenshots)?;

        Ok(Some(
            shortcuts_data
                .iter()
                .cloned()
                .filter_map(|shortcut_data| {
                    screenshots_data
                        .iter_mut()
                        // Reverse because the last entry is the newest one and this file doesn't seem to
                        // get reset, so we want to take the one most likely to be correct
                        .rev()
                        .find(|d| !d.title.is_empty() && d.title == shortcut_data.title)
                        .map(|screenshot_data| {
                            ParsableDataCombined::combine(
                                &path_box_art_dir,
                                shortcut_data,
                                mem::take(screenshot_data),
                            )
                        })
                })
                .collect(),
        ))
    }
}

impl Launcher for SteamShortcuts {
    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    fn is_detected(&self) -> bool {
        self.path_steam_userdata_dir.is_dir()
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let shortcut_data = self
            .parse_combined_data()
            .map_err(|e| {
                error!("{LAUNCHER} - {e}");
                e
            })?
            .unwrap_or_default();

        if shortcut_data.is_empty() {
            warn_no_games!();
        }

        Ok(shortcut_data
            .into_iter()
            .map(
                |ParsableDataCombined {
                     app_id,
                     title,
                     path_box_art,
                 }| {
                    let launch_command = get_steam_launch_command(app_id, self.is_using_flatpak);
                    let path_game_dir = None;
                    let title = clean_game_title(title);

                    trace!("{LAUNCHER} - Game directory found for '{title}': {path_game_dir:?}");
                    trace!("{LAUNCHER} - Box art found for '{title}': {path_box_art:?}");

                    Game {
                        title: clean_game_title(&title),
                        launch_command,
                        path_box_art,
                        path_game_dir,
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
    use crate::linux::test_utils::get_mock_file_system_path;

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_steam_shortcuts_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), GamesParsingError> {
        let path_files_system_mock = get_mock_file_system_path();
        let launcher = SteamShortcuts::new(
            &path_files_system_mock,
            &path_files_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;
        assert_eq!(games.len(), 3);

        assert_eq!(games[0].title, "ATLauncher");
        assert_eq!(games[1].title, "Brave");
        assert_eq!(games[2].title, "Lutris");

        assert!(games[0].path_game_dir.is_none());
        assert!(games[1].path_game_dir.is_none());
        assert!(games[2].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_some());
        assert!(games[1].path_box_art.is_some());
        assert!(games[2].path_box_art.is_none());

        Ok(())
    }
}
