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
pub(crate) use debug_path;

macro_rules! debug_fallback_flatpak {
    () => {
        tracing::debug!("{LAUNCHER} - Attempting to fall back to flatpak");
    };
}
pub(crate) use debug_fallback_flatpak;

macro_rules! warn_no_games {
    () => {
        tracing::warn!("{LAUNCHER} - No games found");
    };
}
pub(crate) use warn_no_games;
