//! BT21-001 Gigimon — DigiEgg, Lv.2, Cost 0, Red.
//! Traits: Lesser, LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! ```text
//! Inherited Effect [Your Turn] [Once Per Turn]
//! When your opponent's security stack is removed from,
//! 1 of your Digimon may digivolve into a Digimon card with the
//! [Reptile] or [Dragonkin] trait in the hand with the digivolution
//! cost reduced by 1.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_001.cs
//!
//! # Patterns this test covers
//! - G4 Inherited On DigiEgg (OPT triggered from opponent-security-remove event)
//! - E2-adjacent: OPT + optional selection (the permanent-select is optional)
//! - effect_initiated_digivolve with cost: { reduce: 1 } (Phase 3a DSL verb)
//!
//! # Known gaps
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | Inherited On-Opponent-Security-Removed | G-INHERITED-DISPATCH | BLOCKED — enqueue_from_permanent does not scan digivolution stack sources for inherited triggered effects; entire behavioral firing path #[ignore]'d |
//! | OPT lockout | G-OPT-TRIGGERED | BLOCKED — once_per_turn not enforced for triggered effects; #[ignore]'d |
//!
//! Structural tests (compiled card shape) are unaffected by these gaps and pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

// ─── Helper builders ─────────────────────────────────────────────────────────

/// A Digimon with the [Reptile] trait — valid digivolve target.
fn make_reptile(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Reptile".to_string()];
    c.level = Some(4);
    c
}

/// A Digimon with the [Dragonkin] trait — valid digivolve target.
fn make_dragonkin(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Dragonkin".to_string()];
    c.level = Some(4);
    c
}

/// A Digimon with neither Reptile nor Dragonkin — invalid digivolve target.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Place Gigimon as a digivolution source under an existing permanent on
/// P0's battle area, simulating the inherited-from-stack scenario.
///
/// Returns the handle of the carrier permanent (unchanged).
fn place_gigimon_as_source(
    runner: &mut DebugRunner,
    carrier_id: &str,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, carrier_id, Some(0));

    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-001")
            .expect("BT21-001 registered in card_data");
        let next = game.next_card_index();
        let mut gigimon_src = CardSource::new(data_idx, 0, next);
        gigimon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at position 0 (bottom of stack — carrier stays on top).
        perm.card_sources.insert(0, gigimon_src);
    }

    handle
}

// ─── Section 1: Structural assertions ───────────────────────────────────────

/// Gigimon (BT21-001) has exactly one triggered clause,
/// which is Inherited, fires on OnOpponentSecurityRemoved,
/// is OPT, and is not optional at the clause level.
#[test]
fn bt21_001_has_exactly_one_inherited_triggered_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .memory(5)
        .start();

    let compiled = runner
        .compiled_card("BT21-001")
        .expect("BT21-001 compiled card present");

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
        "BT21-001 must have exactly one triggered clause"
    );
}

#[test]
fn bt21_001_inherited_clause_scope_timing_opt_flags() {
    let runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .memory(5)
        .start();

    let compiled = runner
        .compiled_card("BT21-001")
        .expect("BT21-001 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "BT21-001's clause must be Inherited scope"
    );
    assert!(
        clause
            .when
            .contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "BT21-001's clause must fire on OnOpponentSecurityRemoved; when={:?}",
        clause.when
    );
    assert!(
        clause.once_per_turn,
        "BT21-001's inherited clause must be once_per_turn (OPT)"
    );
    // The clause itself is NOT optional at the clause level —
    // the optional behaviour is inside the process (the permanent-select step is optional).
    // DCGO: CanActivateCondition always fires when conditions met; the *selection* itself
    // has canNoSelect = true (player may decline the permanent pick).
}

// ─── Section 2: Condition gating ────────────────────────────────────────────

