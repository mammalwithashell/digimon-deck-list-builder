//! Phase 1 timing-dispatch integration tests.
//!
//! Sanity-checks that all EffectBuilder constructors added in Phase 1 Task 1
//! compile and produce an Effect with the correct timing. Actual dispatch
//! wiring is tested in subsequent Phase 1 tasks.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use std::sync::Arc;

fn dummy() -> CardHandle {
    CardHandle(0)
}

#[test]
fn new_effect_timings_are_constructible() {
    // Verify each new builder constructor compiles and sets the right timing.
    let card = dummy();

    let e = Effect::start_of_your_turn(card).build();
    assert_eq!(e.timing, EffectTiming::StartOfYourTurn);

    let e = Effect::start_of_opponents_turn(card).build();
    assert_eq!(e.timing, EffectTiming::StartOfOpponentsTurn);

    let e = Effect::start_of_your_main_phase(card).build();
    assert_eq!(e.timing, EffectTiming::StartOfYourMainPhase);

    let e = Effect::end_of_opponents_turn(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfOpponentsTurn);

    let e = Effect::when_attacking(card).build();
    assert_eq!(e.timing, EffectTiming::WhenAttacking);

    let e = Effect::end_of_attack(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfAttack);

    let e = Effect::end_of_battle(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfBattle);

    let e = Effect::on_enter_field_anyone(card).build();
    assert_eq!(e.timing, EffectTiming::OnEnterFieldAnyone);

    let e = Effect::on_any_deletion(card).build();
    assert_eq!(e.timing, EffectTiming::OnAnyDeletion);

    let e = Effect::on_digivolve(card).build();
    assert_eq!(e.timing, EffectTiming::OnDigivolve);

    let e = Effect::on_suspend(card).build();
    assert_eq!(e.timing, EffectTiming::OnSuspend);

    let e = Effect::on_unsuspend(card).build();
    assert_eq!(e.timing, EffectTiming::OnUnsuspend);

    let e = Effect::on_attack_target_change(card).build();
    assert_eq!(e.timing, EffectTiming::OnAttackTargetChange);

    let e = Effect::on_hatch(card).build();
    assert_eq!(e.timing, EffectTiming::OnHatch);

    let e = Effect::on_opponent_security_removed(card).build();
    assert_eq!(e.timing, EffectTiming::OnOpponentSecurityRemoved);

    let e = Effect::on_digivolution_card_trashed(card).build();
    assert_eq!(e.timing, EffectTiming::OnDigivolutionCardTrashed);
}

#[test]
fn existing_builders_still_correct() {
    // Regression check: pre-existing constructors must not have been disturbed.
    let card = dummy();

    let e = Effect::on_play(card).build();
    assert_eq!(e.timing, EffectTiming::OnPlay);
    assert!(e.on_play);

    let e = Effect::when_digivolving(card).build();
    assert_eq!(e.timing, EffectTiming::WhenDigivolving);
    assert!(e.when_digivolving);

    let e = Effect::on_attack(card).build();
    assert_eq!(e.timing, EffectTiming::OnAttack);
    assert!(e.on_attack);

    let e = Effect::on_deletion(card).build();
    assert_eq!(e.timing, EffectTiming::OnDeletion);
    assert!(e.on_deletion);

    let e = Effect::end_of_your_turn(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfYourTurn);

    let e = Effect::declarative(card).build();
    assert_eq!(e.timing, EffectTiming::Declarative);
    assert!(e.declarative);
}

// ─── Helper ───────────────────────────────────────────────────────────────────

