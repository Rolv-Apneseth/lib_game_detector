mod steam_base;
mod steam_shortcuts;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
};

pub use steam_base::Steam;
pub use steam_shortcuts::SteamShortcuts;
use tracing::error;

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
    let path = path_data.join("Steam");
    if path.is_dir() {
        return path;
    }

    // Fallback to reading ~/.steam/root or ~/.steam/steam symlinks
    // See: https://github.com/Rolv-Apneseth/lib_game_detector/issues/45
    let is_valid = |p: &Path| p.is_symlink() && p.is_dir();
    let mut symlink = path_home.join(".steam/root");
    if !is_valid(&symlink) {
        symlink = path_home.join(".steam/steam");
    }

    if !is_valid(&symlink) {
        return path;
    }

    symlink
        .canonicalize()
        .inspect_err(|e| {
            error!(
                "{} - Could not canonicalize symlink: {e}",
                SupportedLaunchers::Steam
            )
        })
        .unwrap_or(path)
}

fn get_steam_flatpak_dir(path_home: &Path) -> PathBuf {
    path_home.join(".var/app/com.valvesoftware.Steam/data/Steam")
}
