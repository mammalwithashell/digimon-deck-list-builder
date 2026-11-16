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
//! | Inherited On-Opponent-Security-Removed | G-INHERITED-DISPATCH | RESOLVED 2026-05-17 (Phase 2 Track D); the dispatch walks the digivolution stack and the clause fires with the stacked source's identity preserved. The 3 negative/positive/decline tests are live. |
//! | Effect-initiated digivolve (cost-1) follow-up | (test fixture) | The fire-test for the "select permanent + card" digivolve path remains `#[ignore]`'d: REPTILE-HAND is built with empty `evo_costs`, so the engine's normal digivolve route rejects it even when the clause supplies `cost: { reduce: 1 }`. Independent of inherited dispatch. |
//! | OPT lockout | G-OPT-TRIGGERED | Closed 2026-05-16 (Phase 2 Track C); the OPT-lockout test is live (Section 5 below). |
//!
//! Structural tests (compiled card shape) are unaffected by these gaps and pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Helper builders ─────────────────────────────────────────────────────────

/// A Lv.4 [Reptile] Digimon that digivolves from a red Lv.3 for `cost`.
fn make_reptile_costing(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Reptile".to_string()];
    c.level = Some(4);
    // A printed digivolve route onto the red Lv.3 CARRIER: the effect
    // digivolve follows the normal digivolution requirements
    // (`ignore_requirements: false`, DCGO `DigivolveIntoHandOrTrashCard`).
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: cost,
    }];
    c
}

/// A Digimon with the [Reptile] trait — valid digivolve target (red Lv.3,
/// digivolution cost 3).
fn make_reptile(id: &str) -> CardData {
    make_reptile_costing(id, 3)
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

/// Tamer security filler — used so the carrier attacking security doesn't
/// mutually destruct with same-DP Digimon security (RULES_CONTEXT 14-2-1-3,
/// see `equal_dp_security_battle_deletes_attacker`). Non-Digimon security
/// skips the battle compare per 13-1-8-3 / 14-2-3.
fn make_non_digimon_security_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.dp = None;
    c.level = None;
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
/// is OPT, and is optional at the clause level (the whole effect is a "may").
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
    // The whole effect is one "may": DCGO BT21_001.cs `SetUpActivateClass(..,
    // 1, true, ..)` (`isOptional`). The clause is `optional: true` with the
    // "may" folded into the permanent pick, so passing it keeps the
    // [Once Per Turn] (Q1818; G-DSL-FOLDED-MAY-OPT-REFUND-AUDIT).
    assert!(clause.optional, "\"may digivolve\" covers the whole effect");
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
#[test]
fn bt21_001_inherited_fires_on_your_turn_and_installs_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-P1"))
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
#[test]
fn bt21_001_declining_permanent_selection_no_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-P1"))
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
/// The fixture gap that kept this `#[ignore]`d (REPTILE-HAND had no
/// `evo_costs`, so no route existed) is closed: `make_reptile` now prints a
/// red Lv.3 route at cost 3, paid as 3 - 1 = 2.
#[test]
fn bt21_001_selecting_permanent_then_hand_card_triggers_digivolve_cost_minus_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-P1"))
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

    // REPTILE-HAND is the carrier's new top card, and the digivolution cost
    // 3 was paid reduced by 1.
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "REPTILE-HAND",
        "the carrier digivolved into REPTILE-HAND"
    );
    assert_eq!(runner.memory(), 8, "digivolution cost 3 reduced by 1");
}

// ─── Section 3b: candidate legality ─────────────────────────────────────────
// DCGO BT21_001.cs:44-53 `CanSelectDigivolvePermanent`: a Digimon is offered
// only if a [Reptile]/[Dragonkin] hand card satisfies
// `CanPlayCardTargetFrame(permanent.PermanentFrame, payCost: true, ...)` (a
// legal digivolve route that can be paid); :68 `maxCount = Min(1, count)`, so
// with none the pick isn't made. :95-102 `DigivolveIntoHandOrTrashCard(...,
// payCost: true, reduceCostTuple: (1, null), isHand: true)` then offers the
// hand cards that can digivolve onto that Digimon. DCGO's permanent check
// prices the full cost; the card's own reduction applies to that digivolve,
// so the cost is priced reduced by 1 (CR 4.3 1-3-11-1: a digivolution whose
// cost can't be paid can't be declared).

fn security_removed_runner(field_extra: &[&str], hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle, Vec<digimon_engine::permanent::PermanentHandle>) {
    let mut lv5 = make_test_card("LV5-BASE", "LV5-BASE");
    lv5.level = Some(5);
    let mut lv6 = make_test_card("REPTILE-LV6", "REPTILE-LV6");
    lv6.traits = vec!["Reptile".to_string()];
    lv6.level = Some(6);
    lv6.evo_costs = vec![EvoCost { card_color: CardColor::Red as u8, level: 5, memory_cost: 3 }];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(lv5)
        .add_card(make_non_digimon_security_filler("SEC-P1"))
        .add_card(make_reptile("REPTILE-HAND"))
        // 25 - 1 = 24 from 10 memory is -14, past the -10 floor.
        .add_card(make_reptile_costing("REPTILE-PRICY", 25))
        .add_card(lv6)
        .security(1, &["SEC-P1", "SEC-P1"])
        .hand(0, hand)
        .deck(0, &["CARRIER"; 4])
        .deck(1, &["CARRIER"; 4])
        .memory(10)
        .start();
    let carrier = place_gigimon_as_source(&mut runner, "CARRIER");
    let extra = field_extra
        .iter()
        .map(|id| runner.place_on_field(0, id, Some(0)))
        .collect();
    runner.attack_player(carrier, 1, false);
    (runner, carrier, extra)
}

