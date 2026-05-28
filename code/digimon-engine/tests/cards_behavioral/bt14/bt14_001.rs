//! BT14-001 Koromon — Digi-Egg, Lv.2, Red.
//! Traits: Lesser
//!
//! # Card text (cards.json)
//!
//! Inherited Effect [Your Turn] [Once Per Turn]
//! When a card is removed from your opponent's security stack, <Draw 1>
//! (Draw 1 card from your deck.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Red/BT14_001.cs
//!
//! # Patterns this test covers
//! - G4 Inherited triggered effect on DigiEgg (scope: inherited)
//! - E2-adjacent: OPT on triggered inherited clause
//! - F9-adjacent: security-conditioned trigger (on_opponent_security_removed)
//! - Active-when: your_turn gate
//!
//! # Known gaps (historical)
//! - G-INHERITED-DISPATCH: closed 2026-05-17 (Phase 2 Track D); the fire-test
//!   and your-turn-gate tests are live.
//! - G-OPT-TRIGGERED: closed 2026-05-16 (Phase 2 Track C); OPT lockout test
//!   is live.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const KOROMON_YAML: &str = include_str!("../../../cards/bt14/BT14-001.yaml");

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Tamer security filler — avoids the carrier mutual-destructing against
/// same-DP Digimon security per the post-fix `equal_dp_security_battle_*`
/// regression (RULES_CONTEXT 14-2-1-3).
fn make_non_digimon_security_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.dp = None;
    c.level = None;
    c.traits = vec![];
    c
}

/// Minimal runner: Koromon parsed from YAML, a filler carrier, a security card
/// for P1, and a filler deck card for P0 (so drawing doesn't panic).
fn koromon_runner() -> DebugRunner {
    // Tamer security so the carrier (2000 DP) doesn't mutual-destruct
    // with same-DP Digimon security per RULES_CONTEXT 14-2-1-3
    // (`equal_dp_security_battle_deletes_attacker`).
    DebugRunner::builder()
        .from_dsl_yaml(KOROMON_YAML)
        .expect("BT14-001 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-CARD"))
        .add_card(make_filler("DECK-CARD"))
        .security(1, &["SEC-CARD"])
        .deck(0, &["DECK-CARD"])
        .memory(5)
        .start()
}

/// Place Koromon as a bottom digivolution source under a CARRIER permanent on
/// P0's battle area. Returns the handle of the stacked permanent.
///
/// This replicates the pattern from BT21-008's test: manually insert the
/// DigiEgg's CardSource at position 0 of the carrier's card_sources stack.
fn place_koromon_as_source(runner: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, "CARRIER", Some(0));

    {
        use digimon_engine::card_source::CardSource;
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT14-001")
            .expect("BT14-001 registered in card_data");
        let next = game.next_card_index();
        let mut koromon_src = CardSource::new(data_idx, 0, next);
        koromon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at bottom (index 0); CARRIER remains at top (last position).
        perm.card_sources.insert(0, koromon_src);
    }

    handle
}

// ── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt14_001_has_exactly_one_triggered_clause() {
    let runner = koromon_runner();
    let compiled = runner
        .compiled_card("BT14-001")
        .expect("BT14-001 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "BT14-001 must have exactly one triggered clause"
    );
}

#[test]
fn bt14_001_inherited_clause_is_scope_inherited() {
    let runner = koromon_runner();
    let compiled = runner
        .compiled_card("BT14-001")
        .expect("BT14-001 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "clause must be scope: Inherited"
    );
}

#[test]
fn bt14_001_inherited_clause_when_on_opponent_security_removed() {
    let runner = koromon_runner();
    let compiled = runner
        .compiled_card("BT14-001")
        .expect("BT14-001 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert!(
        clause
            .when
            .contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "clause must fire on OnOpponentSecurityRemoved"
    );
}

