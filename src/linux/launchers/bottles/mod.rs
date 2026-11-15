// PATHS:
// - ~/.local/share/bottles/
// - Flatpak: ~/.var/app/com.usebottles.bottles/data/bottles/
pub use flatpak::BottlesFlatpak;
pub use standard::Bottles;

mod flatpak;
mod standard;

use std::{
    fs::{read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use nom::{
    IResult, Parser,
    bytes::complete::{is_not, tag, take_till},
    character::complete::multispace1,
    sequence::preceded,
};
use tracing::{debug, error, trace, warn};

use crate::{
    data::{Game, SupportedLaunchers},
    macros::logs::debug_path,
    parsers::{
        parse_not_alphanumeric, parse_till_end_of_line, parse_until_key_yml, parse_value_yml,
    },
    utils::{clean_game_title, some_if_dir, some_if_file},
};

#[derive(Debug, Clone)]
pub struct ParsableBottleSettingsData {
    custom_bottles_path: String,
}

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

    fn into_game<F>(self, path_bottles_dir: &Path, get_launch_command: F) -> Game
    where
        F: Fn([&str; 5]) -> Command,
    {
        let ParsableDataCombined {
            title,
            path_icon,
            box_art,
            bottle_name,
            bottle_subdir,
            game_dir,
        } = self;

        let launch_command = {
            let base_args = ["run", "-p", &title, "-b", &bottle_name];
            get_launch_command(base_args)
        };
        trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

        let path_box_art = box_art.clone().and_then(|s| {
            let path = path_bottles_dir.join(format!("{bottle_subdir}/grids/{s}"));
            some_if_file(path)
        });

        // No banner / background images
        let (path_hero, path_header) = (None, None);

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
            path_hero,
            path_header,
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

/// Used for parsing relevant settings from the given data file contents
#[tracing::instrument(level = "trace", skip(file_content))]
fn parse_bottle_settings_data<'a>(
    file_content: &'a str,
) -> IResult<&'a str, ParsableBottleSettingsData> {
    // CUSTOM BOTTLES PATH
    let key_bottle_name = "custom_bottles_path";
    let (file_content, _) = parse_until_key_yml(file_content, key_bottle_name)?;
    let (file_content, custom_bottles_path) = parse_value_yml(file_content, key_bottle_name)?;

    Ok((
        file_content,
        ParsableBottleSettingsData {
            custom_bottles_path,
        },
    ))
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

/// Apply a custom bottles path if one is defined in the given `data.yml` file
#[tracing::instrument(level = "trace")]
fn get_bottles_dir(path_bottles_data_dir: &Path, path_bottles_data: &Path) -> PathBuf {
    let path_default_bottles_dir = path_bottles_data_dir.join("bottles");

    if !path_bottles_data.is_file() {
        trace!("{} is not a valid file", path_bottles_data.display());
        trace!("Using default bottles path");
        return path_default_bottles_dir;
    }

    let file_content = match read_to_string(path_bottles_data) {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to read the Bottles data file at {path_bottles_data:?}: {e:#?}");
            warn!("Using default bottles path");
            return path_default_bottles_dir;
        }
    };

    let (_, settings_data) = match parse_bottle_settings_data(&file_content) {
        Ok(d) => d,
        Err(e) => {
            debug!("Failed to parse the Bottles data file at {path_bottles_data:?}: {e:#?}");
            debug!("Using default bottles path");
            return path_default_bottles_dir;
        }
    };

    let path_bottles_dir_custom = PathBuf::from(settings_data.custom_bottles_path);
    debug_path!("Custom bottles directory", path_bottles_dir_custom);

    path_bottles_dir_custom
}

/// Parse data from a given `bottle.yml` file
#[tracing::instrument(level = "trace")]
fn get_parsable_bottle_yml_data(path_bottle_yml: PathBuf) -> Option<Vec<ParsableBottleYmlData>> {
    let file_content = read_to_string(&path_bottle_yml)
        .inspect_err(|e| {
            error!("Error with reading bottle yaml file at {path_bottle_yml:?}: {e:#?}")
        })
        .ok()?;

    let mut parsed_games_data: Vec<ParsableBottleYmlData> = Vec::new();
    let mut file_content_str: &str = &file_content;

    while let Ok((new_file_content, parsed_data)) = parse_game_from_bottle_yml(file_content_str) {
        file_content_str = new_file_content;
        parsed_games_data.push(parsed_data)
    }

    Some(parsed_games_data)
}

/// Parse data from all `bottle.yml` files
#[tracing::instrument(level = "trace")]
fn parse_all_bottles(path_bottles_dir: &Path) -> Result<Arc<[ParsableBottleYmlData]>, io::Error> {
    Ok(read_dir(path_bottles_dir)
        .inspect_err(|e| error!("Error with reading the 'bottles' directory: {e:#?}"))?
        .flatten()
        .filter_map(|d| get_parsable_bottle_yml_data(d.path().join("bottle.yml")))
        .flatten()
        .collect())
}

/// Parse data from the Bottle's `library.yml` file
#[tracing::instrument(level = "trace")]
fn parse_bottles_library(
    path_bottles_library: &Path,
) -> Result<Vec<ParsableLibraryData>, io::Error> {
    let library_file_content = read_to_string(path_bottles_library).inspect_err(|e| {
        error!("Error with reading Bottles library file at {path_bottles_library:?}: {e:#?}");
    })?;

    let mut parsed_data: Vec<ParsableLibraryData> = Vec::new();
    let mut library_file_content_str: &str = &library_file_content;

    while let Ok((new_library_file_content, parsed_library_data)) =
        parse_game_from_library(library_file_content_str)
    {
        library_file_content_str = new_library_file_content;
        parsed_data.push(parsed_library_data);
    }

    Ok(parsed_data)
}

/// Get all relevant game data by combining data from the `library.yml` file and
/// each bottle's `.yml` file. Data is matched using game ID.
#[tracing::instrument]
fn parse_game_data(
    path_bottles_dir: &Path,
    path_bottles_library: &Path,
) -> Result<Vec<ParsableDataCombined>, io::Error> {
    let parsed_library_data = parse_bottles_library(path_bottles_library)?;
    let parsed_bottles_data = parse_all_bottles(path_bottles_dir)?;

    Ok(parsed_library_data
        .into_iter()
        .filter_map(|library_data| {
            parsed_bottles_data
                .iter()
                .find(|b| b.id == library_data.id)
                .map(|bottle_data| ParsableDataCombined::combine(library_data, bottle_data.clone()))
        })
        .collect())
}

#[cfg(test)]
mod test_utils {
    use std::path::{Path, PathBuf};

    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
    };

    /// Get override path_bottles_dir for test scenarios
    pub fn get_test_bottles_path(
        path_file_system_mock: &Path,
        path_bottles_data_dir: &Path,
        use_custom_bottles_path: bool,
    ) -> PathBuf {
        // custom_bottles_path is always an absolute path
        // (in our case "/bottles_external_library", see "data.yml").
        // However, since our test data lives in another path outside the default root of our filesystem,
        // we must correct it to ensure that we use path_file_system_mock as the root for our tests.
        if use_custom_bottles_path {
            path_file_system_mock.join("bottles_external_library")
        } else {
            path_bottles_data_dir.join("bottles")
        }
    }

    pub fn check_bottles_launcher<F>(
        launcher: impl Launcher,
        path_starts_with: &Path,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        assert!(launcher.is_detected());

        let games = launcher.get_detected_games()?;
        assert_eq!(games.len(), 4);

        assert_cmd(&games[0]);

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

        assert!(
            games[0]
                .path_box_art
                .as_ref()
                .unwrap()
                .starts_with(path_starts_with)
        );

        // TODO: test icons - need some way to write correct paths in test `library.yml` file
        for g in games {
            assert!(g.path_icon.is_none());
        }

        Ok(())
    }
}
