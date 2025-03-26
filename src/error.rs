//! Error types used by this crate.

use std::io;

use thiserror::Error;

/// Custom error type returned when something goes wrong with parsing games from a launcher.
#[derive(Error, Debug)]
pub enum GamesParsingError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Nom(#[from] nom::Err<nom::error::Error<String>>),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<nom::Err<nom::error::Error<&str>>> for GamesParsingError {
    fn from(err: nom::Err<nom::error::Error<&str>>) -> Self {
        Self::Nom(err.map_input(|input| input.into()))
    }
}
