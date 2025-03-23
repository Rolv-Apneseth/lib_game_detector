#[macro_export]
macro_rules! debug_path {
    ($description: expr, $path: ident) => {
        tracing::debug!(
            "{LAUNCHER} - {} exists at {:?}: {}",
            $description,
            $path,
            $path.exists()
        );
    };
}

#[macro_export]
macro_rules! debug_fallback_flatpak {
    () => {
        tracing::debug!("{LAUNCHER} - Attempting to fall back to flatpak");
    };
}

#[macro_export]
macro_rules! warn_no_games {
    () => {
        tracing::warn!("{LAUNCHER} - No games found");
    };
}
