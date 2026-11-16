//! BT25-003 Frimon — Digi-Egg, Lv.2, Yellow, Cost 0.
//! Traits: Lesser, Glowing Dawn, BEATBREAK. Form: In-Training.
//!
//! # Printed text (data/cards.json — verbatim)
//! The face has NO effect. The INHERITED effect is the whole card:
//!   Inherited [When Attacking] [Once Per Turn] By trashing your top security
//!   card, this Digimon may digivolve into a [Glowing Dawn] trait card in the
//!   hand with the cost reduced by 1.
//!
//! (cards.json also surfaces the same text under the face `effect_description`
//!  with an "Inherited Effect" prefix; it is authored as an inherited-scope
//!  clause only — the egg face is blank.)
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_002.cs` — the file is
//! misnamed; it holds `class BT25_003` (`// Frimon`). Its inherited
//! `OnAllyAttack` clause:
//! - `SetUpActivateClass(CanActivateCondition, ActivateCoroutine, 1, true, ...)`
//!   — [Once Per Turn], optional (the initial Yes/No is the opt-in to pay
//!   "By trashing your top security card");
//! - `CanActivateCondition` = `IsExistOnBattleAreaDigimon && SecurityCards.Any()`
//!   — ONLY the cost's payability, no payoff check;
//! - `ActivateCoroutine`: `IDestroySecurity(fromTop: true)` unconditionally,
//!   THEN `if (HasMatchConditionOwnersHand(IsExistOnHand && EqualsTraits
//!   ("Glowing Dawn")))` → `DigivolveIntoHandOrTrashCard(payCost: true,
//!   reduceCostTuple: (1, null), isHand: true)`, whose pick is optional
//!   (`isOptional = true` → `canNoSelect`) and offers only `IsDigimon &&
//!   cardCondition && !CanNotEvolve(target) && CanPlayCardTargetFrame(PayCost:
//!   true, ...)` (`Script/CardEffectCommons.cs:859-881`).
//!
//! # Rules (CR 4.3)
//! - 15-7-1 / 15-7-2: "By trashing your top security card" is an optional
//!   processing condition; the digivolve happens only if it was executed.
//! - 15-7-4: with no security card the condition can't be executed, so it
//!   can't be chosen (no prompt).
//! - 15-7-5: the player may trash the top security card even when the
//!   digivolve after it can't be executed (no [Glowing Dawn] Digimon in hand,
//!   or none payable). That trash is a real, game-changing choice.
//! - 1-3-11-1: only a card that can digivolve onto this Digimon and whose
//!   (reduced) cost can be paid is a digivolve candidate.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Group G: inherited [When Attacking][Once Per Turn] (single clause)
//! - `optional: true` clause whose first step is the (non-declinable)
//!   `trash_top_security` → the outer accept/decline prompt is the opt-in
//! - effect-initiated digivolve into a HAND card at a PAID reduced cost
//!   (`cost: { reduce: 1 }`, requirements honored) — `target: source`

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT25-003";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.3 Yellow Glowing-Dawn carrier — the Digimon that BT25-003 sits beneath.
/// Lv.3 yellow satisfies the digivolve-target's evo-cost (color Yellow, level 3).
fn carrier_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// The digivolve-INTO target: a Lv.4 Yellow Digimon with the [Glowing Dawn]
/// trait and a synthetic printed digivolution cost of 4 from a Lv.3 yellow.
/// Reduced by 1 → 3 memory paid.
fn gd_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "GD Target");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    // Evo-cost color codes: Red=0, Blue=1, Yellow=2, Green=3, ...
    c.evo_costs = vec![EvoCost {
        card_color: 2, // Yellow
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A Lv.4 NON-Glowing-Dawn Digimon (same digivolution route) — illegal pick
/// for the [Glowing Dawn]-trait hand filter.
fn non_gd_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "Non GD Target");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["Mammal".to_string()]; // no Glowing Dawn
    c.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 4,
    }];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-003 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(gd_target("GD-TARGET"))
        .add_card(non_gd_target("NON-GD"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER", "FILLER"])
        .memory(10)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

/// Assert the outer "By trashing your top security card" opt-in is pending.
fn expect_outer_prompt(r: &DebugRunner, why: &str) {
    let view = r
        .pending_selection_view()
        .unwrap_or_else(|| panic!("outer opt-in prompt must be pending: {why}"));
    assert_eq!(
        view.kind,
        SelectionKind::Replacement,
        "the first prompt is the accept/decline of the optional processing \
         condition (CR 4.3 15-7-1): {why}"
    );
}

fn top_id(r: &DebugRunner, carrier: PermanentHandle) -> String {
    r.game.player(0).battle_area[carrier.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// Place BT25-003 (Frimon egg) as the bottom digivolution source under a Lv.3
/// yellow Glowing-Dawn carrier.
fn carrier_over_frimon(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "CARRIER"])
}

/// Recursively search a process for a `TrashTopSecurity` step.
fn process_contains_trash_top_security(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::TrashTopSecurity { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_trash_top_security(then)
                || process_contains_trash_top_security(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_trash_top_security(body)
        }
        _ => false,
    })
}

/// Recursively search a process for a requirements-honored effect-initiated
/// digivolve (the `if { binding_present }` guard wraps it).
fn process_contains_requirements_honored_digivolve(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::EffectInitiatedDigivolve {
            ignore_requirements: false,
            ..
        } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_requirements_honored_digivolve(then)
                || process_contains_requirements_honored_digivolve(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_requirements_honored_digivolve(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_003_is_yellow_digi_egg_with_glowing_dawn_traits() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Frimon");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
}

#[test]
fn bt25_003_only_effect_is_inherited_when_attacking_opt() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    // Exactly one triggered clause; it is inherited, [When Attacking], OPT.
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "BT25-003 has a single triggered clause");
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited, "the clause is inherited");
    assert!(
        t.when.contains(&CompiledTiming::WhenAttacking),
        "the inherited clause fires [When Attacking]"
    );
    assert!(t.once_per_turn, "[Once Per Turn]");
    // "By trashing your top security card" is an optional processing
    // condition (CR 4.3 15-7-1): the clause is optional, and its first body
    // step is the trash itself, so the outer accept/decline is the opt-in
    // (DCGO BT25_002.cs `class BT25_003`: SetUpActivateClass(..., 1, true, ...)).
    assert!(t.optional, "paying the trash cost is the player's choice");
    assert!(
        matches!(t.process.first(), Some(CompiledStep::TrashTopSecurity { .. })),
        "the trash is paid first, independent of the digivolve pick (CR 4.3 15-7-5)"
    );
    // "this Digimon may digivolve" — the pick after paying is still optional
    // (DCGO DigivolveIntoHandOrTrashCard isOptional = true → canNoSelect).
    let has_optional_hand_pick = t
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectHand { optional: true, .. }));
    assert!(
        has_optional_hand_pick,
        "printed 'may digivolve' → an optional select_hand"
    );
    assert!(process_contains_trash_top_security(&t.process));
    assert!(
        process_contains_requirements_honored_digivolve(&t.process),
        "an effect-initiated digivolve (requirements honored) is present"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: may digivolve into a [Glowing Dawn] hand card -1 cost
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: accept the opt-in, the top security card is trashed, then the
/// carrier digivolves into the [Glowing Dawn] hand card at cost 4 - 1 = 3.
#[test]
fn bt25_003_may_digivolve_into_glowing_dawn_with_cost_reduced_1() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    // Hand: the legal Glowing-Dawn target, plus a non-Glowing-Dawn Lv.4 (illegal).
    push_to_hand(&mut r, 0, "GD-TARGET");
    push_to_hand(&mut r, 0, "NON-GD");
    let mem_before = r.memory();
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "security is present");
    r.accept_optional_trigger().expect("accept the opt-in");
    assert_eq!(
        r.security_count(0),
        sec_before - 1,
        "accepting pays the cost: the top security card is trashed"
    );
    let view = r
        .pending_selection_view()
        .expect("the 'you may digivolve' hand pick must surface");
    assert!(
        view.is_optional,
        "printed 'may digivolve' ⇒ the pick is declinable"
    );
    let picks: Vec<_> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(
        picks.len(),
        1,
        "only the Lv.4 [Glowing Dawn] hand card is a legal pick (not the \
         non-Glowing-Dawn Lv.4); got {:?}",
        view.valid_action_ids
    );
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(top_id(&r, carrier), "GD-TARGET", "the carrier digivolves");
    assert_eq!(r.security_count(0), sec_before - 1, "exactly one security trashed");
    assert_eq!(
        r.memory(),
        mem_before - 3,
        "the digivolution cost (4) is reduced by 1 → pays 3"
    );
}

