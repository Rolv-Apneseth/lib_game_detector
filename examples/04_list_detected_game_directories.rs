use lib_game_detector::get_detector;

fn main() {
    get_detector()
        .get_all_detected_games()
        .into_iter()
        // Ignore any detected games without a root directory
        .filter_map(|g| g.path_game_dir)
        // Filter for specific path(s)
        .filter(|p| p.starts_with("/home/"))
        // Print paths to `stdout`
        .for_each(|path| println!("{}", path.to_string_lossy()));
}
