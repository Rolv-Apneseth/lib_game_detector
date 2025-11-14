use lib_game_detector::{error::GamesParsingError, get_detector};

#[test]
fn test_detector() -> Result<(), GamesParsingError> {
    let detector = get_detector();

    assert!(!detector.get_detected_launchers().is_empty());

    assert!(!detector.get_all_detected_games().is_empty());
    assert!(!detector.get_all_detected_games_with_box_art().is_empty());
    assert!(!detector.get_all_detected_games_per_launcher().is_empty());

    assert!(
        detector
            .get_all_detected_games_from_specific_launcher(
                lib_game_detector::data::SupportedLaunchers::Steam
            )
            .is_some()
    );

    Ok(())
}
