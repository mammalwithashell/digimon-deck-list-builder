//! Event host kind belongs to the captured card, never a compacted field slot.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
const WATCHER: &str = r#"
card: HOST-WATCHER
name: Host watcher
kind: digimon
level: 3
color: [blue]
cost: 0
dp: 1000
effects:
  - when: on_digivolution_card_trashed
    condition: { event_host_is_own_tamer: true }
    process:
      - gain_memory: 1
"#;
fn runner() -> DebugRunner {
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    DebugRunner::builder()
        .from_dsl_yaml(WATCHER)
        .unwrap()
        .add_card(tamer)
        .add_card(make_test_card("DIGIMON", "Digimon"))
        .add_card(make_test_card("UNDER", "Under"))
        .deck(0, &["DIGIMON"; 10])
        .deck(1, &["DIGIMON"; 10])
        .memory(3)
        .build()
}
#[test]
fn own_tamer_departure_retains_host_identity() {
    let mut r = runner();
    r.place_on_field(0, "HOST-WATCHER", None);
    let host = r.place_stack(0, &["UNDER", "TAMER"]);
    r.place_on_field(0, "DIGIMON", None);
    r.game.memory = 3;
    r.game.return_to_hand(host);
    assert_eq!(
        r.memory(),
        4,
        "captured Tamer remains the source-trash event host"
    );
}
#[test]
fn digimon_departure_does_not_borrow_compacted_tamer_kind() {
    let mut r = runner();
    r.place_on_field(0, "HOST-WATCHER", None);
    let host = r.place_stack(0, &["UNDER", "DIGIMON"]);
    r.place_on_field(0, "TAMER", None);
    r.game.memory = 3;
    r.game.return_to_hand(host);
    assert_eq!(
        r.memory(),
        3,
        "a Tamer moving into the old slot is not the event host"
    );
}
#[test]
fn opposing_tamer_is_not_owned_by_observer() {
    let mut r = runner();
    r.place_on_field(0, "HOST-WATCHER", None);
    let host = r.place_stack(1, &["UNDER", "TAMER"]);
    r.game.memory = 3;
    r.game.return_to_hand(host);
    assert_eq!(r.memory(), 3);
}
#[test]
fn leaving_tamer_rule_cleanup_does_not_draw_for_budmon() {
    let mut t = make_test_card("T", "Tamer");
    t.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .dsl_card("BT26-002")
        .unwrap()
        .add_card(t)
        .add_card(make_test_card("F", "Filler"))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.place_stack(0, &["BT26-002", "F"]);
    let host = r.place_stack(0, &["F", "T"]);
    let before = r.deck_size(0);
    r.game.return_to_hand(host);
    assert_eq!(
        r.deck_size(0),
        before,
        "rule disposal is not an effect trashing sources"
    );
}
#[test]
fn leaving_tamer_rule_cleanup_does_not_offer_bearcatmon_unsuspend() {
    let mut t = make_test_card("T", "Tamer");
    t.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .dsl_card("BT26-057")
        .unwrap()
        .add_card(t)
        .add_card(make_test_card("F", "Filler"))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let observer = r.place_on_field(0, "BT26-057", None);
    r.game.players[0].battle_area[observer.index as usize].is_suspended = true;
    let host = r.place_stack(0, &["F", "T"]);
    r.game.return_to_hand(host);
    assert!(
        r.pending_selection().is_none(),
        "rule cleanup must not offer an effect-only trigger"
    );
    assert!(r.game.players[0].battle_area[observer.index as usize].is_suspended);
}
