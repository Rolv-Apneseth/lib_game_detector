use divan::AllocProfiler;
use lib_game_detector::get_detector;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// Basic benchmark for getting a rough idea of overall speed and memory usage
#[divan::bench(sample_size = 100)]
fn bench_all() {
    let detector = get_detector();
    detector.get_detected_launchers();
    detector.get_all_detected_games();
    detector.get_all_detected_games_per_launcher();
    detector.get_all_detected_games_with_box_art();
}
