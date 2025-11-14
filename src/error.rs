//! Error types used by this crate.

use std::io;

use thiserror::Error;

/// Custom error type returned when something goes wrong with parsing games from a launcher.
#[derive(Error, Debug)]
pub enum GamesParsingError {
    /// Error originating from [`io::Error`]
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Error originating from [`nom::Err`]
    #[error(transparent)]
    Nom(#[from] nom::Err<nom::error::Error<String>>),

    /// Error originating from [`rusqlite::Error`]
    #[error(transparent)]
    Db(#[from] rusqlite::Error),

    /// Error originating from any other source
    #[error("Other error: {0}")]
    Other(String),
}

impl From<nom::Err<nom::error::Error<&str>>> for GamesParsingError {
    fn from(err: nom::Err<nom::error::Error<&str>>) -> Self {
        Self::Nom(err.map_input(Into::into))
    }
}
