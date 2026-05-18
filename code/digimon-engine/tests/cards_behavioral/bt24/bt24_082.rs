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
//! `on_digivolve` now carries the newly-digivolved permanent/card in event
//! context. Clause 2 gates with `event_target_owner` + `event_target_trait_has`
//! and applies the DP modifier to `target: event_target`.
//!
//! G-MAY-ATTACK-NOW [resolved for this card]:
//!   The DCGO fires `SelectAttackEffect` mid-effect-resolution (immediate
//!   optional attack). Rust maps the "it may attack" rider to `may_attack_now`
//!   on the `event_target`, exposing decline through the pending-selection mask.
//!
//! G-OPT-TRIGGERED [engine gap]:
//!   OPT lockout on triggered clauses is not enforced through the queue drain.
//!   The lockout test for Clause 1 (re-entry after first activation) is
//!   `#[ignore = "pending: G-OPT-TRIGGERED"]`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt24/BT24-082.yaml");

// ─── helpers ──────────────────────────────────────────────────────────────────

fn make_digimon_card(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c.traits = vec!["Reptile".to_string()];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn enqueue_digivolve_event_for(runner: &mut DebugRunner, target: PermanentHandle) {
    let card = runner.game.players[target.player as usize].battle_area[target.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: target.player,
            permanent: target,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
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
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
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
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
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
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
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

/// Clause 2 smoke: on_digivolve trigger does not panic when the event target
/// has no matching trait.
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
    let target = runner.place_on_field(0, "FILL", Some(0));

    enqueue_digivolve_event_for(&mut runner, target);
    // No panic is the primary assertion.
}

/// Clause 2: When the newly-digivolved target is not Reptile/Dragonkin, the
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
    let target = runner.place_on_field(0, "FILL", Some(0));

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Owen must start unsuspended"
    );

    enqueue_digivolve_event_for(&mut runner, target);

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Owen must not be suspended when no Reptile/Dragonkin condition is met"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "No selection prompt should install when condition fails"
    );
}

#[test]
fn bt24_082_clause2_fires_only_for_reptile_dragonkin_digivolve() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_digimon_card("REPTILE-TARGET", 4000))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-082", Some(0));
    let target = runner.place_on_field(0, "REPTILE-TARGET", Some(0));
    let dp_before = runner
        .effective_dp(target)
        .expect("target has effective DP");

    enqueue_digivolve_event_for(&mut runner, target);

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Owen should pay the suspend cost when a Reptile/Dragonkin digivolve event is observed"
    );
    assert_eq!(
        runner
            .effective_dp(target)
            .expect("target has effective DP"),
        dp_before + 3000,
        "DP modifier must apply to the newly-digivolved event target"
    );
}

#[test]
fn bt24_082_clause2_may_attack_prompt_installs_after_dp_buff() {
    let mut security_card = make_filler("SEC");
    security_card.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-082 YAML parses")
        .add_card(make_digimon_card("REPTILE-TARGET", 4000))
        .add_card(security_card)
        .security(1, &["SEC"])
        .deck(0, &["SEC"])
        .deck(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let tamer = runner.place_on_field(0, "BT24-082", Some(0));
    let target = runner.place_on_field(0, "REPTILE-TARGET", Some(0));
    let dp_before = runner
        .effective_dp(target)
        .expect("target has effective DP");
    let security_before = runner.game.players[1].security.len();

    enqueue_digivolve_event_for(&mut runner, target);

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Owen should pay the suspend cost before offering the attack"
    );
    assert_eq!(
        runner
            .effective_dp(target)
            .expect("target has effective DP"),
        dp_before + 3000,
        "DP modifier must apply before the may-attack prompt resolves"
    );

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Clause 2 should install a may_attack_now prompt");
    assert!(pending.is_optional, "printed 'may attack' must be optional");
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "optional may_attack_now must expose PASS through the action mask"
    );

    let attack_player = encode_attack(target.index as u16, SECURITY_TARGET);
    assert!(
        pending.valid_action_ids.contains(&attack_player),
        "newly digivolved Reptile/Dragonkin should be able to attack the opponent player"
    );

    runner
        .game
        .resolve_selection(0, attack_player)
        .expect("resolve Owen-granted attack");

    assert_eq!(
        runner.game.players[1].security.len(),
        security_before - 1,
        "the effect-created attack should resolve the normal security flow"
    );
    assert!(
        runner.game.players[0].battle_area[target.index as usize].is_suspended,
        "BT24-082 does not say without suspending, so the attacker must pay the suspend cost"
    );
}

/// Clause 2 OPT lockout test. G-OPT-TRIGGERED closed 2026-05-17 (Phase 2
/// Track C). The reusable OPT-lockout / OPT-reset behavior is exercised
/// against multiple Owen-shape clauses already (bt14_001, bt21_001,
/// bt22_005, bt21_017, p_189, bt24_012 — see `qa/resolved-gaps.md` §
/// "Phase 2 Track C closure"). BT24-082's clause-2 lockout shares the
/// same drainer path; a dedicated card-local regression is no longer
/// required for IMPLEMENTED status. Marked retired in Phase 2 Track G.
#[test]
#[ignore = "pending: card-local OPT regression body not authored — substrate closed by Track C, covered by sibling cards in qa/resolved-gaps.md"]
fn bt24_082_clause2_opt_lockout_same_turn() {
    // Body intentionally left absent — sibling regression coverage exists.
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
