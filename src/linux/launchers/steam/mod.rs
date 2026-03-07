mod steam_base;
mod steam_shortcuts;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
};

pub use steam_base::Steam;
pub use steam_shortcuts::SteamShortcuts;
use tracing::{debug, error};

use crate::{
    data::SupportedLaunchers,
    utils::{get_launch_command, get_launch_command_flatpak},
};

fn get_steam_launch_command(app_id: impl Display, is_using_flatpak: bool) -> Command {
    let game_run_arg = format!("steam://rungameid/{app_id}");
    let args = [game_run_arg.as_str()];
    if is_using_flatpak {
        get_launch_command_flatpak("com.valvesoftware.Steam", [], args, [])
    } else {
        get_launch_command("steam", args, [])
    }
}

fn get_steam_dir(path_home: &Path, path_data: &Path) -> PathBuf {
    use SupportedLaunchers::Steam;

    // Try following ~/.steam/{root,steam} symlinks and only use
    // $XDG_DATA_HOME/Steam as a fallback, since it's not reliable.
    // See: https://github.com/Rolv-Apneseth/lib_game_detector/issues/45
    let is_valid = |p: &Path| p.is_symlink() && p.is_dir();
    let mut symlink = path_home.join(".steam/root");
    if !is_valid(&symlink) {
        symlink = path_home.join(".steam/steam");
    }
    if !is_valid(&symlink) {
        debug!("{Steam} - Could not find symlinked directories, using fallback");
        return path_data.join("Steam");
    }

    symlink.canonicalize().unwrap_or_else(|e| {
        error!("{Steam} - Could not canonicalize symlink, using fallback: {e}");
        path_data.join("Steam")
    })
}

fn get_steam_flatpak_dir(path_home: &Path) -> PathBuf {
    path_home.join(".var/app/com.valvesoftware.Steam/data/Steam")
}