/// Declining the opt-in: nothing happens — no security trashed, no digivolve,
/// no memory paid — and the [Once Per Turn] is not used, so a later
/// [When Attacking] the same turn offers the effect again (DCGO's skippable
/// Yes/No drops the effect before its use is recorded).
#[test]
fn bt25_003_declining_leaves_security_and_keeps_once_per_turn() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");
    let mem_before = r.memory();
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "first attack");
    r.decline_optional_trigger().expect("declining is legal");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(top_id(&r, carrier), "CARRIER", "declined ⇒ no digivolve");
    assert_eq!(r.security_count(0), sec_before, "declined ⇒ no security trashed");
    assert_eq!(r.memory(), mem_before, "declined ⇒ no cost paid");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "a declined activation does not use [Once Per Turn]");
}

/// Accepting the opt-in and then passing the pick: the trash was already paid
/// (CR 4.3 15-7-5); "may digivolve" still lets the player skip the digivolve
/// (DCGO DigivolveIntoHandOrTrashCard isOptional = true).
#[test]
fn bt25_003_pick_is_declinable_after_paying() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");
    let mem_before = r.memory();
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    r.accept_optional_trigger().expect("accept the opt-in");
    let view = r.pending_selection_view().expect("the pick surfaces");
    assert!(view.is_optional);
    r.execute_action(0, PASS).expect("passing the pick is legal");
    let _ = r.auto_resolve();

    assert_eq!(top_id(&r, carrier), "CARRIER", "no digivolve");
    assert_eq!(r.security_count(0), sec_before - 1, "the trash cost was paid");
    assert_eq!(r.memory(), mem_before, "no digivolve cost paid");
}