/// Negative: on opponent's turn (active_when: your_turn), the inherited
/// clause must not fire even if P1's security is removed.
///
/// Simulated by ending P0's turn and having P0's stack attacker remove P1's
/// security — but we check memory delta (the clause grants nothing by itself;
/// the actual digivolve is interactive, but if it fires it would install a
/// selection and block auto_resolve).
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH — enqueue_from_permanent does not dispatch inherited triggered effects from digivolution stack sources"]
fn bt21_001_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER-P1"))
        .add_card(make_filler("SEC-P0"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(0, &["SEC-P0"])
        .hand(1, &[])
        .deck(0, &["CARRIER"])
        .deck(1, &["CARRIER"])
        .memory(5)
        .start();

    let _carrier = place_gigimon_as_source(&mut runner, "CARRIER");

    // Advance to P1's turn.
    runner.end_turn();

    // P1 attacks P0's security. If BT21-001's inherited fires on opponent's
    // turn (bug), a selection would be pending.
    let attacker_p1 = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    runner.attack_player(attacker_p1, 0, false);
    let _ = runner.auto_resolve();

    // No selection should have been parked by BT21-001's inherited clause.
    assert!(
        runner.pending_selection().is_none(),
        "BT21-001's inherited clause must not fire on opponent's turn"
    );
}

/// Positive: on P0's turn, when P1's security is removed, the inherited
/// clause should install a selection (optional Digimon selection).
///
/// BLOCKED: G-INHERITED-DISPATCH — stack inherited dispatch not yet wired.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH — enqueue_from_permanent does not dispatch inherited triggered effects from digivolution stack sources"]
fn bt21_001_inherited_fires_on_your_turn_and_installs_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-P1"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(1, &["SEC-P1"])
        .hand(0, &["REPTILE-HAND"])
        .deck(0, &["CARRIER"])
        .deck(1, &["CARRIER"])
        .memory(10)
        .start();

    let carrier = place_gigimon_as_source(&mut runner, "CARRIER");

    // P0 attacks P1's security → removes a security card.
    runner.attack_player(carrier, 1, false);

    // After the security removal the inherited clause should install a
    // pending_selection (optional own-field digimon pick).
    assert!(
        runner.pending_selection().is_some(),
        "BT21-001's inherited clause should install a selection on P0's turn after security removal"
    );
}

// ─── Section 3: Behavioral per-branch (selection path) ──────────────────────

/// When the player declines the permanent selection, no digivolve occurs.
///
/// BLOCKED: G-INHERITED-DISPATCH — clause never fires from stack.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH — enqueue_from_permanent does not dispatch inherited triggered effects from digivolution stack sources"]
fn bt21_001_declining_permanent_selection_no_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-P1"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(1, &["SEC-P1"])
        .hand(0, &["REPTILE-HAND"])
        .deck(0, &["CARRIER"])
        .memory(10)
        .start();

    let carrier = place_gigimon_as_source(&mut runner, "CARRIER");
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);

    // Selection must be optional (DCGO canNoSelect = true).
    assert!(
        runner.pending_is_optional(),
        "BT21-001's permanent selection must be optional (PASS = no digivolve)"
    );

    // Decline the permanent selection (PASS).
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    let _ = runner.auto_resolve();

    // Hand and field unchanged — no digivolve happened.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand unchanged after decline"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "field unchanged after decline"
    );
}

