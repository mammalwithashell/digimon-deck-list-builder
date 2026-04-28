//! BT24-082 Owen Dreadnought — Tamer, Cost 3, Red, Traits: LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! [Start of Your Main Phase] By returning this Tamer to the bottom of the
//! deck, you may play 1 [Owen Dreadnought] from your hand without paying the
//! cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from
//! your trash without paying the cost.
//!
//! [Your Turn] When any of your Digimon digivolve into a [Reptile] or
//! [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP
//! for the turn. Then, it may attack.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_082.cs
//!
//! # Patterns this test covers
//! - B1 / B2: Start-of-main tamer with return-self-to-deck cost
//! - play_from_hand_free + play_from_trash_free chained
//! - Conditional gate: no_permanent (no Digimon) for trash sub-clause
//! - F9: on_digivolve tamer observer (security-loss/digivolve conditioned)
//!
//! # Known engine and DSL gaps affecting these tests
//!
//! G-ON-DIGIVOLVE-TRAIT-FILTER [engine gap]:
//!   `on_digivolve` fires with `TriggerSource::PlayerBattleArea`, which sets
//!   `trigger_context.target_permanent` = the OBSERVER permanent (the Tamer
//!   itself), not the newly-digivolved permanent. There is no way to:
//!     (a) filter "the newly-digivolved card has Reptile/Dragonkin",
//!     (b) reference the newly-digivolved permanent as the dp-modifier target.
//!   The YAML uses an `any_permanent` condition approximation (a Reptile/
//!   Dragonkin is anywhere on the field) and a select_own_permanent prompt
//!   for the DP modifier target — both under-specify the effect. Tests for
//!   correct per-Digimon targeting and trait filtering are `#[ignore]`'d
//!   pending a `TriggerContext::digivolve_target` field being added to
//!   `TriggerSource::PlayerBattleArea`.
//!
//! G-MAY-ATTACK-NOW [dsl+engine gap]:
//!   The DCGO fires `SelectAttackEffect` mid-effect-resolution (immediate
//!   optional attack). The Rust engine has no `EffectContext::may_attack_now`
//!   primitive, and `ModifierType::MayAttack` targets the end-of-turn attack
//!   window (Execute / Vortex), not an in-effect attack. The "it may attack"
//!   sub-clause of Clause 2 is omitted pending this primitive being added.
//!   Tests that assert the attack prompt installs are `#[ignore]`'d.
//!
//! G-OPT-TRIGGERED [engine gap]:
//!   OPT lockout on triggered clauses is not enforced through the queue drain.
//!   The lockout test for Clause 1 (re-entry after first activation) is
//!   `#[ignore = "pending: G-OPT-TRIGGERED"]`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt24/BT24-082.yaml");

// ─── helpers ──────────────────────────────────────────────────────────────────

fn make_digimon_card(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// BT24-082 must compile with exactly 3 triggered clauses:
///   - Clause 1: start_of_your_main_phase (face-up, optional)
///   - Clause 2: on_digivolve (face-up, optional)
///   - Clause 3: on_security (face-up, not optional)
#[test]
fn bt24_082_has_exactly_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-082")
        .expect("BT24-082 compiled card present");

    let triggered: Vec<_> = compiled
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
        "Expected 3 triggered clauses (start_of_your_main_phase, on_digivolve, on_security), got {}",
        triggered.len()
    );
}

/// Clause 1 fires at start_of_your_main_phase, is optional, and is face-up scoped.
#[test]
fn bt24_082_clause1_is_start_of_main_optional_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-082")
        .expect("BT24-082 compiled card present");

    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("must have a start_of_your_main_phase clause");

    assert!(
        clause1.optional,
        "Clause 1 must be optional (player may decline)"
    );
    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "Clause 1 must have face-up scope"
    );
}

/// Clause 2 fires on on_digivolve, is optional, and is face-up scoped.
#[test]
fn bt24_082_clause2_is_on_digivolve_optional_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-082")
        .expect("BT24-082 compiled card present");

    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolve))
        .expect("must have an on_digivolve clause");

    assert!(
        clause2.optional,
        "Clause 2 must be optional (cost = suspend self)"
    );
    assert_eq!(
        clause2.scope,
        CompiledScope::FaceUp,
        "Clause 2 must have face-up scope"
    );
}

/// The security clause exists with on_security timing, FaceUp scope, not optional.
#[test]
fn bt24_082_has_on_security_clause_not_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-082")
        .expect("BT24-082 compiled card present");

    let security = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("BT24-082 must have an on_security clause");

    assert!(
        !security.optional,
        "on_security must not be optional — [Security] play is mandatory"
    );
    assert_eq!(
        security.scope,
        CompiledScope::FaceUp,
        "[Security] clause must have FaceUp scope (security clauses use FaceUp, \
         not a separate Security variant)"
    );
}

// ─── SECTION 2 — Clause 1: Start-of-Main behavior ────────────────────────────

/// Clause 1: When Owen Dreadnought is on the field at start of main phase,
/// enqueuing the trigger does not panic and the game state settles.
#[test]
fn bt24_082_clause1_start_of_main_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-082", Some(0));

    // Manually fire the start-of-main trigger.
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();

    // No panic is the primary assertion. Drain any pending selections.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect("resolve succeeds");
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

