//! PendingSelectionView exposes only the serializable fields of a
//! `PendingSelection`. Round-trip test via TEST-010: play it, grab the
//! installed selection, take its view, assert every field matches.

use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::SelectionKind;

fn runner_with_two_opponents() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-010", "PilotDelete"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["TEST-010"])
        .memory(3)
        .start();
    r.place_on_field(1, "ALLY", Some(0));
    r.place_on_field(1, "ALLY", Some(0));
    r
}

#[test]
fn pending_selection_view_mirrors_serializable_fields() {
    let mut r = runner_with_two_opponents();
    r.play(0, 0);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("TEST-010 installs a pending selection");
    let view = sel.view();

    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.selecting_player, 0);
    assert_eq!(view.previous_phase, GamePhase::Breeding);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)],
    );
    assert!(view.is_optional);
    assert!(view.prompt.len() > 0, "prompt must not be empty");
    assert!(view.effect_choices.is_none());
}

#[test]
fn pending_selection_view_kind_as_str_round_trips() {
    let mut r = runner_with_two_opponents();
    r.play(0, 0);
    let sel = r.game.pending_selection.as_ref().unwrap();
    let view = sel.view();
    // Stable string form used by the PyO3 layer and by the Python-side
    // state filter for UI rendering. Variants use their `Debug` spelling.
    assert_eq!(view.kind_str(), "OppField");
    assert_eq!(view.previous_phase_str(), "Breeding");
}
