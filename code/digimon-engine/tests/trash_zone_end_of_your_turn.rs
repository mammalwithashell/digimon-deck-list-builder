//! `{Trash}` [End of Your Turn] triggers — G-ENGINE-TRASH-ZONE-END-OF-YOUR-TURN.
//!
//! A clause printed with the `{Trash}` zone icon and an `[End of Your Turn]`
//! timing (P-250 Ogremon (X Antibody); DCGO `IsExistOnTrashTrigger` +
//! `IsOwnerTurn`) triggers at the end of its owner's turn while the card is in
//! that player's trash — and only there: not from the battle area, not from
//! the hand, and not at the end of the opponent's turn. The engine scans the
//! ENDING player's trash in `Game::fire_end_of_your_turn`, enqueueing only
//! `scope: trash` clauses, into the same simultaneous end-of-turn pool as the
//! battle-area and security-stack `[End of Your Turn]` effects.
//!
//! Covered with an inline fixture so the primitive is pinned independently of
//! the real card (P-250's own tests live in `cards_behavioral/p/p_250.rs`).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

const TRASH_EOT_YAML: &str = r#"
card: T-TRASHEOT
name: Trash End Drawer
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 3000
traits: []
effects:
  - scope: trash
    when: end_of_your_turn
    summary: "{Trash} [End of Your Turn] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

fn setup() -> DebugRunner {
    let mut filler = make_test_card("FILLER", "Filler");
    filler.level = Some(3);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(TRASH_EOT_YAML)
        .expect("fixture compiles")
        .add_card(filler)
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}

#[test]
fn trash_scoped_end_of_your_turn_fires_from_the_ending_players_trash() {
    let mut r = setup();
    r.inject_trash(0, "T-TRASHEOT");
    let hand_before = r.hand_size(0);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "the {{Trash}} clause drew 1");
}

#[test]
fn trash_scoped_end_of_your_turn_is_dormant_on_the_battle_area() {
    let mut r = setup();
    r.place_on_field(0, "T-TRASHEOT", Some(0));
    let hand_before = r.hand_size(0);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before, "a {{Trash}} clause never fires from the field");
}

#[test]
fn trash_scoped_end_of_your_turn_is_dormant_in_the_hand() {
    let mut r = setup();
    r.add_to_hand(0, "T-TRASHEOT");
    let hand_before = r.hand_size(0);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before);
}

#[test]
fn trash_scoped_end_of_your_turn_ignores_the_opponents_turn_end() {
    // Player 1's trash holds the card; player 0's turn ends. Compare against
    // a control run without the card so player 1's own turn-start draw (if
    // the turn machine runs it) cancels out.
    let mut control = setup();
    control.end_turn();
    let _ = control.auto_resolve();

    let mut r = setup();
    r.inject_trash(1, "T-TRASHEOT");
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(
        r.hand_size(1),
        control.hand_size(1),
        "[End of YOUR Turn] - silent at the opponent's turn end"
    );
    assert_eq!(r.hand_size(0), control.hand_size(0));
}
