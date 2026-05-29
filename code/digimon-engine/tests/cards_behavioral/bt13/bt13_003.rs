//! BT13-003 Kyaromon
//!
//! Inherited Effect [Your Turn] [Once Per Turn] When a card is removed from
//! your security stack, 1 of your Digimon gains <Jamming> for the turn.

use digimon_engine::action::build_action_mask;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

#[test]
fn bt13_003_security_removed_selects_own_digimon_to_gain_jamming() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-003")
        .expect("BT13-003 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ALLY", "Ally"))
        .start();
    let carrier = runner.place_stack(0, &["BT13-003", "CARRIER"]);
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    fire_own_security_removed(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("security-removed trigger should select an own Digimon");
    assert!(!runner.pending_is_optional());
    let action = view.valid_action_ids[0];
    assert_eq!(
        build_action_mask(&runner.game, 0)[action as usize],
        1.0,
        "selected own-Digimon action must be legal in the action mask"
    );
    runner
        .execute_action(0, action)
        .expect("resolve Jamming target selection");

    assert!(
        runner.game.has_keyword(
            if action == view.valid_action_ids[0] {
                carrier
            } else {
                ally
            },
            Keyword::Jamming
        ) || runner.game.has_keyword(ally, Keyword::Jamming)
    );
}

#[test]
fn bt13_003_is_your_turn_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-003")
        .expect("BT13-003 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    runner.place_stack(0, &["BT13-003", "CARRIER"]);

    fire_own_security_removed(&mut runner);
    assert!(runner.pending_selection_view().is_some());
    let player = runner.pending_selection().unwrap().selecting_player;
    let action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(player, action)
        .expect("resolve first");

    fire_own_security_removed(&mut runner);
    assert!(
        runner.pending_selection_view().is_none(),
        "once-per-turn should block a second security-removed prompt"
    );

    runner.game.turn_player_idx = 1;
    fire_own_security_removed(&mut runner);
    assert!(
        runner.pending_selection_view().is_none(),
        "[Your Turn] should not fire on the opponent's turn"
    );
}

fn fire_own_security_removed(runner: &mut DebugRunner) {
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}
