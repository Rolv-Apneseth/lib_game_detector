//! [![Crate](https://img.shields.io/crates/v/lib_game_detector.svg)](https://crates.io/crates/lib_game_detector)
//! ![License](https://img.shields.io/badge/License-AGPLv3-green.svg)
//!
//! A Rust library for detecting and parsing data about games installed on the system. Currently
//! only supports Linux.
//!
//! # Description
//!
//! This is a Rust library intended to be used for programs which need information on currently
//! installed games, such as a games launcher, or mod manager. It can provide information such as
//! what games are installed across multiple launchers (such as Steam and Heroic Games Launcher),
//! where those games are installed, what command will launch them, and more.
//!
//! # Quick start
//!
//! Install with `cargo add lib_game_detector` or add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! lib_game_detector = "0.0.1"
//! ```
//!
//! # Usage
//!
//! ```rust
//! use lib_game_detector::{data::SupportedLaunchers, get_detector};
//!
//! let detector = get_detector();
//! let detected_launchers = detector.get_detected_launchers();
//! let all_games = detector.get_all_detected_games();
//! let all_games_by_launcher = detector.get_all_detected_games_per_launcher();
//! let all_games_from_steam = detector.get_all_detected_games_from_specific_launcher(SupportedLaunchers::Steam);
//! ```
//!
//! # Examples
//!
//! - See [rofi-games](https://github.com/Rolv-Apneseth/rofi-games) for an example which uses this
//! library to find games and their box art to use for displaying in a launcher
//!
//! # Currently supported game sources
//!
//! - Steam
//!   - Non-steam games are currently not supported
//! - Heroic Games Launcher (Legendary, Nile and GOG)
//! - Lutris
//!
//! # TODO
//!
//! - Support additional game sources
//!   - Bottles
//!   - Steam's non-Steam games
//!   - Games available on `$PATH`, such as 0 A.D.
//! - Support Windows
use cfg_if::cfg_if;

pub mod data;
mod parsers;
mod utils;

use data::GamesDetector;

cfg_if! {
    if #[cfg(all(target_os = "linux"))] {
        mod linux;
        use linux::GamesDetectorLinux;

        pub fn get_detector() -> Box<dyn GamesDetector> {
            Box::new(GamesDetectorLinux::new())
        }
    } else {
        compiler_error!("This platform is currently not supported by lib_game_detector");
    }
}
