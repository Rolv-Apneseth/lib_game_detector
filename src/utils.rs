use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};

/// Cleans up parsed game title
pub fn clean_game_title(title: &str) -> String {
    title.replace(['™', '®'], "")
}

/// Returns a std::process::Command from a given command str and it's arguments
pub fn get_launch_command<'a>(
    command: &str,
    args: impl IntoIterator<Item = &'a str>,
    env_vars: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Arc<Mutex<Command>> {
    let mut command = Command::new(command);
    command.envs(env_vars).args(args);

    Arc::new(Mutex::new(command))
}

pub fn get_launch_command_flatpak<'a>(
    bottle_name: &str,
    flatpak_args: impl IntoIterator<Item = &'a str>,
    other_args: impl IntoIterator<Item = &'a str>,
    env_vars: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Arc<Mutex<Command>> {
    let command = get_launch_command("flatpak", flatpak_args, env_vars);

    if let Ok(mut c) = command.lock() {
        c.arg("run").arg(bottle_name).args(other_args);
    };

    command
}

/// Returns an Option containing the given `PathBuf`, if the `PathBuf` points to an actual file
pub fn some_if_file(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// Returns an Option containing the given `PathBuf`, if the `PathBuf` points to an actual directory
pub fn some_if_dir(path: PathBuf) -> Option<PathBuf> {
    path.is_dir().then_some(path)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use test_case::test_case;

    #[test_case("Soon™", "Soon")]
    #[test_case("Game®", "Game")]
    #[test_case("®T™i®t™l®e™", "Title")]
    fn test_clean_game_title(dirty: &str, clean: &str) {
        assert_eq!(clean_game_title(dirty), String::from(clean));
    }
}
