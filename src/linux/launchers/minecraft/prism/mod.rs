// PATHS:
// - ~/.local/share/PrismLauncher/
pub use flatpak::MinecraftPrismFlatpak;
pub use standard::MinecraftPrism;

mod flatpak;
mod standard;

use std::{
    ffi::OsString,
    fs::{DirEntry, read_dir, read_to_string},
    path::{Path, PathBuf},
    process::Command,
};

use nom::IResult;
use tracing::{error, trace, warn};

use crate::{
    data::{Game, SupportedLaunchers},
    error::GamesParsingError,
    linux::launchers::minecraft::get_minecraft_title,
    macros::logs::warn_no_games,
    parsers::{parse_until_key_cfg, parse_value_cfg},
    utils::{get_existing_image_path, some_if_dir, some_if_file},
};

type FnGetCmd = fn(s: [&str; 2]) -> Command;

const LAUNCHER: SupportedLaunchers = SupportedLaunchers::MinecraftPrism;

struct ParsableConfigData {
    path_instances: PathBuf,
}

struct ParsableInstanceData {
    name: String,
    icon_key: Option<String>,
}

#[tracing::instrument(level = "trace")]
fn parse_prism_config<'a>(
    path_root: &Path,
    file_content: &'a str,
) -> IResult<&'a str, ParsableConfigData> {
    // INSTANCES DIR
    let instances_id = "InstanceDir";
    let (file_content, _) = parse_until_key_cfg(file_content, instances_id)?;
    let (file_content, instances_dir) = parse_value_cfg(file_content, instances_id)?;

    let mut path_instances = PathBuf::from(instances_dir);
    if !path_instances.is_absolute() {
        path_instances = path_root.join(path_instances);
    }

    Ok((file_content, ParsableConfigData { path_instances }))
}

#[tracing::instrument(level = "trace")]
fn parse_instance_config<'a>(file_content: &'a str) -> IResult<&'a str, ParsableInstanceData> {
    let (_, name) = parse_value_cfg(file_content, "name")?;

    let icon_key = parse_value_cfg(file_content, "iconKey")
        .ok()
        .map(|(_, k)| k);

    Ok((file_content, ParsableInstanceData { name, icon_key }))
}

#[tracing::instrument(level = "trace")]
fn get_path_instances(path_root: &Path, file_content: &str) -> PathBuf {
    match parse_prism_config(path_root, file_content) {
        Ok((_, config_data)) => {
            let ParsableConfigData { path_instances } = config_data;
            path_instances
        }
        Err(e) => {
            trace!("error in parsing instances path from config file: {e:#?}");
            path_root.join("instances")
        }
    }
}

fn get_path_icon(
    path_icons: &Path,
    icon_key: Option<&String>,
    path_instance: Option<&PathBuf>,
) -> Option<PathBuf> {
    icon_key
        .and_then(|key| get_existing_image_path(path_icons, key))
        // Support for older Prism versions
        .or_else(|| {
            let path_instance = path_instance?;

            some_if_file(path_instance.join("icon.png"))
                .or_else(|| some_if_file(path_instance.join("minecraft").join("icon.png")))
                .or_else(|| some_if_file(path_instance.join(".minecraft").join("icon.png")))
        })
}

fn try_parse_instance_dir_entry(
    dir_entry: DirEntry,
) -> Option<(OsString, Option<ParsableInstanceData>)> {
    let path = dir_entry.path();
    let instance_config = path.join("instance.cfg");
    if !path.is_dir() || !instance_config.is_file() {
        return None;
    }

    let name = dir_entry.file_name();

    let file_content = read_to_string(&instance_config)
        .map_err(|e| {
            warn!("{LAUNCHER} - couldn't read instance config at {instance_config:?}: {e:#?}");
        })
        .ok();
    let instance_data = file_content.as_ref().and_then(|file_content| {
        parse_instance_config(file_content)
            .map_err(|e| {
                warn!("{LAUNCHER} - couldn't parse instance config at {instance_config:?}: {e:#?}");
            })
            .ok()
            .map(|(_, d)| d)
    });

    Some((name, instance_data))
}