#[test]
fn bt14_001_inherited_clause_is_once_per_turn() {
    let runner = koromon_runner();
    let compiled = runner
        .compiled_card("BT14-001")
        .expect("BT14-001 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert!(clause.once_per_turn, "clause must be once_per_turn");
}

#[test]
fn bt14_001_inherited_clause_is_not_optional() {
    // Card text "[Your Turn][Once Per Turn] When a card is removed from your
    // opponent's security stack, <Draw 1>" — no "you may"; mandatory trigger.
    let runner = koromon_runner();
    let compiled = runner
        .compiled_card("BT14-001")
        .expect("BT14-001 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert!(
        !clause.optional,
        "clause is not optional per card text (no 'you may')"
    );
}

// ── Section 2: Condition gating (your_turn gate) ────────────────────────────

/// NEGATIVE: Koromon's inherited clause is gated by `active_when: { your_turn: true }`.
/// On the opponent's turn, even if opponent removes P0's security, the clause
/// must NOT fire.
///
/// Note: This test places Koromon under a carrier and has P1 attack P0's security.
/// Because G-INHERITED-DISPATCH is an active gap, the clause would not fire anyway.
/// However, this test is structurally correct and documents the intended gate.
#[test]
fn bt14_001_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(KOROMON_YAML)
        .expect("BT14-001 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER-P1"))
        .add_card(make_filler("SEC-P0"))
        .add_card(make_filler("DECK-P0"))
        .add_card(make_filler("DECK-P1"))
        .security(0, &["SEC-P0"])
        .deck(0, &["DECK-P0"])
        .deck(1, &["DECK-P1"])
        .memory(5)
        .start();

    let _carrier = place_koromon_as_source(&mut runner);

    // Advance to P1's turn.
    runner.end_turn();

    let attacker_p1 = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let hand_before = runner.hand_size(0);

    // P1 attacks P0's security — this removes a card from P0's security stack.
    runner.attack_player(attacker_p1, 0, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);

    // Koromon's inherited clause is your_turn gated; it must NOT fire on P1's turn.
    // Hand should not grow (from the inherited draw).
    assert_eq!(
        hand_after, hand_before,
        "BT14-001 inherited clause must NOT fire on opponent's turn; \
         hand grew unexpectedly: {hand_before} -> {hand_after}"
    );
}

/// POSITIVE (BLOCKED — G-INHERITED-DISPATCH): Koromon as inherited source under
/// a carrier, P0 attacks P1's security on P0's turn. Clause fires → P0 draws 1.
///
/// Blocked until engine gains digivolution-stack inherited triggered-effect
/// dispatch (enqueue_from_permanent iterates card_sources[0..n-1]).
#[test]
fn bt14_001_inherited_fires_draws_one_card_on_your_turn() {
    let mut runner = koromon_runner();
    let carrier = place_koromon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after,
        hand_before + 1,
        "BT14-001 inherited clause must draw 1 card when it fires; \
         hand before={hand_before}, after={hand_after}"
    );
}

// ── Section 3: Behavioral outcome ───────────────────────────────────────────
// The single behavioral path (draw 1 on security removal) is covered by the
// positive test above. The draw is unconditional once the trigger fires —
// no branch selection needed (unlike BT15-003 Nyaromon's top/bottom choice).
// No additional behavioral sub-tests required.

// ── Section 4: Cost firing (events) ─────────────────────────────────────────
// BT14-001 has no cost-as-side-effect. The draw is the effect itself, not a
// cost, so no OnDiscardSecurity / OnLoseSecurity event-log assertions apply.

// ── Section 5: OPT enforcement ──────────────────────────────────────────────

/// OPT: second security removal in the same turn must NOT trigger the draw.
///
/// BLOCKED: G-OPT-TRIGGERED — max_per_turn is not enforced in run_queued_effect_inner
/// for triggered effects. Also depends on G-INHERITED-DISPATCH (the clause must
/// fire at all before OPT can be tested).
#[test]
fn bt14_001_inherited_opt_blocks_second_trigger_same_turn() {
    // Tamer security so the carrier can attack twice without dying to
    // a same-DP Digimon security on the first attack (post-fix per
    // RULES_CONTEXT 14-2-1-3).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(KOROMON_YAML)
        .expect("BT14-001 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-1"))
        .add_card(make_non_digimon_security_filler("SEC-2"))
        .add_card(make_filler("DECK-P0"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["DECK-P0", "DECK-P0"])
        .memory(10)
        .start();

    let carrier = place_koromon_as_source(&mut runner);

    // First attack: fires OPT → draw 1.
    let h0 = runner.hand_size(0);
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let h1 = runner.hand_size(0);
    let first_delta = h1 as i32 - h0 as i32;

    assert!(
        first_delta >= 1,
        "first security removal must draw 1 card; delta={first_delta}"
    );

    if runner.game_over() {
        return;
    }

    // Second attack: OPT used — clause must be locked.
    let carrier2 = runner.perm_handle(0, 0);
    let h2 = runner.hand_size(0);
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    let h3 = runner.hand_size(0);
    let second_delta = h3 as i32 - h2 as i32;

    assert!(
        second_delta < first_delta,
        "OPT must block second trigger; first_delta={first_delta}, second_delta={second_delta}"
    );
}
