// PATHS:
// - ~/.local/share/bottles/
// - Flatpak: ~/.var/app/com.usebottles.bottles/data/bottles/
use std::{
    fs::{read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use nom::{
    bytes::complete::{is_not, tag, take_till},
    character::complete::multispace1,
    sequence::preceded,
    IResult, Parser,
};
use tracing::{error, trace, warn};

use crate::{
    data::{Game, GamesResult, Launcher, SupportedLaunchers},
    macros::logs::{debug_fallback_flatpak, debug_path, warn_no_games},
    parsers::{
        parse_not_alphanumeric, parse_till_end_of_line, parse_until_key_yml, parse_value_yml,
    },
    utils::{
        clean_game_title, get_launch_command, get_launch_command_flatpak, some_if_dir, some_if_file,
    },
};

#[derive(Debug, Clone)]
pub struct ParsableLibraryData {
    id: String,
    title: String,
    icon: Option<PathBuf>,
    box_art: Option<String>,
    bottle_name: String,
    bottle_subdir: String,
}

#[derive(Debug, Clone)]
pub struct ParsableBottleYmlData {
    id: String,
    game_dir: String,
}

#[derive(Debug)]
pub struct ParsableDataCombined {
    title: String,
    path_icon: Option<PathBuf>,
    box_art: Option<String>,
    bottle_name: String,
    bottle_subdir: String,
    game_dir: String,
}

impl ParsableDataCombined {
    fn combine(library_data: ParsableLibraryData, bottle_data: ParsableBottleYmlData) -> Self {
        ParsableDataCombined {
            title: library_data.title,
            path_icon: library_data.icon,
            box_art: library_data.box_art,
            bottle_subdir: library_data.bottle_subdir,
            bottle_name: library_data.bottle_name,
            game_dir: bottle_data.game_dir,
        }
    }
}

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::Bottles;

// UTILS --------------------------------------------------------------------------------
/// Used for parsing a single game's relevant data from the given bottle `.yml` file's contents
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_bottle_yml(file_content: &str) -> IResult<&str, ParsableBottleYmlData> {
    // GAME DIR
    let key_game_dir = "folder";
    let (file_content, _) = parse_until_key_yml(file_content, key_game_dir)?;
    let (mut file_content, first_path_fragment) =
        preceded(take_till(|c| c == '/'), parse_till_end_of_line).parse(file_content)?;

    // Path can be split into multiple lines unfortunately
    let mut game_dir_fragments: Vec<&str> = vec![&first_path_fragment];
    loop {
        let (new_file_content, line) =
            preceded(tag("\n"), parse_till_end_of_line).parse(file_content)?;

        if line.contains(':') {
            break;
        };

        file_content = new_file_content;

        let (path_fragment, _) = multispace1.parse(line)?;

        game_dir_fragments.push(path_fragment);
    }

    let game_dir = game_dir_fragments.join(" ");

    // ID
    let key_id = "id";
    let (file_content, _) = parse_until_key_yml(file_content, key_id)?;
    let (file_content, id) = parse_value_yml(file_content, key_id)?;

    Ok((file_content, ParsableBottleYmlData { id, game_dir }))
}

/// Used for parsing relevant games' data from the given bottle library file's contents
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_game_from_library<'a>(file_content: &'a str) -> IResult<&'a str, ParsableLibraryData> {
    // BOTTLE NAME
    let key_bottle_name = "name";
    let (file_content, _) = parse_until_key_yml(file_content, key_bottle_name)?;
    let (file_content, bottle_name) = parse_value_yml(file_content, key_bottle_name)?;

    // BOTTLE SUBDIR
    let key_bottle_subdir = "path";
    let (file_content, _) = parse_until_key_yml(file_content, key_bottle_subdir)?;
    let (file_content, bottle_subdir) = parse_value_yml(file_content, key_bottle_subdir)?;

    // ICON
    let key_icon = "icon";
    let (file_content, _) = parse_until_key_yml(file_content, key_icon)?;
    let (file_content, _) = tag(format!("{key_icon}: ").as_str()).parse(file_content)?;
    let (mut file_content, line1) = parse_till_end_of_line(file_content)?;
    file_content = file_content.trim_start();

    let (next_file_content, line2) = parse_till_end_of_line(file_content)?;
    let (file_content, icon_str) = if line2.contains("id:") {
        (file_content, line1.to_owned())
    } else {
        (next_file_content, [line1, line2].join(" "))
    };
    let icon_path = PathBuf::from(icon_str);
    let icon = icon_path.is_file().then_some(icon_path);

    // ID
    let key_id = "id";
    let (file_content, _) = parse_until_key_yml(file_content, key_id)?;
    let (file_content, id) = parse_value_yml(file_content, key_id)?;

    // TITLE
    let key_title = "name";
    let (file_content, _) = parse_until_key_yml(file_content, key_title)?;
    let (file_content, title) = parse_value_yml(file_content, key_title)?;

    // BOX ART
    let key_box_art = "thumbnail";
    let (file_content, _) = parse_until_key_yml(file_content, key_box_art)?;
    let (file_content, _) = preceded(parse_not_alphanumeric, is_not(":")).parse(file_content)?;

    let box_art = if let Ok((_, box_art)) =
        preceded(tag(": grid:"), parse_till_end_of_line).parse(file_content)
    {
        Some(box_art.to_owned())
    } else {
        None
    };

    Ok((
        file_content,
        ParsableLibraryData {
            id,
            title,
            bottle_subdir,
            bottle_name,
            box_art,
            icon,
        },
    ))
}

