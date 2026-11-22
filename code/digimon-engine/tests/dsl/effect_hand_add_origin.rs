//! Independent coverage of effect-only hand-add event origin and recipient.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
const OBSERVER: &str = r#"
card: ADD-OBSERVER
name: Add observer
kind: digimon
level: 3
color: [blue]
cost: 0
dp: 1000
effects:
  - when: on_add_to_hand
    condition:
      all_of:
        - event_add_to_hand_player: opponent
        - event_is_effect_initiated: true
    process:
      - gain_memory: 1
"#;
fn setup(recipient: &str) -> DebugRunner {
    let draw = format!(
        r#"
card: DRAW-EFFECT
name: Draw effect
kind: digimon
level: 3
color: [blue]
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      - draw: {{ of: {recipient}, count: 1 }}
"#
    );
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(OBSERVER)
        .unwrap()
        .from_dsl_yaml(&draw)
        .unwrap()
        .add_card(make_test_card("F", "Filler"))
        .hand(0, &["DRAW-EFFECT"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.place_on_field(0, "ADD-OBSERVER", None);
    r
}
#[test]
fn opposing_hand_added_by_effect_triggers_observer() {
    let mut r = setup("opponent");
    let before = r.hand_size(1);
    r.play(0, 0).unwrap();
    assert_eq!(r.hand_size(1), before + 1);
    assert_eq!(r.memory(), 4);
}
#[test]
fn own_hand_added_by_effect_does_not_trigger_observer() {
    let mut r = setup("you");
    r.play(0, 0).unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), 3);
}
#[test]
fn ordinary_draw_is_not_effect_hand_add() {
    let mut r = setup("opponent");
    let before = r.hand_size(1);
    assert!(r.game.draw_one_for_player(1).unwrap());
    assert_eq!(r.hand_size(1), before + 1);
    assert_eq!(r.memory(), 3);
    assert!(r.pending_selection().is_none());
}