/// Clause 1: When the optional clause fires and the player accepts, Owen is
/// returned to deck — the battle area loses the tamer and/or deck grows.
#[test]
fn bt24_082_clause1_returns_self_to_deck_when_activated() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-082", Some(0));
    let deck_before = runner.game.players[0].deck.len();
    let field_before = runner.game.players[0].battle_area.len();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();

    // Accept the activation if it's the first prompt.
    if let Some(ref pending) = runner.game.pending_selection {
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect("accept activation");
        runner.game.drain_effect_queue();
    }

    // Drain follow-up optional sub-selections (pass them to isolate cost).
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        // Use first action (may include PASS if available, or the only option).
        let action = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    let deck_after = runner.game.players[0].deck.len();
    let field_after = runner.game.players[0].battle_area.len();

    assert!(
        deck_after > deck_before || field_after < field_before,
        "Either deck grows (Owen returned to deck) or battle area shrinks \
         when clause 1 fires; deck_before={deck_before}, deck_after={deck_after}, \
         field_before={field_before}, field_after={field_after}"
    );
}

/// Clause 1 condition gate: when player 0 has a Digimon on field, the Elizamon
/// sub-clause must NOT fire (no_permanent condition is false → trash unchanged).
#[test]
fn bt24_082_clause1_elizamon_gate_blocked_when_digimon_present() {
    let digimon_cd = make_digimon_card("DIGTEST", 3000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(digimon_cd)
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-082", Some(0));
    runner.place_on_field(0, "DIGTEST", Some(0));

    // Put something in trash.
    if let Some(cs) = runner.game.players[0].deck.pop() {
        runner.game.players[0].trash.push(cs);
    }
    let trash_before = runner.game.players[0].trash.len();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();

    // Drain all selections by choosing first action.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    let trash_after = runner.game.players[0].trash.len();
    assert!(
        trash_after >= trash_before,
        "Trash must not shrink when no_permanent condition is NOT met (Digimon present); \
         trash_before={trash_before}, trash_after={trash_after}"
    );
}

/// Clause 1 condition gate positive: when player 0 has NO Digimon on field,
/// at least one pending selection fires (the effect reaches the Elizamon step).
#[test]
fn bt24_082_clause1_elizamon_gate_open_when_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Owen is the ONLY permanent on player 0's field (no Digimon).
    let tamer = runner.place_on_field(0, "BT24-082", Some(0));

    // Put something in trash for the potential Elizamon selection.
    if let Some(cs) = runner.game.players[0].deck.pop() {
        runner.game.players[0].trash.push(cs);
    }

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();

    // Either a pending selection or Owen was already returned to deck.
    let pending = runner.game.pending_selection.is_some();
    let field_shrank = runner.game.players[0].battle_area.len() == 0;

    assert!(
        pending || field_shrank,
        "Clause 1 must fire a prompt or return Owen to deck when no Digimon is present"
    );
}

// ─── SECTION 3 — Clause 2: On-Digivolve behavior ─────────────────────────────

/// Clause 2 smoke: on_digivolve trigger does not panic. Since make_test_card
/// doesn't set traits, the any_permanent condition won't fire here — this is
/// purely a non-panic verification.
#[test]
fn bt24_082_clause2_no_panic_on_digivolve_trigger() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT24-082", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    // No panic is the primary assertion.
}

/// Clause 2: When there are NO Reptile/Dragonkin Digimon on the field, the
/// condition blocks — Owen must remain unsuspended and no selection installs.
#[test]
fn bt24_082_clause2_condition_blocked_without_reptile_dragonkin() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-082", Some(0));

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Owen must start unsuspended"
    );

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Owen must not be suspended when no Reptile/Dragonkin condition is met"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "No selection prompt should install when condition fails"
    );
}

/// Clause 2 IGNORED: The correct filter should be 'the newly-digivolved card
/// has Reptile/Dragonkin'. With G-ON-DIGIVOLVE-TRAIT-FILTER, this is not
/// implementable — the context carries the observer's permanent, not the
/// newly-digivolved one.
#[test]
#[ignore = "pending: G-ON-DIGIVOLVE-TRAIT-FILTER — on_digivolve trigger context does not carry the newly-digivolved permanent's traits"]
fn bt24_082_clause2_fires_only_for_reptile_dragonkin_digivolve() {
    unimplemented!("blocked on G-ON-DIGIVOLVE-TRAIT-FILTER");
}

/// Clause 2 IGNORED: "it may attack" — no may_attack_now primitive.
#[test]
#[ignore = "pending: G-MAY-ATTACK-NOW — no DSL verb or engine primitive for immediate optional attack within an effect"]
fn bt24_082_clause2_may_attack_prompt_installs_after_dp_buff() {
    unimplemented!("blocked on G-MAY-ATTACK-NOW");
}

/// Clause 2 IGNORED: OPT lockout test.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — OPT lockout not enforced on triggered effects"]
fn bt24_082_clause2_opt_lockout_same_turn() {
    unimplemented!("blocked on G-OPT-TRIGGERED");
}

// ─── SECTION 4 — Security clause structural ───────────────────────────────────

/// [Security] clause fires on on_security timing, scope is FaceUp, not optional.
#[test]
fn bt24_082_security_clause_structural_play_from_security() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-082")
        .expect("BT24-082 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert!(
        !security_clause.optional,
        "on_security must not be optional — [Security] is mandatory"
    );
    assert_eq!(
        security_clause.scope,
        CompiledScope::FaceUp,
        "on_security must have FaceUp scope"
    );
}
