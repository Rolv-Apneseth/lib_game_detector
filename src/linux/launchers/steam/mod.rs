mod steam_base;
mod steam_shortcuts;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

pub use steam_base::Steam;
pub use steam_shortcuts::SteamShortcuts;

use crate::utils::{get_launch_command, get_launch_command_flatpak};

fn get_steam_launch_command(app_id: impl Display, is_using_flatpak: bool) -> Arc<Mutex<Command>> {
    let game_run_arg = format!("steam://rungameid/{app_id}");
    let args = [game_run_arg.as_str()];
    if is_using_flatpak {
        get_launch_command_flatpak("com.valvesoftware.Steam", [], args, [])
    } else {
        get_launch_command("steam", args, [])
    }
}

fn get_steam_dir(path_data: &Path) -> PathBuf {
    path_data.join("Steam")
}

fn get_steam_flatpak_dir(path_home: &Path) -> PathBuf {
    path_home.join(".var/app/com.valvesoftware.Steam/data/Steam")
}
