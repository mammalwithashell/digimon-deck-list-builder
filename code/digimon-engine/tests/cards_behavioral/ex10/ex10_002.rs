//! EX10-002 Koromon — Digi-Egg, Lv.2, Black.
//!
//! # Card text (cards.json / cards/ex10/EX10-002.json)
//!
//! Inherited Effect [All Turns] [Once Per Turn] When attack targets change,
//! <Draw 1> (Draw 1 card from your deck.)
//!
//! # Patterns this test covers
//! - `scope: inherited`
//! - `on_attack_target_change` observer using the structured
//!   `AttackTargetChanged` event-context payload
//! - mandatory `draw: { of: you, count: 1 }`
//! - `once_per_turn` structural flag

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackTarget, TriggerSource};
use digimon_engine::AttackTargetChangeReason;

fn koromon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-002")
        .expect("EX10-002 YAML parses, compiles, and is in the embedded DSL pack")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("OLD-TARGET", "Old Target"))
        .add_card(make_test_card("NEW-TARGET", "New Target"))
        .add_card(make_test_card("DECK-PAD", "Deck Pad"))
        .deck(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .deck(1, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .build()
}

fn place_koromon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    runner.push_source(carrier, "EX10-002");
    carrier
}

fn enqueue_attack_target_changed(
    runner: &mut DebugRunner,
    attacker: PermanentHandle,
    old_target: AttackTarget,
    new_target: AttackTarget,
) {
    let card = runner.game.players[attacker.player as usize].battle_area[attacker.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnAttackTargetChange,
        TriggerSource::AttackTargetChanged {
            player: attacker.player,
            attacker,
            card,
            old_target,
            new_target,
            reason: AttackTargetChangeReason::EffectRedirect(None),
        },
    );
    runner.game.drain_effect_queue();
}

#[test]
fn ex10_002_yaml_compiles_and_is_in_pack() {
    let runner = koromon_runner();
    assert!(
        runner.compiled_card("EX10-002").is_some(),
        "EX10-002 must be present in the embedded DSL pack"
    );
}

#[test]
fn ex10_002_has_inherited_attack_target_change_opt_clause() {
    let runner = koromon_runner();
    let card = runner
        .compiled_card("EX10-002")
        .expect("EX10-002 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|effect| match effect {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| {
            triggered
                .when
                .contains(&CompiledTiming::OnAttackTargetChange)
                && triggered.scope == CompiledScope::Inherited
        })
        .expect("EX10-002 must have an inherited on_attack_target_change clause");

    assert!(
        clause.once_per_turn,
        "EX10-002 inherited attack-target-change clause must be [Once Per Turn]"
    );
    assert!(
        !clause.optional,
        "EX10-002 inherited draw is mandatory per printed text (no 'you may')"
    );
}

#[test]
fn ex10_002_inherited_draws_one_when_attack_target_changes_on_your_turn() {
    let mut runner = koromon_runner();
    let carrier = place_koromon_as_source(&mut runner);
    let old_target = runner.place_on_field(1, "OLD-TARGET", Some(0));
    let new_target = runner.place_on_field(1, "NEW-TARGET", Some(0));

    let hand_before = runner.hand_size(0);

    enqueue_attack_target_changed(
        &mut runner,
        carrier,
        AttackTarget::Digimon(old_target),
        AttackTarget::Digimon(new_target),
    );

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "Koromon source owner must draw 1 when attack targets change"
    );
}

#[test]
fn ex10_002_inherited_draws_on_opponents_turn_too() {
    let mut runner = koromon_runner();
    let carrier = place_koromon_as_source(&mut runner);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    let old_target = runner.place_on_field(0, "OLD-TARGET", Some(0));

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    let hand_before = runner.hand_size(0);

    enqueue_attack_target_changed(
        &mut runner,
        attacker,
        AttackTarget::Digimon(old_target),
        AttackTarget::Digimon(carrier),
    );

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "Koromon is [All Turns], so its owner must draw on opponent-turn target changes"
    );
}

#[test]
fn ex10_002_does_not_draw_without_koromon_source() {
    let mut runner = koromon_runner();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let old_target = runner.place_on_field(1, "OLD-TARGET", Some(0));
    let new_target = runner.place_on_field(1, "NEW-TARGET", Some(0));

    let hand_before = runner.hand_size(0);

    enqueue_attack_target_changed(
        &mut runner,
        attacker,
        AttackTarget::Digimon(old_target),
        AttackTarget::Digimon(new_target),
    );

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no Koromon source means no inherited draw"
    );
}

#[test]
fn ex10_002_opt_blocks_second_attack_target_change_same_turn() {
    let mut runner = koromon_runner();
    let carrier = place_koromon_as_source(&mut runner);
    let old_target = runner.place_on_field(1, "OLD-TARGET", Some(0));
    let new_target = runner.place_on_field(1, "NEW-TARGET", Some(0));

    enqueue_attack_target_changed(
        &mut runner,
        carrier,
        AttackTarget::Digimon(old_target),
        AttackTarget::Digimon(new_target),
    );
    let after_first = runner.hand_size(0);

    enqueue_attack_target_changed(
        &mut runner,
        carrier,
        AttackTarget::Digimon(new_target),
        AttackTarget::Player(1),
    );

    assert_eq!(
        runner.hand_size(0),
        after_first,
        "OPT must suppress a second EX10-002 draw in the same turn"
    );
}