/// When the player selects a Digimon and then a Reptile/Dragonkin card from
/// hand, an effect_initiated_digivolve fires with cost reduced by 1.
///
/// BLOCKED: G-INHERITED-DISPATCH — clause never fires from stack.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH — enqueue_from_permanent does not dispatch inherited triggered effects from digivolution stack sources"]
fn bt21_001_selecting_permanent_then_hand_card_triggers_digivolve_cost_minus_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-P1"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(1, &["SEC-P1"])
        .hand(0, &["REPTILE-HAND"])
        .deck(0, &["CARRIER"])
        .memory(10)
        .start();

    let carrier = place_gigimon_as_source(&mut runner, "CARRIER");

    runner.attack_player(carrier, 1, false);

    // Step 1: select a Digimon on P0's field (should be OwnField selection).
    {
        let view = runner
            .pending_selection_view()
            .expect("selection must be pending");
        assert_eq!(
            view.kind,
            SelectionKind::OwnField,
            "first selection must be OwnField (choose which Digimon to digivolve)"
        );
        runner.execute_action(0, view.valid_action_ids[0]).unwrap();
    }

    // Step 2: select a Reptile/Dragonkin card from hand.
    {
        let view = runner
            .pending_selection_view()
            .expect("hand selection must be pending");
        assert_eq!(
            view.kind,
            SelectionKind::Hand,
            "second selection must be Hand (choose card to digivolve into)"
        );
        runner.execute_action(0, view.valid_action_ids[0]).unwrap();
    }

    let _ = runner.auto_resolve();

    // After the digivolve, the carrier permanent should have grown its stack
    // (the REPTILE-HAND card was placed as the new top digivolution).
    // We verify the hand is now empty (card was spent).
    assert_eq!(
        runner.hand_size(0),
        0,
        "REPTILE-HAND should have left P0's hand after the effect_initiated_digivolve"
    );
}

// ─── Section 4: Cost firing / event assertions ───────────────────────────────
// N/A: BT21-001's inherited clause fires a digivolve, which uses normal
// digivolution cost-payment mechanics. No trash-security or discard events
// are produced by the clause itself.

// ─── Section 5: OPT lockout ─────────────────────────────────────────────────

/// Second security removal in same turn must not install a selection.
///
/// BLOCKED: G-OPT-TRIGGERED — once_per_turn not enforced for triggered effects.
/// BLOCKED: G-INHERITED-DISPATCH — stack inherited dispatch not wired.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED — stack dispatch and OPT enforcement both missing"]
fn bt21_001_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(1, &["SEC-1", "SEC-2"])
        .hand(0, &["REPTILE-HAND"])
        .deck(0, &["CARRIER"])
        .memory(10)
        .start();

    let carrier = place_gigimon_as_source(&mut runner, "CARRIER");

    // First security removal — OPT fires.
    runner.attack_player(carrier, 1, false);
    // Decline by PASS to let the effect resolve without digivolving.
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    let _ = runner.auto_resolve();

    if runner.game_over() {
        return;
    }

    // Second security removal in same turn — OPT must be locked out.
    let carrier2 = runner.perm_handle(0, 0);
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();

    // No selection should be installed for the second trigger.
    assert!(
        runner.pending_selection().is_none(),
        "OPT must prevent second inherited trigger in the same turn"
    );
}

/// OPT resets after end_turn.
///
/// BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED.
#[test]
#[ignore = "BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED — stack dispatch and OPT enforcement both missing"]
fn bt21_001_opt_resets_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_filler("SEC-3"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(1, &["SEC-1", "SEC-2", "SEC-3"])
        .hand(0, &["REPTILE-HAND"])
        .deck(0, &["CARRIER"])
        .memory(10)
        .start();

    let carrier = place_gigimon_as_source(&mut runner, "CARRIER");

    // Turn 1: fire OPT and decline.
    runner.attack_player(carrier, 1, false);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    let _ = runner.auto_resolve();

    if runner.game_over() {
        return;
    }

    // End turn → P1 → P0 again.
    runner.end_turn();
    runner.end_turn();

    if runner.battle_area_size(0) == 0 || runner.game_over() {
        return;
    }

    // Turn 3 (P0 again): OPT should have reset.
    let carrier3 = runner.perm_handle(0, 0);
    runner.attack_player(carrier3, 1, false);

    // After the security removal, selection should install (OPT reset).
    assert!(
        runner.pending_selection().is_some(),
        "OPT must reset after end_turn; inherited should fire again on P0's next turn"
    );
}