/// No [Glowing Dawn] card in hand: the opt-in is still offered (CR 4.3 15-7-5;
/// DCGO's CanActivateCondition checks only that security exists). Accepting
/// trashes the top security card; no pick installs.
#[test]
fn bt25_003_no_glowing_dawn_in_hand_still_offers_trash() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "NON-GD");
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "the trash is choosable without a payoff");
    r.accept_optional_trigger().expect("accept the opt-in");
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no [Glowing Dawn] candidate ⇒ no pick installs"
    );
    assert_eq!(top_id(&r, carrier), "CARRIER", "nothing digivolved");
    assert_eq!(r.security_count(0), sec_before - 1, "the top security card is trashed");
}

/// NEGATIVE (cost gate): with NO security cards the trash-top-security cost
/// can't be executed, so it can't be chosen (CR 4.3 15-7-4) — no prompt.
#[test]
fn bt25_003_no_activation_without_security_to_trash() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-003 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(gd_target("GD-TARGET"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        // No own security cards.
        .memory(10)
        .start();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no security to pay the cost ⇒ effect cannot activate, no prompt"
    );
    assert_eq!(top_id(&r, carrier), "CARRIER", "no digivolve when the cost is unpayable");
    assert_eq!(r.memory(), mem_before, "no cost paid");
}

/// [Once Per Turn]: a second [When Attacking] in the same turn cannot
/// re-activate. (Take the first digivolve, then re-fire and assert no prompt.)
#[test]
fn bt25_003_inherited_digivolve_is_once_per_turn() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    r.accept_optional_trigger().expect("accept the opt-in");
    let view = r
        .pending_selection_view()
        .expect("first [When Attacking] offers the digivolve");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "GD-TARGET", "first activation digivolved");

    // The carrier is now GD-TARGET (BT25-003 still underneath it as a source).
    push_to_hand(&mut r, 0, "GD-TARGET");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] ⇒ the second [When Attacking] must not re-offer the effect"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — the digivolve pick offers only cards that can digivolve AND be paid
// ════════════════════════════════════════════════════════════════════════════
//
// DCGO `class BT25_003` (file BT25_002.cs) calls the shared helper
// `CardEffectCommons.DigivolveIntoHandOrTrashCard` (CardEffectCommons.cs:
// 859-881), which offers only `IsDigimon && cardCondition &&
// !CanNotEvolve(target) && CanPlayCardTargetFrame(target frame, PayCost: true,
// ...)` — a card that meets the digivolution requirements onto THIS Digimon
// and whose (reduced) cost can be paid. CR 4.3 1-3-11-1: a card can be
// declared for digivolution only if it can be digivolved and its cost paid.
// [Glowing Dawn] also names Tamers (BT25-088 Kyo Sawashiro), Options (P-236)
// and Digi-Eggs (this card): none can digivolve onto a Digimon, so none is
// offered. The trash cost itself is offered regardless (CR 4.3 15-7-5).

