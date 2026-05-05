//! P-134 Shoemon.
//! Printed text covered here: [On Play] 1 opponent Digimon gains
//! <Security A. -1> until the end of their turn. Inherited: [When Attacking]
//! [Once Per Turn] 1 opponent Digimon gets -2000 DP for the turn.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn p_134_on_play_grants_security_attack_minus_to_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-134")
        .expect("P-134 YAML loads")
        .add_card(make_test_card("OPP", "Opponent"))
        .hand(0, &["P-134"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.play(0, 0).expect("play Shoemon");

    let view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish grant");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1
    );
}

#[test]
fn p_134_inherited_when_attacking_gives_minus_2000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-134")
        .expect("P-134 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OPP", "Opponent"))
        .start();
    let carrier = runner.place_stack(0, &["P-134", "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP modifier");

    assert_eq!(runner.game.effective_dp(opp), Some(0));
}
