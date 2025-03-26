use lib_game_detector::get_detector;

fn main() {
    let detector = get_detector();

    // WARN: errors for each launcher are ignored and will only be visible in the logs
    let games = detector.get_all_detected_games();

    if games.is_empty() {
        println!("No games detected.")
    } else {
        println!("Detected games ({}):", games.len());

        for game in games {
            println!("\t- {}", game.title);
        }
    }
}
