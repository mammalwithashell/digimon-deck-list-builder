//! BT25-086 Dan Yuki — Tamer, Red, Play Cost 3. Traits: ADAMAS/TS.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT25-086.md; confirmed
//! against card image BT25-086)
//!
//! [Start of Your Main Phase] If you have 4 or less memory, gain 1 memory.
//! [End of Your Turn] By suspending this Tamer, 1 of your [TS] trait Digimon
//! gets +1000 DP for the turn for each memory your opponent has. After, that
//! Digimon may attack.
//!
//! Security Effect: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_086.cs
//!   - OnStartMainPhase: IsOwnerTurn gate, if MemoryForPlayer <= 4 gain 1.
//!   - OnEndTurn: suspend-self activation cost (isOptional:true) → mandatory
//!     (canNoSelect:false) select 1 own [TS] Digimon → ChangeDigimonDP(+dpGain,
//!     UntilEachTurnEnd) where dpGain = Math.Max(0, opponent.MemoryForPlayer *
//!     1000) → if CanAttack, a real declinable SelectAttackEffect.
//!   - SecuritySkill: PlaySelfTamerSecurityEffect (mandatory play-self).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §5/§11)
//! - Structural: 3 triggered clauses (start_of_your_main_phase, end_of_your_turn
//!   optional, on_security not-optional), traits/color/cost metadata.
//! - Clause 1: memory_lte gate at exactly 4 (positive) and 5 (negative, no gain).
//! - Clause 2: suspend-self activation cost pays (Tamer ends suspended);
//!   mandatory [TS]-only target selection (non-TS Digimon not offered);
//!   DP scales as `+1000 * opponent_memory` (0, 1, 3 memory cases, "for each
//!   memory your opponent has" — event-log / modifier-registry assertion);
//!   optional may-attack follow-up exposed as a real declinable prompt.
//! - OPT lockout: with zero own [TS] Digimon, the clause condition gate keeps
//!   the whole "you may suspend" prompt from being offered at all.
//! - Clause 3: on_security clause exists, FaceUp scope, NOT optional.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-086";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn ts_digimon(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card_with_level(id, id, 4);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.traits = vec!["TS".to_string()];
    card
}

fn non_ts_digimon(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card_with_level(id, id, 4);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.traits = vec!["Not TS".to_string()];
    card
}

fn enqueue_start_of_main(runner: &mut DebugRunner, tamer: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();
}

fn enqueue_end_of_turn(runner: &mut DebugRunner, tamer: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_086_yaml_has_printed_metadata() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-086 must be in the embedded DSL pack");

    assert_eq!(card.name, "Dan Yuki");
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    for trait_name in ["ADAMAS", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-086 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_086_has_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        3,
        "expected 3 triggered clauses (start_of_your_main_phase, end_of_your_turn, on_security), got {}",
        triggered.len()
    );
}

#[test]
fn bt25_086_end_of_turn_clause_is_optional_and_has_expected_steps() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let eot = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EndOfYourTurn clause exists");

    assert!(eot.optional, "the 'by suspending this Tamer' clause must be a 'you may'");
    assert!(
        eot.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddDpModifier { .. })),
        "clause must contain an add_dp_modifier step"
    );
    assert!(
        eot.process
            .iter()
            .any(|s| matches!(s, CompiledStep::MayAttackNow { .. })),
        "clause must contain a may_attack_now step"
    );
}

#[test]
fn bt25_086_has_on_security_clause_not_optional() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("BT25-086 must have an on_security clause");

    assert!(!security.optional, "[Security] play is mandatory");
    assert_eq!(
        security.scope,
        CompiledScope::FaceUp,
        "on_security clauses use FaceUp scope"
    );
}

// ─── Section 2 — [Start of Your Main Phase] memory gain ─────────────────────

#[test]
fn bt25_086_start_of_main_gains_memory_at_four_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .memory(4)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    enqueue_start_of_main(&mut runner, tamer);

    assert_eq!(runner.game.memory, 5, "at exactly 4 memory, gain 1 (-> 5)");
}