fn get_game_from_instance_data(
    path_instances: &Path,
    path_icons: &Path,
    name: OsString,
    opt_instance_data: Option<ParsableInstanceData>,
    get_launch_command: FnGetCmd,
) -> Game {
    let name = &name.to_string_lossy();
    let name = name.as_ref();
    let title = opt_instance_data.as_ref().map_or(name, |d| &d.name);

    let launch_command = {
        let args = ["--launch", name];
        get_launch_command(args)
    };
    trace!("{LAUNCHER} - launch command for '{title}': {launch_command:?}");

    let path_game_dir = some_if_dir(path_instances.join(name));

    let icon_key = opt_instance_data.as_ref().and_then(|d| d.icon_key.as_ref());
    let path_icon = get_path_icon(path_icons, icon_key, path_game_dir.as_ref());

    // AFAICT, no images saved other than the icon
    let (path_box_art, path_hero, path_header) = (None, None, None);

    trace!("{LAUNCHER} - Game directory for '{title}': {path_game_dir:?}");
    trace!("{LAUNCHER} - Icon for '{title}': {path_icon:?}");

    Game {
        title: get_minecraft_title(title),
        launch_command,
        path_box_art,
        path_game_dir,
        path_icon,
        source: LAUNCHER.clone(),
        path_hero,
        path_header,
    }
}

fn get_prism_games(
    path_root: &Path,
    path_config: &Path,
    get_launch_command: FnGetCmd,
) -> Result<Vec<Game>, GamesParsingError> {
    let file_content = read_to_string(path_config)?;

    let path_instances = get_path_instances(path_root, &file_content);
    if !path_instances.is_dir() {
        error!(
            "{LAUNCHER} - the parsed instances dir does not exist: {:?}",
            path_instances
        );
        return Ok(vec![]);
    }

    let path_icons = path_root.join("icons");
    if !path_icons.is_dir() {
        warn!("{LAUNCHER} - could not find icons directory at {path_icons:?}")
    }

    let games: Vec<Game> = read_dir(&path_instances)?
        .flatten()
        .filter_map(try_parse_instance_dir_entry)
        .map(|(name, opt_instance_data)| {
            get_game_from_instance_data(
                &path_instances,
                &path_icons,
                name,
                opt_instance_data,
                get_launch_command,
            )
        })
        .collect();

    if games.is_empty() {
        warn_no_games!();
    };

    Ok(games)
}

#[cfg(test)]
mod test_utils {
    use crate::{
        data::{Game, Launcher},
        error::GamesParsingError,
        linux::launchers::minecraft::get_minecraft_title,
    };

    pub fn check_prism_launcher<F>(
        launcher: impl Launcher,
        assert_cmd: F,
    ) -> Result<(), GamesParsingError>
    where
        F: Fn(&Game),
    {
        assert!(launcher.is_detected());

        let mut games = launcher.get_detected_games()?;
        games.sort_by_key(|a| a.title.clone());

        assert_eq!(games.len(), 3);

        assert_cmd(&games[0]);

        assert_eq!(games[0].title, get_minecraft_title("1.20.6"));
        assert_eq!(
            games[1].title,
            get_minecraft_title("All The Forge 10 v10.8.2")
        );
        assert_eq!(games[2].title, get_minecraft_title("The Pixelmon Modpack"));

        assert!(games[0].path_icon.is_none());
        assert!(games[1].path_icon.as_ref().is_some_and(|p| p.is_file()));
        assert!(games[2].path_icon.as_ref().is_some_and(|p| p.is_file()));

        assert!(games.iter().all(|g| g.path_game_dir.is_some()));
        assert!(games.iter().all(|g| g.path_box_art.is_none()));

        Ok(())
    }
}
