use digimon_engine::enums::{CardColor, Keyword, ModifierType};

use super::support::{
    bottom_source_id, hand_index, is_suspended, plain_digimon, push_to_trash,
    select_first_non_pass, select_hand_card, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-036";

#[test]
fn ex12_036_has_barrier_and_evade() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .start();
    let ryugumon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(runner.game.has_keyword(ryugumon, Keyword::Barrier));
    assert!(runner.game.has_keyword(ryugumon, Keyword::Evade));
}

#[test]
fn ex12_036_places_matching_hand_card_as_bottom_source_then_unsuspends_ally() {
    let aqua = super::support::digimon("AQUATIC-HAND", CardColor::Blue, 4, 4, 5000, &["Aquatic"]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(aqua)
        .add_card(plain_digimon("ALLY", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID, "AQUATIC-HAND"])
        .memory(12)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.players[0].battle_area[ally.index as usize].is_suspended = true;

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-036");
    select_hand_card(&mut runner, 0, "AQUATIC-HAND");
    assert_eq!(bottom_source_id(&runner, 0, 1), "AQUATIC-HAND");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve unsuspend");

    assert!(!is_suspended(&runner, 0, ally.index as usize));
}

#[test]
fn ex12_036_ally_played_locks_opponent_when_digivolving_and_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("ALLY", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 5000))
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve locks");

    assert!(runner
        .game
        .modifiers
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
    assert!(runner.game.modifiers.has(opp, ModifierType::CannotSuspend));
}

#[test]
fn ex12_036_ally_played_observer_does_not_fire_from_trash() {
    // Regression for the EX12-063 exam phantom (qa/dcgo-exams/EX12/
    // EX12-063-inherited0.yaml): a DEAD EX12-036 sitting in the trash must
    // NOT enqueue its "[All Turns][Once Per Turn] when any of your Digimon
    // are played..." observer. The printed clause carries no [Trash] scope
    // (rule 15-14-3: only effects with the {Trash} icon activate from the
    // trash), and DCGO does not fire dead cards' observers. Before the fix,
    // the EnteredField
    // dispatch's trash scan enqueued this clause from the trash, parking a
    // phantom selection / TriggerOrder entry.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("ALLY", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 5000))
        .start();
    push_to_trash(&mut runner, 0, CARD_ID);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);

    assert!(
        runner.game.pending_selection.is_none(),
        "dead EX12-036 in trash must not fire its on-ally-played observer \
         (got pending selection: {:?})",
        runner.game.pending_selection.as_ref().map(|s| &s.kind)
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects),
        "phantom trash observer must not have locked the opponent Digimon"
    );
    assert!(!runner.game.modifiers.has(opp, ModifierType::CannotSuspend));
}
