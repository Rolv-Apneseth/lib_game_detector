use is_terminal::IsTerminal;
use lib_game_detector::get_detector;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

// NOTE: run with, e.g. `RUST_LOG=debug cargo run --example 03_env_logger > logs.txt`
fn main() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .without_time()
                .with_line_number(true)
                // Don't output colours for logs not being printed to a terminal
                .with_ansi(std::io::stdout().is_terminal()),
        )
        .with(env_filter)
        .init();

    get_detector().get_all_detected_games();
}
