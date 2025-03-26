use is_terminal::IsTerminal;
use lib_game_detector::get_detector;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// NOTE: run with, e.g. `RUST_LOG=debug cargo run --example 03_env_logger > logs.txt`
fn main() {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .without_time()
                .with_line_number(true)
                // Don't output colours for logs not being printed to a terminal
                .with_ansi(std::io::stdout().is_terminal()),
        )
        .with(EnvFilter::from_default_env())
        .init();

    get_detector().get_all_detected_games();
}