/// A Lv.3 Red Digimon with configurable play_cost and no inherent effects.
fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── TEST-P1-T2 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the start of the controller's turn.
struct StartTurnMemoryGain;
impl CardEffect for StartTurnMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::start_of_your_turn(card)
            .name("+1 at turn start")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn start_of_your_turn_fires_for_controller() {
    // Provide non-empty decks for both players so neither decks out on draw.
    let filler: Vec<&str> = vec!["FILLER"; 5];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("STM", "StartMem", 3))
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["STM"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();

    r.register_effect("STM", Arc::new(StartTurnMemoryGain));

    // Play STM on turn 1 (controller = player 0). StartOfYourTurn does NOT
    // fire on the same turn the card was played (begin_turn already ran).
    let played = r.play(0, 0);
    assert_eq!(played, Some(0));

    // Pass turn — now player 1's turn. STM's start_of_your_turn should NOT
    // fire (player 0 is not the turn player).
    r.pass_turn();
    // Pass again — back to player 0. Now STM's start_of_your_turn fires.
    r.pass_turn();

    // Build a control game without the STM effect to derive the baseline
    // memory value after the same sequence of two pass_turn calls.
    let mut control = DebugRunner::builder()
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["FILLER"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    let _ = control.play(0, 0);
    control.pass_turn();
    control.pass_turn();
    let expected_no_effect = control.memory();

    assert_eq!(
        r.memory(),
        expected_no_effect + 1,
        "StartOfYourTurn should have fired for player 0, granting +1 memory"
    );
}

// ─── TEST-P1-T3 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the start of the controller's main phase.
struct MainPhaseMemoryGain;
impl CardEffect for MainPhaseMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::start_of_your_main_phase(card)
            .name("+1 at main phase start")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn start_of_your_main_phase_fires_for_controller() {
    let filler: Vec<&str> = vec!["F"; 10];
    // Both games use a cost-1 card so post-play memory is identical.
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("MPM", "MainPhase", 1))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["MPM"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    r.register_effect("MPM", Arc::new(MainPhaseMemoryGain));

    // Play the card (works from Breeding phase — play_from_hand has no phase check).
    r.play(0, 0);

    // Advance to Main phase on player 0's turn — fires StartOfYourMainPhase.
    r.game_mut().enter_main_phase();
    let mem_after = r.memory();

    // Control: same sequence without the effect, using the same cost-1 card.
    let mut ctrl = DebugRunner::builder()
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["F"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    ctrl.play(0, 0);
    ctrl.game_mut().enter_main_phase();
    let ctrl_mem = ctrl.memory();

    assert_eq!(
        mem_after,
        ctrl_mem + 1,
        "StartOfYourMainPhase should have fired for player 0, granting +1 memory"
    );
}

// ─── TEST-P1-T4 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the end of the opponent's turn.
struct EndOfOppTurnMem;
impl CardEffect for EndOfOppTurnMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_opponents_turn(card)
            .name("+1 at end of opponent's turn")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn end_of_opponents_turn_fires_for_non_turn_player() {
    // Player 0 plays a card with EndOfOpponentsTurn; when player 1 ends
    // their turn, player 0's effect should fire.
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("EOM", "EndOppMem", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["EOM"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    r.register_effect("EOM", Arc::new(EndOfOppTurnMem));
    r.play(0, 0);

    // Control game without the effect.
    let mut ctrl = DebugRunner::builder()
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["F"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    ctrl.play(0, 0);

    // Pass once: player 0 ends their turn → player 1's turn.
    // EndOfOpponentsTurn does NOT fire here (player 1 is the non-turn player,
    // but they have no permanents with the effect).
    r.pass_turn();
    ctrl.pass_turn();

    // Pass again: player 1 ends their turn → EndOfOpponentsTurn fires for
    // player 0's permanents (player 0 is the non-active player during player
    // 1's turn ending).
    r.pass_turn();
    ctrl.pass_turn();

    // gain_memory(1) fires during player 1's (ending) turn, so it shifts the
    // seesaw in player 1's favour. After the memory flip, player 0's observed
    // memory is 1 less than in the control game — proving the effect fired.
    assert_eq!(
        r.memory() + 1,
        ctrl.memory(),
        "EndOfOpponentsTurn should have fired for player 0's permanent at end of player 1's turn"
    );
}
