use lib_game_detector::{data::SupportedLaunchers, get_detector};
use tracing::debug;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    // Init tracing
    tracing_subscriber::fmt::init();

    debug!("Initialising detector");
    let detector = get_detector();

    // dbg!(detector.get_detected_launchers());
    dbg!(detector.get_all_detected_games());
    // dbg!(detector.get_all_detected_games_per_launcher());
    // dbg!(detector.get_all_detected_games_with_box_art());
    dbg!(detector
        .get_all_detected_games_from_specific_launcher(SupportedLaunchers::HeroicGamesAmazon));
}