// BOTTLES LAUNCHER ----------------------------------------------------------------------
#[derive(Debug)]
pub struct Bottles {
    path_bottles_dir: PathBuf,
    path_bottles_library: PathBuf,
    is_using_flatpak: bool,
}

impl Bottles {
    pub fn new(path_home: &Path, path_data: &Path) -> Self {
        let mut path_bottles_data = path_data.join("bottles");
        let mut is_using_flatpak = false;

        if !path_bottles_data.is_dir() {
            debug_fallback_flatpak!();

            is_using_flatpak = true;
            path_bottles_data = path_home.join(".var/app/com.usebottles.bottles/data/bottles");
        }

        let path_bottles_dir = path_bottles_data.join("bottles");
        let path_bottles_library = path_bottles_data.join("library.yml");

        debug_path!("data directory", path_bottles_data);
        debug_path!("bottles directory", path_bottles_dir);
        debug_path!("library yaml file", path_bottles_library);

        Bottles {
            path_bottles_dir,
            path_bottles_library,
            is_using_flatpak,
        }
    }

    /// Parse data from a given `bottle.yml` file
    #[tracing::instrument(level = "trace")]
    fn get_parsable_bottle_yml_data(
        &self,
        path_bottle_yml: PathBuf,
    ) -> Option<Vec<ParsableBottleYmlData>> {
        let file_content = read_to_string(&path_bottle_yml)
            .map_err(|e| {
                error!(
                    "Error with reading bottle yaml file at {:?}:\n{e}",
                    path_bottle_yml
                );
            })
            .ok()?;

        let mut parsed_games_data: Vec<ParsableBottleYmlData> = Vec::new();
        let mut file_content_str: &str = &file_content;

        loop {
            let Ok((new_file_content, parsed_data)) = parse_game_from_bottle_yml(file_content_str)
            else {
                break;
            };
            file_content_str = new_file_content;

            parsed_games_data.push(parsed_data)
        }

        Some(parsed_games_data)
    }

    /// Parse data from all `bottle.yml` files
    #[tracing::instrument(level = "trace")]
    fn parse_all_bottles(&self) -> Result<Arc<[ParsableBottleYmlData]>, io::Error> {
        Ok(read_dir(&self.path_bottles_dir)
            .map_err(|e| {
                error!("Error with reading the 'bottles' directory: {e:?}");
                e
            })?
            .flatten()
            .filter_map(|d| self.get_parsable_bottle_yml_data(d.path().join("bottle.yml")))
            .flatten()
            .collect())
    }