fn gd_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "GD Tamer");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A [Glowing Dawn] Lv.4 whose digivolution cost (12, reduced to 11) can't be
/// paid from 0 memory (0 - 11 = -11, past the opponent's 10).
fn gd_pricey(id: &str) -> CardData {
    let mut c = gd_target(id);
    c.card_name = "GD Pricey".to_string();
    c.evo_costs = vec![EvoCost { card_color: 2, level: 3, memory_cost: 12 }];
    c
}

fn runner_with(extra: Vec<CardData>, memory: i16) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-003 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(gd_target("GD-TARGET"))
        .add_card(filler("FILLER"));
    for c in extra {
        b = b.add_card(c);
    }
    let mut r = b
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER", "FILLER"])
        .memory(memory)
        .start();
    r.set_first_player(0);
    r
}

/// An unpayable [Glowing Dawn] card is not offered as a digivolve candidate.
/// The trash opt-in is still offered (CR 4.3 15-7-5); after paying it, no pick
/// installs and nothing digivolves.
#[test]
fn bt25_003_unaffordable_glowing_dawn_not_offered() {
    let mut r = runner_with(vec![gd_pricey("GD-PRICEY")], 0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-PRICEY");
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "the trash is choosable without a payable digivolve");
    r.accept_optional_trigger().expect("accept the opt-in");
    assert!(
        r.game.pending_selection.is_none(),
        "the 11-cost digivolve can't be paid from 0 memory: no pick is offered"
    );
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec_before - 1, "the trash cost was paid");
    assert_eq!(top_id(&r, carrier), "CARRIER");
    assert_eq!(r.memory(), 0, "no digivolve cost paid");
}

/// The same 12-cost card is offered when 1 more memory makes 11 payable
/// (1 - 11 = -10): the filter is the payment check, not a cost cap.
#[test]
fn bt25_003_glowing_dawn_offered_when_reduced_cost_is_exactly_payable() {
    let mut r = runner_with(vec![gd_pricey("GD-PRICEY")], 1);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-PRICEY");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    r.accept_optional_trigger().expect("accept the opt-in");
    let view = r
        .pending_selection_view()
        .expect("payable at exactly -10: the pick installs");
    assert_eq!(view.valid_action_ids.iter().filter(|&&a| a != PASS).count(), 1);
}

/// A [Glowing Dawn] Tamer can't digivolve onto a Digimon, but the effect is
/// still offered (CR 4.3 15-7-5). Accepting trashes the top security card,
/// digivolves nothing, and uses the [Once Per Turn]: a later [When Attacking]
/// the same turn does not offer it again, even with a real candidate in hand.
#[test]
fn bt25_003_tamer_only_hand_accept_trashes_and_spends_once_per_turn() {
    let mut r = runner_with(vec![gd_tamer("GD-TAMER")], 10);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TAMER");
    let sec_before = r.security_count(0);
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "a Tamer-only hand still allows the trash");
    r.accept_optional_trigger().expect("accept the opt-in");
    assert!(
        r.game.pending_selection.is_none(),
        "a [Glowing Dawn] Tamer is not a digivolve candidate"
    );
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec_before - 1, "the top security card is trashed");
    assert_eq!(top_id(&r, carrier), "CARRIER", "nothing digivolved");
    assert_eq!(r.memory(), mem_before);

    push_to_hand(&mut r, 0, "GD-TARGET");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "the accepted activation used the [Once Per Turn]"
    );
    assert_eq!(r.security_count(0), sec_before - 1, "no second trash");
}

/// A [Glowing Dawn] Tamer-only hand, declined: security unchanged, and the
/// effect is offered again on a later [When Attacking] the same turn.
#[test]
fn bt25_003_tamer_only_hand_decline_keeps_security_and_once_per_turn() {
    let mut r = runner_with(vec![gd_tamer("GD-TAMER")], 10);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TAMER");
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "a Tamer-only hand still allows the trash");
    r.decline_optional_trigger().expect("declining is legal");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec_before, "declined ⇒ no security trashed");

    push_to_hand(&mut r, 0, "GD-TARGET");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    expect_outer_prompt(&r, "a declined activation does not use [Once Per Turn]");
    r.accept_optional_trigger().expect("accept the opt-in");
    let view = r
        .pending_selection_view()
        .expect("the real [Glowing Dawn] Digimon is now offered");
    assert_eq!(view.valid_action_ids.iter().filter(|&&a| a != PASS).count(), 1);
}
