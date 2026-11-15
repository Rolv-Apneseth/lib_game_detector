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

macro_rules! warn_no_games {
    () => {
        tracing::warn!("{LAUNCHER} - Detected but no games found");
    };
}
pub(crate) use warn_no_games;