#[test]
fn bt25_086_start_of_main_no_gain_above_four() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    enqueue_start_of_main(&mut runner, tamer);

    assert_eq!(runner.game.memory, 5, "above 4 memory, no gain");
}

// ─── Section 3 — [End of Your Turn] suspend → scaled DP → may attack ────────

#[test]
fn bt25_086_end_of_turn_no_prompt_without_own_ts_digimon() {
    // No own [TS] Digimon on the field at all -> the whole "you may suspend"
    // clause condition gate must keep the prompt from being offered.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(non_ts_digimon("PLAIN", 3000))
        .memory(0)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let _plain = runner.place_on_field(0, "PLAIN", Some(0));

    enqueue_end_of_turn(&mut runner, tamer);

    assert!(
        runner.pending_selection().is_none(),
        "with zero own [TS] Digimon, the suspend-and-buff clause must not fire at all"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer must not be suspended when the clause never activates"
    );
}

#[test]
fn bt25_086_end_of_turn_mandatory_target_is_ts_only() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(ts_digimon("TS-A", 5000))
        .add_card(non_ts_digimon("PLAIN", 3000))
        .memory(0)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let ts_ally = runner.place_on_field(0, "TS-A", Some(0));
    let plain_ally = runner.place_on_field(0, "PLAIN", Some(0));

    enqueue_end_of_turn(&mut runner, tamer);

    assert!(
        runner.pending_is_optional(),
        "the printed 'by suspending this Tamer' clause must start as a declineable prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept BT25-086 end-of-turn effect");

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "accepting the effect pays the printed suspend-this-Tamer activation cost"
    );

    let target_view = runner
        .pending_selection_view()
        .expect("mandatory [TS] Digimon target selection must be exposed");
    // Only the [TS] ally's own-field slot may be selected; the non-TS ally
    // must not appear among valid targets. OwnField selections encode their
    // valid_action_ids as encode_attack(0, slot) (selection.rs SelectionKind::OwnField).
    let ts_action = digimon_engine::action::space::encode_attack(0, ts_ally.index as u16);
    let plain_action = digimon_engine::action::space::encode_attack(0, plain_ally.index as u16);
    assert!(
        target_view.valid_action_ids.contains(&ts_action),
        "the [TS] Digimon must be a valid target"
    );
    assert!(
        !target_view.valid_action_ids.contains(&plain_action),
        "the non-[TS] Digimon must NOT be a valid target"
    );
}

#[test]
fn bt25_086_dp_scales_with_opponent_memory_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(ts_digimon("TS-A", 5000))
        .start();
    runner.game.turn_count = 1;
    // Turn player is P0 (the Tamer's controller). `game.memory` is stored
    // signed from the turn player's perspective; a value of -3 means the
    // opponent (P1) has MemoryForPlayer == +3.
    runner.game.set_memory(-3);

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let ts_ally = runner.place_on_field(0, "TS-A", Some(0));
    let base_dp = runner.effective_dp(ts_ally).expect("ally has dp");

    enqueue_end_of_turn(&mut runner, tamer);
    runner.accept_optional_trigger().expect("accept effect");

    let target_view = runner
        .pending_selection_view()
        .expect("mandatory [TS] Digimon target selection");
    let ts_action = digimon_engine::action::space::encode_attack(0, ts_ally.index as u16);
    runner
        .execute_action(target_view.selecting_player, ts_action)
        .expect("select the [TS] Digimon target");

    let dp_bonus = runner.modifiers().sum(ts_ally, ModifierType::ChangeDp);
    assert_eq!(
        dp_bonus, 3000,
        "opponent has 3 memory -> +1000 * 3 = +3000 DP for the turn"
    );
    assert_eq!(
        runner.effective_dp(ts_ally),
        Some(base_dp + 3000),
        "effective DP must reflect the +3000 grant"
    );
}

