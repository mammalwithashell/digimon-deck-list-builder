//! Phase 1 timing-dispatch integration tests.
//!
//! Sanity-checks that all EffectBuilder constructors added in Phase 1 Task 1
//! compile and produce an Effect with the correct timing. Actual dispatch
//! wiring is tested in subsequent Phase 1 tasks.

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, EffectTiming};
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

// ─── TEST-P1-T5 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when any Digimon in its battle area is
/// attacking (observer timing — fires for the whole attacker's side).
struct AttackingMemoryGain;
impl CardEffect for AttackingMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .name("+1 when attacking")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn when_attacking_fires_for_attackers_battle_area() {
    // ATK is the attacker (Lv5, 8000 DP so it survives any security hit).
    // OBS is a bystander on the same side with WhenAttacking — it should fire.
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("OBS", "Observer", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(AttackingMemoryGain));

    // Play ATK then OBS. ATK lands at battle_area[0], OBS at battle_area[1].
    r.play(0, 0); // ATK (hand slot 0)
    r.play(0, 0); // OBS (now the only card left in hand)

    let attacker_handle = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    let before = r.memory();
    // vortex=true bypasses summoning sickness (both were just played this turn).
    let _ = r.attack_player(attacker_handle, 1, /* vortex */ true);

    // WhenAttacking fires for every permanent in player 0's battle area.
    // OBS's effect grants +1 memory.
    assert!(
        r.memory() > before,
        "WhenAttacking should have fired, granting +1 from OBS (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T6 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the end of any attack.
struct EoAttackMemoryGain;
impl CardEffect for EoAttackMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_attack(card)
            .name("+1 end of attack")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn end_of_attack_fires_for_all_players() {
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("OBS", "EoAObserver", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(EoAttackMemoryGain));

    r.play(0, 0); // ATK
    r.play(0, 0); // OBS

    let attacker_handle = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    let before = r.memory();
    let _ = r.attack_player(attacker_handle, 1, true);

    assert!(
        r.memory() > before,
        "EndOfAttack should have fired, granting +1 from OBS (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T7 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when a Digimon-vs-Digimon battle resolves.
struct EoBattleMemoryGain;
impl CardEffect for EoBattleMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_battle(card)
            .name("+1 end of battle")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn end_of_battle_fires_on_digimon_vs_digimon() {
    // ATK (9000 DP) beats DEF (5000 DP). OBS on ATK's side has EndOfBattle.
    let mut atk = plain_digimon("ATK", "Attacker", 5);
    atk.level = Some(5);
    atk.dp = Some(9000);
    let mut def = plain_digimon("DEF", "Defender", 5);
    def.level = Some(5);
    def.dp = Some(5000); // will lose to ATK

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(def)
        .add_card(plain_digimon("OBS", "BattleObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(EoBattleMemoryGain));

    r.play(0, 0); // ATK → battle_area[0]
    r.play(0, 0); // OBS → battle_area[1]

    // Place DEF on player 1's field directly (bypasses the memory seesaw).
    let defender_h = r.place_on_field(1, "DEF", Some(0));

    let attacker_h = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    let before = r.memory();
    let _ = r.attack_digimon(attacker_h, defender_h, true);

    assert!(
        r.memory() > before,
        "EndOfBattle should have fired on the ATK vs DEF battle (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T8 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any Digimon enters the field.
struct EntryObserverMem;
impl CardEffect for EntryObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("+1 when any Digimon enters")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_enter_field_anyone_fires_for_all_players() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Observer", 3))
        .add_card(plain_digimon("P1", "Play1", 2))
        .hand(0, &["OBS", "P1"])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(EntryObserverMem));

    // Play OBS first (its own entry triggers OnEnterFieldAnyone — but
    // the observer is the card being played, so its effects fire on
    // itself too, yielding +1). Then play P1, which also triggers
    // OnEnterFieldAnyone for OBS (which is now on the field), yielding +1.
    r.play(0, 0); // OBS
    let after_obs = r.memory();
    r.play(0, 0); // P1
    let after_p1 = r.memory();

    // The delta between after_obs and after_p1 includes:
    //   -2 (play_cost of P1) + 1 (OBS observed P1's entry) = -1
    // So after_p1 = after_obs - 1.
    assert_eq!(
        after_p1,
        after_obs - 1,
        "OnEnterFieldAnyone should have fired when P1 entered, granting OBS +1"
    );
}

// ─── TEST-P1-T9 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any Digimon is deleted.
struct DeletionObserverMem;
impl CardEffect for DeletionObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_any_deletion(card)
            .name("+1 when any Digimon deleted")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_any_deletion_fires_for_battle_deletion() {
    let mut atk = plain_digimon("ATK", "Atk", 5);
    atk.level = Some(5);
    atk.dp = Some(9000);
    let mut def = plain_digimon("DEF", "Def", 5);
    def.level = Some(5);
    def.dp = Some(3000); // weaker — will be deleted

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk.clone())
        .add_card(def.clone())
        .add_card(plain_digimon("OBS", "DelObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(15)
        .start();
    r.register_effect("OBS", Arc::new(DeletionObserverMem));

    r.play(0, 0); // ATK → battle_area[0]
    r.play(0, 0); // OBS → battle_area[1]

    // Seed DEF directly on player 1's field (bypasses memory seesaw).
    let defender_h = r.place_on_field(1, "DEF", Some(0));

    let attacker_h = digimon_engine::permanent::PermanentHandle { player: 0, index: 0 };
    let before = r.memory();
    let _ = r.attack_digimon(attacker_h, defender_h, true);

    // OBS fires once (DEF deleted). Only OBS is registered against
    // OnAnyDeletion, so any memory increase relative to `before` is
    // attributable to OBS. Other new timings (EndOfBattle/EndOfAttack)
    // have nothing registered against them in this test.
    assert!(
        r.memory() > before,
        "OnAnyDeletion should have fired when DEF was deleted, granting OBS +1 (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T10 ──────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any permanent suspends.
struct SuspendObserverMem;
impl CardEffect for SuspendObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_suspend(card)
            .name("+1 on suspend")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// A CardEffect that grants +1 memory whenever any permanent unsuspends.
struct UnsuspendObserverMem;
impl CardEffect for UnsuspendObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_unsuspend(card)
            .name("+1 on unsuspend")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_suspend_fires_when_permanent_suspends() {
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("SUS", "Suspended", 3))
        .add_card(plain_digimon("OBS", "SuspendObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(SuspendObserverMem));

    // Place OBS on field (it's in hand → play it, then place SUS directly).
    r.play(0, 0); // OBS → battle_area[0]

    // Seed SUS directly on field at index 1 (bypasses memory/summoning checks).
    let sus_h = r.place_on_field(0, "SUS", Some(0));

    let before = r.memory();
    // Suspend SUS — fires OnSuspend → OBS gains +1.
    r.game_mut().suspend(sus_h);

    assert!(
        r.memory() > before,
        "OnSuspend should have fired when SUS was suspended (before={}, after={})",
        before,
        r.memory()
    );
}

#[test]
fn on_unsuspend_fires_when_permanent_unsuspends() {
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("SUS", "Suspended", 3))
        .add_card(plain_digimon("OBS", "UnsuspendObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(UnsuspendObserverMem));

    // Play OBS onto field.
    r.play(0, 0); // OBS → battle_area[0]

    // Seed SUS directly on field and pre-suspend it by setting the flag
    // directly (bypassing Game::suspend so OnSuspend doesn't interfere).
    let sus_h = r.place_on_field(0, "SUS", Some(0));
    r.game_mut().players[sus_h.player as usize].battle_area[sus_h.index as usize].is_suspended =
        true;

    let before = r.memory();
    // Unsuspend SUS — fires OnUnsuspend → OBS gains +1.
    r.game_mut().unsuspend(sus_h);

    assert!(
        r.memory() > before,
        "OnUnsuspend should have fired when SUS was unsuspended (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T11 ──────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when an egg hatches.
struct HatchObsMem;
impl CardEffect for HatchObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_hatch(card)
            .name("+1 on hatch")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_hatch_fires_when_egg_hatches() {
    let mut egg = plain_digimon("EGG", "Egg", 0);
    egg.level = Some(2); // Lv.2 eggs are standard digitama

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(egg)
        .add_card(plain_digimon("OBS", "HatchObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["OBS"])
        .digitama(0, &["EGG"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(HatchObsMem));

    // Play OBS onto field so it can observe.
    r.play(0, 0); // OBS → battle_area[0]

    // No breeding-area yet.
    assert!(r.game_mut().player(0).breeding_area.is_none());

    let before = r.memory();
    let ok = r.game_mut().hatch(0);
    assert!(ok, "hatch should succeed when digitama deck is non-empty");

    assert!(
        r.memory() > before,
        "OnHatch should have fired after the egg moved to breeding (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T12 ──────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any Digimon digivolves.
struct DigivolveObsMem;
impl CardEffect for DigivolveObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolve(card)
            .name("+1 on digivolve")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// A Lv.4 Red Digimon that can digivolve onto a Lv.3 Red base for free.
fn lv4_digimon(card_id: &str, name: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0, // Red
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn on_digivolve_fires_globally_when_any_digimon_digivolves() {
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("B3", "Base3", 3))
        .add_card(lv4_digimon("E4", "Evo4"))
        .add_card(plain_digimon("OBS", "DigObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["E4", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DigivolveObsMem));

    // Place B3 directly on player 0's field (index 0).
    let base_h = r.place_on_field(0, "B3", Some(0));
    assert_eq!(base_h.index, 0);

    // Play OBS from hand (hand[1]) → battle_area[1].
    r.play(0, 1); // OBS → field

    // E4 is still at hand[0]. Digivolve E4 onto B3 (hand_index=0, target=base_h).
    let before = r.memory();
    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0, // E4 is hand[0]
        base_h,
        CostDelta::Free,
        false,
    );
    assert!(ok, "effect_initiated_digivolve should succeed");

    assert!(
        r.memory() > before,
        "OnDigivolve should have fired globally after E4 digivolved onto B3 (before={}, after={})",
        before,
        r.memory()
    );
}
