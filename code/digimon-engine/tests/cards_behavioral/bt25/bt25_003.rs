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
//! BT25-003 is NOT implemented in the DCGO submodule (no BT25_003.cs). The
//! behavioral target is therefore the printed text + general_rule.pdf + the
//! shape of the comparable BT12-016 / BT17-015 effect-initiated digivolve
//! (DigivolveIntoHandOrTrashCard, payCost: true, reduceCost 1, isHand) gated by
//! a trash-top-security activation cost (NyaromonBT15-003 idiom for the cost).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Group G: inherited [When Attacking][Once Per Turn] (single clause)
//! - activation cost: trash_top_security (NyaromonBT15-003 idiom)
//! - effect-initiated digivolve into a HAND card at a PAID reduced cost
//!   (`cost: { reduce: 1 }`, requirements honored) — `target: source` (the
//!   carrier of the inherited effect), the working BT12-016 shape (NOT the
//!   blocked select-own-permanent target shape of BT17-027).
//! - "may" → optional; security-count gate so the cost is payable

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
use digimon_engine::selection::TriggerSource;

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

/// Place BT25-003 (Frimon egg) as the bottom digivolution source under a Lv.3
/// yellow Glowing-Dawn carrier.
fn carrier_over_frimon(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "CARRIER"])
}

/// Recursively search a process for a `TrashTopSecurity` step (it lives inside
/// the `if { binding_present }` guard).
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
    // The printed "may digivolve" optionality is carried by the inner optional
    // `select_hand` (so declining leaves the trash cost unpaid), NOT the
    // clause-level outer-optional flag — the BT12-016 idiom. The behavioral
    // `bt25_003_digivolve_is_declinable` test proves the decline path.
    let has_optional_hand_pick = t
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectHand { optional: true, .. }));
    assert!(
        has_optional_hand_pick,
        "printed 'may digivolve' → an optional select_hand carries the decline"
    );

    // The body installs the trash-top-security cost + an effect-initiated
    // digivolve at cost: reduce 1 (requirements honored). Both live inside an
    // `if { binding_present }` guard (the trash is only paid on a real pick),
    // so search recursively.
    assert!(
        process_contains_trash_top_security(&t.process),
        "activation cost trashes the top security card"
    );
    assert!(
        process_contains_requirements_honored_digivolve(&t.process),
        "an effect-initiated digivolve (requirements honored) is present"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: may digivolve into a [Glowing Dawn] hand card -1 cost
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with security to trash and a legal [Glowing Dawn] Lv.4 in hand,
/// the carrier digivolves into it; the top security card is trashed (cost) and
/// the digivolution cost is reduced by 1 (printed 4 → pay 3).
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
    let view = r
        .pending_selection_view()
        .expect("the 'you may digivolve' hand pick must surface");
    assert!(
        view.is_optional,
        "printed 'may digivolve' ⇒ the pick is declinable"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Lv.4 [Glowing Dawn] hand card is a legal pick (not the \
         non-Glowing-Dawn Lv.4); got {:?}",
        view.valid_action_ids
    );
    let pick = view.valid_action_ids[0];
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();

    let perm = &r.game.player(0).battle_area[carrier.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "GD-TARGET",
        "the carrier digivolves into the [Glowing Dawn] hand card"
    );
    assert_eq!(
        r.security_count(0),
        sec_before - 1,
        "the activation cost trashes the top security card"
    );
    assert_eq!(
        r.memory(),
        mem_before - 3,
        "the digivolution cost (4) is reduced by 1 → pays 3"
    );
}

/// The digivolve is DECLINABLE ("may"): PASS leaves the board and security
/// stack unchanged (the trash cost is only paid if the digivolve is taken).
#[test]
fn bt25_003_digivolve_is_declinable() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");
    let mem_before = r.memory();
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("the optional hand pick surfaces");
    assert!(view.is_optional);
    r.execute_action(0, PASS)
        .expect("declining the 'you may digivolve' is legal");
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "declined ⇒ still the original carrier"
    );
    assert_eq!(
        r.security_count(0),
        sec_before,
        "declined ⇒ no security trashed"
    );
    assert_eq!(r.memory(), mem_before, "declined ⇒ no cost paid");
}

/// NEGATIVE (filter): no legal [Glowing Dawn] in hand ⇒ clean no-op, no stuck
/// selection. Only a non-Glowing-Dawn Lv.4 is in hand.
#[test]
fn bt25_003_no_glowing_dawn_in_hand_clean_noop() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_frimon(&mut r);
    push_to_hand(&mut r, 0, "NON-GD");
    let sec_before = r.security_count(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no legal [Glowing Dawn] pick ⇒ no stuck selection"
    );
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "nothing digivolved"
    );
    assert_eq!(
        r.security_count(0),
        sec_before,
        "no digivolve ⇒ no security trashed"
    );
}

/// NEGATIVE (cost gate): with NO security cards the trash-top-security cost is
/// unpayable, so the effect cannot activate — no prompt, clean no-op.
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
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "no digivolve when the cost is unpayable"
    );
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

    // First activation — take the digivolve.
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("first [When Attacking] offers the digivolve");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "GD-TARGET",
        "first activation digivolved"
    );

    // The carrier is now GD-TARGET (BT25-003 still underneath it as a source).
    // A second [When Attacking] in the same turn must not re-offer the
    // inherited digivolve (OPT lockout). Put another GD card in hand to prove
    // it's the OPT gate, not an empty hand.
    push_to_hand(&mut r, 0, "GD-TARGET");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] ⇒ the second [When Attacking] must not re-offer the digivolve"
    );
}