fn offered_ids(runner: &DebugRunner) -> Vec<u16> {
    let mut v: Vec<u16> = runner
        .pending_selection_view()
        .expect("a pick is pending")
        .valid_action_ids
        .into_iter()
        .filter(|&id| id != digimon_engine::action::space::PASS)
        .collect();
    v.sort();
    v
}

/// A Digimon no [Reptile]/[Dragonkin] hand card can digivolve onto (the
/// Lv.5) is not offered; the red Lv.3 carrier is.
#[test]
fn bt21_001_permanent_pick_excludes_a_digimon_without_a_legal_digivolve() {
    let (runner, carrier, _extra) = security_removed_runner(&["LV5-BASE"], &["REPTILE-HAND"]);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert_eq!(
        offered_ids(&runner),
        vec![digimon_engine::action::space::encode_attack(0, carrier.index as u16)]
    );
}

/// The only matching hand card can't be paid for even with the -1: no
/// Digimon is offered, so no pick appears.
#[test]
fn bt21_001_unpayable_digivolve_offers_no_pick() {
    let (runner, _carrier, _extra) = security_removed_runner(&[], &["REPTILE-PRICY"]);
    assert!(
        runner.pending_selection().is_none(),
        "no payable digivolve: {:?}",
        runner.pending_selection_view().map(|v| (v.kind, v.prompt))
    );
}

/// After choosing the carrier, the hand pick offers only the card that can
/// digivolve onto it at a payable cost: REPTILE-PRICY (unpayable) and
/// REPTILE-LV6 (needs a Lv.5) are not offered.
#[test]
fn bt21_001_hand_pick_offers_only_payable_routes_onto_the_chosen_digimon() {
    let (mut runner, carrier, _extra) = security_removed_runner(
        &[],
        &["REPTILE-PRICY", "REPTILE-LV6", "REPTILE-HAND"],
    );
    runner
        .execute_action(0, digimon_engine::action::space::encode_attack(0, carrier.index as u16))
        .unwrap();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let offered: Vec<String> = offered_ids(&runner)
        .into_iter()
        .map(|id| hand[(id - digimon_engine::action::space::PLAY_HAND_START) as usize].clone())
        .collect();
    assert_eq!(offered, vec!["REPTILE-HAND".to_string()]);
}

// ─── Section 4: Cost firing / event assertions ───────────────────────────────
// N/A: BT21-001's inherited clause fires a digivolve, which uses normal
// digivolution cost-payment mechanics. No trash-security or discard events
// are produced by the clause itself.

// ─── Section 5: OPT lockout ─────────────────────────────────────────────────

/// Official Q&A Q1818 / CR 4.3 §15-14-1-4: passing the "may" is choosing not
/// to activate the effect, so a second security removal the same turn
/// offers it again.
#[test]
fn bt21_001_declined_trigger_is_offered_again_same_turn_q1818() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-1"))
        .add_card(make_non_digimon_security_filler("SEC-2"))
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

    // Second security removal in the same turn: the declined effect never
    // used its [Once Per Turn], so it is offered again. (Unsuspend the
    // carrier so the second attack is legal.)
    let carrier2 = runner.perm_handle(0, 0);
    runner.game.players[0].battle_area[carrier2.index as usize].is_suspended = false;
    let sec_before = runner.security_count(1);
    runner.attack_player(carrier2, 1, false);
    assert_eq!(runner.security_count(1), sec_before - 1, "the second attack removed security");

    assert!(
        runner.pending_selection().is_some(),
        "Q1818: a declined [Once Per Turn] 'may' effect is offered again the same turn"
    );
}

/// OPT resets after end_turn.
#[test]
fn bt21_001_opt_resets_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_non_digimon_security_filler("SEC-1"))
        .add_card(make_non_digimon_security_filler("SEC-2"))
        .add_card(make_non_digimon_security_filler("SEC-3"))
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

// ─── Section 6: breeding area (CR 4.3 §3-4-7-4) ─────────────────────────────

/// Gigimon under a Digimon in P0's BREEDING area: the inherited effect has no
/// breeding-area reference, so a security removal on P0's turn offers nothing
/// (CR 4.3 §3-4-7-4 / §3-4-7-6; DCGO `BT21_001.cs` `CanUseCondition` /
/// `CanActivateCondition` require `IsExistOnBattleAreaDigimon(card)`).
/// G-ENGINE-BREEDING-AREA-INHERITED-TRIGGERS-ACTIVE: the breeding fan-out
/// used to enqueue it (replay goldens 001 / 002 / 008).
#[test]
fn bt21_001_under_a_breeding_area_digimon_does_not_trigger() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 in embedded DSL pack")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_non_digimon_security_filler("SEC-P1"))
        .add_card(make_reptile("REPTILE-HAND"))
        .security(1, &["SEC-P1", "SEC-P1"])
        .hand(0, &["REPTILE-HAND"])
        .deck(0, &["CARRIER"; 4])
        .deck(1, &["CARRIER"; 4])
        .memory(10)
        .start();

    // Gigimon under CARRIER in P0's breeding area.
    let breeding = runner.place_stack(0, &["BT21-001", "CARRIER"]);
    let perm = runner.game.players[0]
        .battle_area
        .remove(breeding.index as usize);
    runner.game.players[0].breeding_area = Some(perm);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    assert_eq!(runner.security_count(1), 1, "one security card removed");
    assert!(
        runner.pending_selection().is_none(),
        "BT21-001 in the breeding area must not trigger: {:?}",
        runner.pending_selection()
    );
}
