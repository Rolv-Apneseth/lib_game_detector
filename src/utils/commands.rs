use std::process::Command;

/// Returns a std::process::Command from a given command str and it's arguments
pub fn get_launch_command<'a>(
    command: &str,
    args: impl IntoIterator<Item = &'a str>,
    env_vars: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Command {
    let mut command = Command::new(command);
    command.envs(env_vars).args(args);

    command
}

pub fn get_launch_command_flatpak<'a>(
    bottle_name: &str,
    flatpak_args: impl IntoIterator<Item = &'a str>,
    other_args: impl IntoIterator<Item = &'a str>,
    env_vars: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Command {
    let mut command = get_launch_command("flatpak", flatpak_args, env_vars);
    command.arg("run").arg(bottle_name).args(other_args);

    command
}
