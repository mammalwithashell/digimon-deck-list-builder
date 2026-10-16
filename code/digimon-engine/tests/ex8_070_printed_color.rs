//! Official EX8-070 black color, verified against base official observation and image.
use digimon_engine::action::{mask::build_action_mask, space::PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;
fn fixture(color: CardColor) -> DebugRunner {
    let mut provider = make_test_card("PROVIDER", "Color witness");
    provider.colors = vec![color];
    provider.traits = vec!["Rock".to_string()];
    let mut r = DebugRunner::builder()
        .dsl_card("EX8-070")
        .unwrap()
        .add_card(provider)
        .add_card(make_test_card("FILL", "Source and deck"))
        .hand(0, &["EX8-070"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();
    r.skip_mulligan();
    let host = r.place_on_field(0, "PROVIDER", Some(0));
    r.push_source(host, "FILL");
    r.game.memory = 10;
    r.game.enter_main_phase();
    r
}
#[test]
fn black_provider_exposes_zofr_kabus_paid_action() {
    let mut r = fixture(CardColor::Black);
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(r.memory(), 8);
    assert_eq!(r.hand_size(0), 0);
    assert!(
        r.pending_selection_view().unwrap().is_optional,
        "{:?}",
        r.pending_selection_view()
    );
}
#[test]
fn purple_only_provider_does_not_satisfy_black_option_color() {
    let r = fixture(CardColor::Purple);
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 0.0);
}
#[test]
fn red_only_provider_does_not_satisfy_black_option_color() {
    let r = fixture(CardColor::Red);
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 0.0);
}
#[test]
fn black_color_legality_is_preserved_in_clone() {
    let r = fixture(CardColor::Black);
    let cloned = r.game.clone();
    assert_eq!(build_action_mask(&cloned, 0)[PLAY_HAND_START as usize], 1.0);
    assert_eq!(build_action_mask(&cloned, 0), build_action_mask(&r.game, 0));
}
