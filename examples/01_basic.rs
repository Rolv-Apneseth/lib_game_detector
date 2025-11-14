use lib_game_detector::{data::SupportedLaunchers, get_detector};
use tracing::debug;

fn main() {
    // Init tracing
    tracing_subscriber::fmt::init();

    debug!("Initialising detector");
    let detector = get_detector();

    dbg!(detector.get_detected_launchers());
    dbg!(detector.get_all_detected_games());
    dbg!(detector.get_all_detected_games_per_launcher());
    dbg!(detector.get_all_detected_games_with_box_art());
    dbg!(
        detector
            .get_all_detected_games_from_specific_launcher(SupportedLaunchers::HeroicGamesAmazon)
    );
}
