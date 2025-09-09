use lib_game_detector::get_detector;

fn main() {
    let games = get_detector().get_all_detected_games();
    let with_icons: Vec<_> = games.iter().filter(|g| g.path_icon.is_some()).collect();

    if games.is_empty() {
        println!("No games detected.");
        return;
    }

    println!("Games with icons ({}/{}):", with_icons.len(), games.len());
    for game in with_icons {
        let path_icon = game.path_icon.as_ref();
        assert!(path_icon.is_some_and(|p| p.exists()));
        println!("  - {}", game.title);
        println!("    icon: {}", path_icon.unwrap().to_string_lossy());
    }
}
