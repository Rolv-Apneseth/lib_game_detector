use std::io::{Write, stdout};

use lib_game_detector::get_detector;

fn main() {
    let detector = get_detector();
    let games = detector.get_all_detected_games();

    if games.is_empty() {
        println!("No games detected.")
    } else {
        let serialized = serde_json::to_string_pretty(&games).expect("failed to serialize games");
        let mut stdout = stdout().lock();
        writeln!(&mut stdout, "{serialized}").expect("failed to write to stdout");
    }
}