    /// Parse data from the Bottle's `library.yml` file
    #[tracing::instrument(level = "trace")]
    fn parse_bottles_library(&self) -> Result<Vec<ParsableLibraryData>, io::Error> {
        let library_file_content = read_to_string(&self.path_bottles_library).map_err(|e| {
            error!(
                "Error with reading Bottles library file at {:?}:\n{e}",
                &self.path_bottles_library
            );
            e
        })?;

        let mut parsed_data: Vec<ParsableLibraryData> = Vec::new();
        let mut library_file_content_str: &str = &library_file_content;

        loop {
            let Ok((new_library_file_content, parsed_library_data)) =
                parse_game_from_library(library_file_content_str)
            else {
                break;
            };

            library_file_content_str = new_library_file_content;
            parsed_data.push(parsed_library_data);
        }

        Ok(parsed_data)
    }

    /// Get all relevant game data by combining data from the `library.yml` file and
    /// each bottle's `.yml` file. Data is matched using game ID.
    #[tracing::instrument]
    pub fn parse_game_data(&self) -> Result<Vec<ParsableDataCombined>, io::Error> {
        let parsed_library_data = self.parse_bottles_library()?;
        let parsed_bottles_data = self.parse_all_bottles()?;

        Ok(parsed_library_data
            .into_iter()
            .filter_map(|library_data| {
                parsed_bottles_data
                    .iter()
                    .find(|b| b.id == library_data.id)
                    .map(|bottle_data| {
                        ParsableDataCombined::combine(library_data, bottle_data.clone())
                    })
            })
            .collect())
    }
}

impl Launcher for Bottles {
    fn is_detected(&self) -> bool {
        self.path_bottles_library.exists()
    }

    fn get_launcher_type(&self) -> SupportedLaunchers {
        LAUNCHER
    }

    #[tracing::instrument(level = "trace")]
    fn get_detected_games(&self) -> GamesResult {
        let parsed_data = self.parse_game_data()?;

        if parsed_data.is_empty() {
            warn_no_games!();
        }

        Ok(parsed_data
            .into_iter()
            .map(
                |ParsableDataCombined {
                     title,
                     path_icon,
                     box_art,
                     bottle_name,
                     bottle_subdir,
                     game_dir,
                 }| {
                    let launch_command = {
                        let base_args = ["run", "-p", &title, "-b", &bottle_name];
                        if self.is_using_flatpak {
                            get_launch_command_flatpak(
                                "com.usebottles.bottles",
                                ["--command=bottles-cli"],
                                base_args,
                                [],
                            )
                        } else {
                            get_launch_command("bottles-cli", base_args, [])
                        }
                    };
                    trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

                    let path_box_art = box_art.clone().and_then(|s| {
                        let path = self
                            .path_bottles_dir
                            .join(format!("{bottle_subdir}/grids/{s}"));
                        some_if_file(path)
                    });

                    let path_game_dir = some_if_dir(PathBuf::from(game_dir));

                    trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
                    trace!("{LAUNCHER} - Box art for '{title}': {path_box_art:?}");
                    trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");

                    Game {
                        title: clean_game_title(title),
                        path_icon,
                        launch_command,
                        path_box_art,
                        path_game_dir,
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

    #[test_case(false, ".local/share"; "standard")]
    #[test_case(true, "invalid/data/path"; "flatpak")]
    fn test_bottles_launcher(
        is_testing_flatpak: bool,
        path_data: &str,
    ) -> Result<(), GamesParsingError> {
        let path_file_system_mock = get_mock_file_system_path();
        let launcher = Bottles::new(
            &path_file_system_mock,
            &path_file_system_mock.join(path_data),
        );

        assert!(launcher.is_detected());
        assert!(launcher.is_using_flatpak == is_testing_flatpak);

        let games = launcher.get_detected_games()?;
        assert_eq!(games.len(), 4);

        assert_eq!(games[0].title, "Warcraft III");
        assert_eq!(games[1].title, "GOG Galaxy");
        assert_eq!(games[2].title, "EA Client");
        assert_eq!(games[3].title, "Estlcam");

        assert!(games[0].path_game_dir.is_some());
        assert!(games[1].path_game_dir.is_none());
        assert!(games[2].path_game_dir.is_none());
        assert!(games[3].path_game_dir.is_none());

        assert!(games[0].path_box_art.is_some());
        assert!(games[1].path_box_art.is_some());
        assert!(games[2].path_box_art.is_some());
        assert!(games[3].path_box_art.is_none());

        // TODO: test icons - need some way to write correct paths in test `library.yml` file
        for g in games {
            assert!(g.path_icon.is_none());
        }

        Ok(())
    }
}