#[test]
fn bt25_086_dp_is_zero_when_opponent_has_no_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(ts_digimon("TS-A", 5000))
        .start();
    runner.game.turn_count = 1;
    // Turn player P0 has all the memory (+5); opponent MemoryForPlayer == -5,
    // clamped to 0 by the engine's `of: opponent` selector (DCGO Math.Max(0, ..)).
    runner.game.set_memory(5);

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let ts_ally = runner.place_on_field(0, "TS-A", Some(0));

    enqueue_end_of_turn(&mut runner, tamer);
    runner.accept_optional_trigger().expect("accept effect");

    let target_view = runner
        .pending_selection_view()
        .expect("mandatory [TS] Digimon target selection");
    let ts_action = digimon_engine::action::space::encode_attack(0, ts_ally.index as u16);
    runner
        .execute_action(target_view.selecting_player, ts_action)
        .expect("select the [TS] Digimon target");

    let dp_bonus = runner.modifiers().sum(ts_ally, ModifierType::ChangeDp);
    assert_eq!(
        dp_bonus, 0,
        "opponent has 0 (clamped) memory -> +1000 * 0 = +0 DP, a genuine zero grant"
    );
}

#[test]
fn bt25_086_after_dp_grant_may_attack_is_a_real_declinable_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(ts_digimon("TS-A", 5000))
        .start();
    runner.game.turn_count = 1;
    runner.game.players[1].security.clear();
    runner.game.set_memory(-1);

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let ts_ally = runner.place_on_field(0, "TS-A", Some(0));

    enqueue_end_of_turn(&mut runner, tamer);
    runner.accept_optional_trigger().expect("accept effect");

    let target_view = runner
        .pending_selection_view()
        .expect("mandatory [TS] Digimon target selection");
    let ts_action = digimon_engine::action::space::encode_attack(0, ts_ally.index as u16);
    runner
        .execute_action(target_view.selecting_player, ts_action)
        .expect("select the [TS] Digimon target");

    // After the DP grant resolves, the "may attack" follow-up must be a real
    // declinable prompt (PASS must be legal) — not a forced attack.
    let attack_view = runner
        .pending_selection_view()
        .expect("may_attack_now prompt must be exposed");
    assert!(
        attack_view.is_optional,
        "the printed 'may attack' follow-up must remain declineable"
    );
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, attack_view.selecting_player);
    assert_eq!(
        mask[digimon_engine::action::space::PASS as usize],
        1.0,
        "PASS must be legal for the declinable may-attack prompt"
    );
}

#[test]
fn bt25_086_declining_may_attack_leaves_digimon_unattacked() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(ts_digimon("TS-A", 5000))
        .start();
    runner.game.turn_count = 1;
    runner.game.players[1].security.clear();
    runner.game.set_memory(-1);

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let ts_ally = runner.place_on_field(0, "TS-A", Some(0));

    enqueue_end_of_turn(&mut runner, tamer);
    runner.accept_optional_trigger().expect("accept effect");

    let target_view = runner
        .pending_selection_view()
        .expect("mandatory [TS] Digimon target selection");
    let ts_action = digimon_engine::action::space::encode_attack(0, ts_ally.index as u16);
    runner
        .execute_action(target_view.selecting_player, ts_action)
        .expect("select the [TS] Digimon target");

    let attack_view = runner
        .pending_selection_view()
        .expect("may_attack_now prompt must be exposed");
    runner
        .execute_action(attack_view.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline the may-attack follow-up");

    assert!(
        !runner.game.players[0].battle_area[ts_ally.index as usize].is_suspended,
        "declining the may-attack must leave the Digimon unsuspended (did not attack)"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        0,
        "no security should have been checked when the attack was declined"
    );
}

#[test]
fn bt25_086_no_second_use_when_tamer_already_suspended() {
    // If the Tamer is already suspended (e.g. used earlier this turn or via
    // another effect), the suspend-self activation cost cannot be paid again,
    // so the clause condition gate (source_is_unsuspended) must keep the
    // prompt from being offered.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-086 YAML parses and compiles")
        .add_card(ts_digimon("TS-A", 5000))
        .memory(0)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let _ts_ally = runner.place_on_field(0, "TS-A", Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;

    enqueue_end_of_turn(&mut runner, tamer);

    assert!(
        runner.pending_selection().is_none(),
        "an already-suspended Tamer must not offer the suspend-and-buff clause again"
    );
}
