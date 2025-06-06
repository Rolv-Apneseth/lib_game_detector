# lib_game_detector

<!-- cargo-rdme start -->

[![Docs](https://img.shields.io/docsrs/lib_game_detector)](https://docs.rs/lib_game_detector)
[![Crate](https://img.shields.io/crates/v/lib_game_detector.svg)](https://crates.io/crates/lib_game_detector)
[![Downloads](https://img.shields.io/crates/d/lib_game_detector.svg?label=crates.io%20downloads)](https://crates.io/crates/lib_game_detector)
[![Dependency status](https://deps.rs/repo/github/Rolv-Apneseth/lib_game_detector/status.svg)](https://deps.rs/repo/github/Rolv-Apneseth/lib_game_detector)
[![License](https://img.shields.io/badge/License-AGPLv3-green.svg)](https://github.com/Rolv-Apneseth/lib_game_detector/blob/main/LICENSE)

A Rust library for detecting and parsing data about games installed on the system. Currently
only supports Linux.

## Description

This is a Rust library intended to be used for programs which need information on currently
installed games, such as a games launcher, or mod manager. It can provide information such as
what games are installed across multiple launchers (such as Steam and Heroic Games Launcher),
where those games are installed, what command will launch them, and more.

## Quick start

Install with:

```sh
cargo add lib_game_detector
```

## Usage

```rust
use lib_game_detector::{data::SupportedLaunchers, get_detector};

let detector = get_detector();
let detected_launchers = detector.get_detected_launchers();
let all_games = detector.get_all_detected_games();
let all_games_by_launcher = detector.get_all_detected_games_per_launcher();
let all_games_from_steam = detector.get_all_detected_games_from_specific_launcher(SupportedLaunchers::Steam);
```

## Examples

- See [rofi-games](https://github.com/Rolv-Apneseth/rofi-games) for an example which uses this library to find games and their box art to use for displaying in a launcher
- Check the [examples folder](https://github.com/Rolv-Apneseth/lib_game_detector/tree/main/examples)

## Currently supported game sources

- Steam
  - Non-Steam games added as shortcuts are also supported. Just make sure to launch newly added
    shortcuts through Steam at at least once for them to be detected correctly (some files need
    to be generated).
- Heroic Games Launcher (Legendary, Nile, GOG, and manually added games)
- Lutris
- Bottles
  - Only lists entries included in the Library
- Modded Minecraft (Prism Launcher, ATLauncher)
  - Titles are given as `Minecraft - {instance name}`

<!-- cargo-rdme end -->

## License

[AGPL-3.0](./LICENSE)
