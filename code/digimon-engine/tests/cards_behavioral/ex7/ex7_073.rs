//! EX7-073 BeelStarmon (X Antibody) — Digimon, Lv.6, Purple, DP 12000, Cost 12.
//! Traits: Wizard, X Antibody, Three Musketeers. Attribute: Virus. Form: Mega.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX7-073.md, verbatim)
//!
//! ```text
//! [When Digivolving] You may use 1 Option card with [Three Musketeers] in its
//! text from your hand without paying the cost.
//! [When Digivolving] [When Attacking] By trashing 2 cards with the [Three
//! Musketeers] trait from this Digimon's digivolution cards, delete 1 of your
//! opponent's Digimon with the highest level, and trash their top security
//! card.
//! ```
//!
//! Standard digivolve: Lv.5 Purple / Cost 4.
//! Special digivolution condition (printed xros_req black text): "[Digivolve]
//! Lv.6 w/[Three Musketeers] trait w/o [X Antibody] trait: Cost 1".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_073.cs
//!
//! # Faithfulness fix under test (2026-07-02)
//! DCGO (lines 246-275 / 387-416) runs `IDestroySecurity` UNCONDITIONALLY
//! inside the `if (trashed)` block once 2 sources are trashed — the mandatory
//! delete of the highest-level opponent Digimon is a SEPARATE, independently
//! gated `if (HasMatchConditionPermanent(...))` branch. A flat sibling list of
//! [select_opponent_permanent, delete_permanent, trash_top_security] would be
//! unfaithful: `install_select_opponent_permanent` short-circuits with a bare
//! `return` on an empty candidate set, dropping the composed tail
//! (`trash_top_security` included) whenever the opponent has 0 Digimon but
//! non-empty security. The delete is therefore wrapped in its own nested
//! `if any_permanent(...)` guard, with `trash_top_security` as a following
//! OUTER sibling (BT20-020 Clause 2 / BT21-024 "outer-tail sibling" idiom).
//!
//! # Patterns this test covers
//! - Digivolution alt-path: `not: { trait_has: ... }` negated-trait gate.
//! - [When Digivolving] optional free-play from hand: `select_hand` (kind:
//!   option, effect_text_contains) + `play_from_hand_free` (BT25-082 Clause 1
//!   sibling idiom).
//! - [When Digivolving] / [When Attacking] trash-2-own-sources cost gate:
//!   `select_own_sources` (min:0/max:2) + `binding_count_eq(n:2)` (EX4-073 /
//!   BT12-031 sibling idiom).
//! - Nested `if any_permanent` guard around a mandatory `select_opponent_permanent`
//!   + `delete_permanent`, with `trash_top_security` as an unconditional outer
//!   tail sibling — the reviewer-directed faithfulness fix.
//! - `level_matches_aggregate { selector: highest_level, of: opponent }`
//!   filter on a single mandatory select (not a `for_each`) — EX9-021 sibling
//!   idiom adapted from all-highest-level delete to single-highest-level pick.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX7-073";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A Purple Lv.5 base — the standard digivolution source for EX7-073
/// (Lv.5 Purple / cost 4).
fn purple_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "PurpleChampion");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(5);
    c.dp = Some(8000);
    c
}

/// A source card carrying the [Three Musketeers] trait — the trashable
/// digivolution-cost fodder for Clause 2/3.
fn tm_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "TMSource");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

/// A source card WITHOUT the [Three Musketeers] trait — must never be
/// selectable by the trash-2 clause.
fn plain_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "PlainSource");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Beast".to_string()];
    c
}

/// An opponent Digimon with a parameterized level, for the "highest level"
/// delete-target tests.
fn opp_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, "OppDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// An Option card whose printed text carries "Three Musketeers" — the
/// free-play target for Clause 1.
fn tm_option(id: &str) -> CardData {
    let mut c = make_test_card(id, "TMOption");
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.effect_text = "[Three Musketeers] Deal with it.".to_string();
    c
}

/// An Option card with NO [Three Musketeers] text — must not be a candidate.
fn plain_option(id: &str) -> CardData {
    let mut c = make_test_card(id, "PlainOption");
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.effect_text = "A perfectly ordinary option.".to_string();
    c
}

/// A Lv.6 base carrying [Three Musketeers] but NOT [X Antibody] — the alt
/// digivolve source for BeelStarmon (X Antibody) at cost 1.
fn tm_lv6_no_xa(id: &str) -> CardData {
    let mut c = make_test_card(id, "TMLv6NoXA");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(10000);
    c.traits = vec!["Three Musketeers".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 4,
    }];
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-073 YAML loads from the embedded pack")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

/// Digivolve EX7-073 (hand index 0) onto the given base slot via the standard
/// Lv.5 Purple path; the [When Digivolving] triggers fire inside this call.
fn digivolve_ex7_073(runner: &mut DebugRunner, base: PermanentHandle) {
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-073 must digivolve from a Purple Lv.5 base (cost 4)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "EX7-073 is now the top card of the digivolved stack"
    );
}

fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// EX7-073 queues TWO own-controller [When Digivolving] triggers (Clause 1
/// free-play + Clause 2 trash-2), so the drainer installs a `TriggerOrder`
/// selection letting the controller pick which resolves first. If one is
/// pending, resolve it by picking the entry whose `is_optional` flag matches
/// `want_optional` (Clause 1 is optional; Clause 2 is not). No-op if no
/// TriggerOrder selection is pending (only one clause queued this time).
fn resolve_trigger_order_if_pending(runner: &mut DebugRunner, want_optional: bool) {
    if runner.pending_kind() != Some(SelectionKind::TriggerOrder) {
        return;
    }
    let view = runner
        .pending_selection_view()
        .expect("TriggerOrder selection view");
    let choice = view
        .effect_choices
        .as_ref()
        .expect("TriggerOrder selection carries effect_choices")
        .iter()
        .find(|c| c.is_optional == want_optional)
        .expect("a trigger with the requested optionality must be queued");
    runner
        .execute_action(view.selecting_player, choice.action_id)
        .expect("resolve TriggerOrder pick");
}

/// Push a card identified by `card_id` into `player`'s security stack (top).
fn push_to_security_top(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("card {card_id} not found in card_data"));
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].security.insert(0, card);
}

/// Auto-resolve all remaining prompts by always taking the first legal
/// action — used once a specific branch decision is already locked in.
fn drain_prompts(runner: &mut DebugRunner) {
    while let Some(v) = runner.pending_selection_view() {
        let act = v
            .valid_action_ids
            .first()
            .copied()
            .expect("a resolvable action");
        runner.execute_action(v.selecting_player, act).ok();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_073_compiles_with_printed_stats_and_lv5_purple_path() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-073 in compiled cards");

    assert_eq!(compiled.card, CARD_ID);
    assert_eq!(compiled.name, "BeelStarmon (X Antibody)");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.color, vec![CompiledColor::Purple]);
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    assert!(
        compiled.traits.iter().any(|t| t == "Three Musketeers"),
        "printed [Three Musketeers] trait missing"
    );
    assert!(
        compiled.traits.iter().any(|t| t == "X Antibody"),
        "printed [X Antibody] trait missing"
    );

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5) || from.all_of.iter().any(|p| p.level_eq == Some(5)))
                        && (from.color_is == Some(CompiledColor::Purple)
                            || from
                                .all_of
                                .iter()
                                .any(|p| p.color_is == Some(CompiledColor::Purple)))
                })
        }),
        "BeelStarmon (X Antibody) must digivolve from a Purple Lv.5 for cost 4"
    );
}

/// The printed xros_req alt-path: Lv.6 base w/[Three Musketeers] trait w/o
/// [X Antibody] trait, cost 1.
#[test]
fn ex7_073_has_tm_no_xa_alt_path_cost1() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-073 in compiled cards");

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve && path.cost == Some(CompiledCost::Literal(1))
        }),
        "must have an alt-source digivolve path at cost 1; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Three triggered clauses: Clause 1 ([WD] optional free-play Option),
/// Clause 2 ([WD] trash-2/delete/security), Clause 3 ([WA] trash-2/delete/
/// security) — DCGO wires WD and WA as two independent ActivateClasses.
#[test]
fn ex7_073_has_three_triggered_clauses() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-073 in compiled cards");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
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
        "expected 3 triggered clauses (WD free-play, WD trash-2, WA trash-2); got {}",
        triggered.len()
    );

    let wd_count = triggered
        .iter()
        .filter(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .count();
    let wa_count = triggered
        .iter()
        .filter(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .count();
    assert_eq!(wd_count, 2, "two clauses fire on WhenDigivolving");
    assert_eq!(wa_count, 1, "one clause fires on WhenAttacking");
}

/// Clause 1 (free-play Option) is optional, face-up, not once-per-turn.
#[test]
fn ex7_073_clause1_is_optional_and_face_up() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-073 in compiled cards");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause1 = triggered
        .iter()
        .find(|t| {
            t.when == vec![CompiledTiming::WhenDigivolving] && t.optional
        })
        .expect("optional WD clause (free-play Option) must exist");
    assert_eq!(clause1.scope, CompiledScope::FaceUp);
    assert!(!clause1.once_per_turn);
}

/// Clause 2 (WD trash-2) is mandatory at clause level (the "you may" lives on
/// the inner select_own_sources min:0, not the outer trigger) and Clause 3
/// (WA trash-2) mirrors it.
#[test]
fn ex7_073_clause2_and_clause3_are_not_optional_at_clause_level() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-073 in compiled cards");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause2 = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenDigivolving] && !t.optional)
        .expect("mandatory-at-clause-level WD trash-2 clause must exist");
    assert_eq!(clause2.scope, CompiledScope::FaceUp);

    let clause3 = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("WA trash-2 clause must exist");
    assert!(!clause3.optional);
    assert_eq!(clause3.scope, CompiledScope::FaceUp);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1: [When Digivolving] may free-play TM-text Option
// ─────────────────────────────────────────────────────────────────────────────

fn wd_clause1_builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(tm_option("TM-OPT"))
        .add_card(plain_option("PLAIN-OPT"))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &[CARD_ID, "TM-OPT"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
}

/// POSITIVE: with a TM-text Option in hand, digivolving offers the optional
/// free-play prompt; accepting plays it without paying its cost.
#[test]
fn ex7_073_wd_may_free_play_tm_text_option_from_hand() {
    let mut runner = wd_clause1_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));

    digivolve_ex7_073(&mut runner, base);

    // Two own-controller WD triggers queue simultaneously (Clause 1 free-play
    // + Clause 2 trash-2); resolve the TriggerOrder pick in favor of the
    // optional Clause 1 first.
    resolve_trigger_order_if_pending(&mut runner, true);

    assert!(
        runner.pending_selection().is_some(),
        "the optional 'may play a TM-text Option' prompt must install"
    );
    assert!(
        runner.pending_is_optional(),
        "Clause 1 is 'you may' — the outer trigger must be optional"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional free-play trigger");
    runner
        .auto_resolve()
        .expect("pick the TM Option + free-play it");

    // BeelStarmon is now stacked on BASE (index 0); the Option card should
    // have moved off P0's hand into the trash (options resolve to trash).
    let hand_ids: Vec<String> = zone_ids(
        &runner
            .game
            .players
            .get(0)
            .map(|_| Vec::new())
            .unwrap_or_default(),
        &runner.game.card_data,
    );
    // Direct hand inspection: TM-OPT must no longer sit in hand.
    let still_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT");
    assert!(
        !still_in_hand,
        "the free-played Option must leave the hand"
    );
}

/// NEGATIVE: no TM-text Option in hand (only a plain one) → Clause 1 is a
/// silent no-op — no prompt installs.
#[test]
fn ex7_073_wd_no_fire_without_tm_text_option_in_hand() {
    let mut runner = base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(plain_option("PLAIN-OPT"))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &[CARD_ID, "PLAIN-OPT"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "BASE", Some(0));

    digivolve_ex7_073(&mut runner, base);
    drain_prompts(&mut runner);

    // Neither Clause 1 (no TM-text Option in hand) nor Clause 2 (no TM-trait
    // sources under BASE) has any candidate — both queued triggers must
    // resolve to no-ops. Draining any TriggerOrder/no-candidate prompts must
    // leave nothing pending and the field/hand unchanged.
    assert!(
        runner.pending_selection().is_none(),
        "no TM-text Option in hand and no TM-trait sources → both clauses resolve to no-ops"
    );
    let still_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "PLAIN-OPT");
    assert!(
        still_in_hand,
        "the non-matching Option must never be offered or played"
    );
}

/// Decline path: the player may decline the optional free-play; the Option
/// stays in hand.
#[test]
fn ex7_073_wd_decline_free_play_keeps_option_in_hand() {
    let mut runner = wd_clause1_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));

    digivolve_ex7_073(&mut runner, base);
    resolve_trigger_order_if_pending(&mut runner, true);

    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline the optional free-play trigger");
    // Clause 2's trash-2 prompt may still be pending (min:0, BASE has no TM
    // sources though, so it should also be absent) — drain defensively.
    drain_prompts(&mut runner);

    let still_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT");
    assert!(
        still_in_hand,
        "declining Clause 1 must leave the Option card in hand"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2/3: trash-2 TM sources → delete highest-level → security
// ─────────────────────────────────────────────────────────────────────────────

fn wd_clause2_builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(purple_lv5("BASE"))
        .add_card(tm_source("TM-A"))
        .add_card(tm_source("TM-B"))
        .add_card(plain_source("PLAIN-SRC"))
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(make_test_card("SEC-FODDER", "SecurityFodder"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
}

/// POSITIVE (mandatory pick-2, then delete + security): with 2 TM-trait
/// sources under the base and an opponent Digimon on the field plus
/// security, trashing both sources deletes the (sole) opponent Digimon and
/// trashes their top security card.
#[test]
fn ex7_073_wd_trash_two_sources_deletes_opp_digimon_and_trashes_security() {
    let mut runner = wd_clause2_builder()
        .add_card(opp_digimon("OPP-D", 4, 5000))
        .start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", "BASE"]);
    runner.place_on_field(1, "OPP-D", Some(0));
    push_to_security_top(&mut runner, 1, "OPP-D");
    let security_before = runner.security_count(1);

    digivolve_ex7_073(&mut runner, base);
    // Clause 1 also queues (no TM Option in hand, so it will no-op), so
    // resolve the TriggerOrder pick in favor of Clause 2 (mandatory-at-clause
    // level) first.
    resolve_trigger_order_if_pending(&mut runner, false);

    let view = runner
        .pending_selection_view()
        .expect("Clause 2 must offer the trash-2 source-select (2 TM sources exist)");
    assert_eq!(
        view.valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        2,
        "only the 2 TM-trait sources are candidates; PLAIN-SRC must be excluded"
    );

    let pick_a = encode_source_select(base.index as u16, 0).expect("TM-A source encodes");
    let pick_b = encode_source_select(base.index as u16, 1).expect("TM-B source encodes");
    runner.execute_action(0, pick_a).expect("pick TM-A");
    runner.execute_action(0, pick_b).expect("pick TM-B");
    drain_prompts(&mut runner);

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"TM-A".to_string()) && trash_ids.contains(&"TM-B".to_string()),
        "both trashed sources land in P0's trash; trash={trash_ids:?}"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the sole opponent Digimon (now the unique highest level) must be deleted"
    );
    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "the opponent's top security card must be trashed"
    );
}

/// PARTIAL PICK: the player picks only 1 of the 2 available TM sources
/// (declines to complete the cost) — `binding_count_eq(n:2)` is false, so
/// NEITHER the delete NOR the security trash fires, but the 1 picked source
/// is still trashed (the pick itself is committed once made).
#[test]
fn ex7_073_wd_partial_pick_of_one_source_does_not_trigger_delete_or_security() {
    let mut runner = wd_clause2_builder()
        .add_card(opp_digimon("OPP-D", 4, 5000))
        .start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", "BASE"]);
    runner.place_on_field(1, "OPP-D", Some(0));
    push_to_security_top(&mut runner, 1, "OPP-D");
    let security_before = runner.security_count(1);

    digivolve_ex7_073(&mut runner, base);
    resolve_trigger_order_if_pending(&mut runner, false);

    let view = runner
        .pending_selection_view()
        .expect("trash-2 prompt installs");
    assert!(
        view.valid_action_ids.contains(&PASS),
        "min:0 → PASS must be offered before any pick"
    );
    let pick_a = encode_source_select(base.index as u16, 0).expect("TM-A source encodes");
    runner.execute_action(0, pick_a).expect("pick only TM-A");
    // After 1 pick (min:0 already satisfied), PASS must still be available —
    // the player may stop at 1.
    let view2 = runner
        .pending_selection_view()
        .expect("selection continues after 1 of 2 picks");
    assert!(
        view2.valid_action_ids.contains(&PASS),
        "after 1 pick, PASS must still be offered (player may stop short of 2)"
    );
    runner
        .execute_action(0, PASS)
        .expect("stop after picking only 1 source");
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the opponent Digimon must survive — only 1 source was trashed (count != 2)"
    );
    assert_eq!(
        runner.security_count(1),
        security_before,
        "security must NOT be trashed when fewer than 2 sources were trashed"
    );
}

/// FULL DECLINE: the player declines the trash-2 prompt entirely (0 picked)
/// — no trash, no delete, no security loss.
#[test]
fn ex7_073_wd_decline_trash_prompt_entirely_does_nothing() {
    let mut runner = wd_clause2_builder()
        .add_card(opp_digimon("OPP-D", 4, 5000))
        .start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", "BASE"]);
    runner.place_on_field(1, "OPP-D", Some(0));
    push_to_security_top(&mut runner, 1, "OPP-D");
    let security_before = runner.security_count(1);
    let trash_before = runner.trash_size(0);

    digivolve_ex7_073(&mut runner, base);
    resolve_trigger_order_if_pending(&mut runner, false);

    let view = runner
        .pending_selection_view()
        .expect("trash-2 prompt installs");
    assert!(view.valid_action_ids.contains(&PASS));
    runner
        .execute_action(0, PASS)
        .expect("decline the trash-2 cost entirely");
    drain_prompts(&mut runner);

    assert_eq!(runner.trash_size(0), trash_before, "nothing trashed on decline");
    assert_eq!(runner.battle_area_size(1), 1, "opponent Digimon survives");
    assert_eq!(
        runner.security_count(1),
        security_before,
        "security untouched on decline"
    );
}

/// FAITHFULNESS FIX — the core regression test: the opponent has SECURITY
/// CARDS but ZERO Digimon on the battle area. Trashing both TM sources must
/// still trash the opponent's top security card (DCGO runs IDestroySecurity
/// unconditionally once `trashed == true`), even though the delete sub-step
/// has no legal target and must be skipped.
#[test]
fn ex7_073_wd_trash_two_sources_with_no_opp_digimon_still_trashes_security() {
    let mut runner = wd_clause2_builder().start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", "BASE"]);
    // Zero opponent Digimon on the battle area — but seed security cards.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "fixture: opponent has no battle-area Digimon"
    );
    push_to_security_top(&mut runner, 1, "SEC-FODDER");
    push_to_security_top(&mut runner, 1, "SEC-FODDER");
    let security_before = runner.security_count(1);
    assert_eq!(security_before, 2, "fixture: opponent has 2 security cards");

    digivolve_ex7_073(&mut runner, base);
    resolve_trigger_order_if_pending(&mut runner, false);

    let view = runner
        .pending_selection_view()
        .expect("trash-2 prompt installs — 2 TM sources exist under BASE");
    let pick_a = encode_source_select(base.index as u16, 0).expect("TM-A source encodes");
    let pick_b = encode_source_select(base.index as u16, 1).expect("TM-B source encodes");
    runner.execute_action(0, pick_a).expect("pick TM-A");
    runner.execute_action(0, pick_b).expect("pick TM-B");
    drain_prompts(&mut runner);

    // No delete-target selection may install (no opponent Digimon exists);
    // drain_prompts already resolved anything that did appear, so verifying
    // no panic occurred plus the security count is the load-bearing check.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "still zero opponent Digimon — nothing to delete"
    );
    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "the mandatory security trash must fire even though the delete sub-step \
         had no legal target — this is the reviewer-directed faithfulness fix \
         (a flat sibling list would have silently dropped this trash via \
         install_select_opponent_permanent's empty-candidate short-circuit)"
    );
}

/// "Highest level" targeting: with opponents at Lv.4 and Lv.6 on the field,
/// only the Lv.6 (the higher) may be offered as the delete target.
#[test]
fn ex7_073_wd_delete_targets_highest_level_opponent_only() {
    let mut runner = wd_clause2_builder()
        .add_card(opp_digimon("OPP-LOW", 4, 5000))
        .add_card(opp_digimon("OPP-HIGH", 6, 9000))
        .start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", "BASE"]);
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    digivolve_ex7_073(&mut runner, base);
    resolve_trigger_order_if_pending(&mut runner, false);

    let pick_a = encode_source_select(base.index as u16, 0).expect("TM-A source encodes");
    let pick_b = encode_source_select(base.index as u16, 1).expect("TM-B source encodes");
    runner.execute_action(0, pick_a).expect("pick TM-A");
    runner.execute_action(0, pick_b).expect("pick TM-B");

    let delete_view = runner
        .pending_selection_view()
        .expect("mandatory delete-target select installs after 2 sources trashed");
    let candidate_count = delete_view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .count();
    assert_eq!(
        candidate_count, 1,
        "only the Lv.6 (highest level) opponent Digimon is a candidate"
    );
    assert!(
        !delete_view.is_optional,
        "the delete-target select is mandatory (DCGO canNoSelect: false)"
    );

    drain_prompts(&mut runner);

    let survivors: Vec<String> = zone_ids(
        &runner
            .game
            .players[1]
            .battle_area
            .iter()
            .map(|p| p.top_card().clone())
            .collect::<Vec<_>>(),
        &runner.game.card_data,
    );
    assert!(
        !survivors.contains(&"OPP-HIGH".to_string()),
        "the Lv.6 (highest) opponent Digimon must be deleted; survivors={survivors:?}"
    );
    assert!(
        survivors.contains(&"OPP-LOW".to_string()),
        "the Lv.4 (lower) opponent Digimon must survive; survivors={survivors:?}"
    );
}

/// [When Attacking] mirror: the same trash-2/delete/security shape fires on
/// attack, independent of the [When Digivolving] clause.
#[test]
fn ex7_073_wa_trash_two_sources_deletes_opp_digimon_and_trashes_security() {
    let mut runner = wd_clause2_builder()
        .add_card(opp_digimon("OPP-D", 4, 5000))
        .start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", CARD_ID]);
    runner.place_on_field(1, "OPP-D", Some(0));
    push_to_security_top(&mut runner, 1, "OPP-D");
    let security_before = runner.security_count(1);

    fire_when_attacking(&mut runner, base);

    let view = runner
        .pending_selection_view()
        .expect("WA clause offers the trash-2 source-select");
    let pick_a = encode_source_select(base.index as u16, 0).expect("TM-A source encodes");
    let pick_b = encode_source_select(base.index as u16, 1).expect("TM-B source encodes");
    runner.execute_action(0, pick_a).expect("pick TM-A");
    runner.execute_action(0, pick_b).expect("pick TM-B");
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent Digimon must be deleted on the WA trigger"
    );
    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "opponent top security must be trashed on the WA trigger"
    );
}

/// [When Attacking] faithfulness mirror: no opponent Digimon but security
/// present — the security trash must still fire.
#[test]
fn ex7_073_wa_trash_two_sources_with_no_opp_digimon_still_trashes_security() {
    let mut runner = wd_clause2_builder().start();
    let base = runner.place_stack(0, &["TM-A", "TM-B", CARD_ID]);
    push_to_security_top(&mut runner, 1, "SEC-FODDER");
    let security_before = runner.security_count(1);
    assert_eq!(security_before, 1);

    fire_when_attacking(&mut runner, base);

    let pick_a = encode_source_select(base.index as u16, 0).expect("TM-A source encodes");
    let pick_b = encode_source_select(base.index as u16, 1).expect("TM-B source encodes");
    runner.execute_action(0, pick_a).expect("pick TM-A");
    runner.execute_action(0, pick_b).expect("pick TM-B");
    drain_prompts(&mut runner);

    assert_eq!(runner.battle_area_size(1), 0, "still zero opponent Digimon");
    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "WA mirror: the mandatory security trash must fire even with no delete target"
    );
}

/// Only TM-trait sources are candidates — PLAIN-SRC (no [Three Musketeers]
/// trait) must never appear in the trash-2 selection menu.
#[test]
fn ex7_073_wd_only_tm_trait_sources_are_selectable() {
    let mut runner = wd_clause2_builder().start();
    let base = runner.place_stack(0, &["PLAIN-SRC", "TM-A", "TM-B", "BASE"]);

    digivolve_ex7_073(&mut runner, base);
    resolve_trigger_order_if_pending(&mut runner, false);

    let view = runner
        .pending_selection_view()
        .expect("trash-2 prompt installs");
    let plain_action =
        encode_source_select(base.index as u16, 0).expect("PLAIN-SRC source encodes");
    assert!(
        !view.valid_action_ids.contains(&plain_action),
        "PLAIN-SRC (no [Three Musketeers] trait) must not be a candidate; valid={:?}",
        view.valid_action_ids
    );
}

/// No TM-trait sources at all under the base → Clause 2/3's select_own_sources
/// has 0 candidates; with min:0 the tail still runs with an empty binding,
/// so `binding_count_eq(n:2)` is false and nothing fires — no panic, no
/// selection installs (the empty-candidate finalize path is synchronous).
#[test]
fn ex7_073_wd_no_tm_sources_silently_skips_trash_clause() {
    let mut runner = wd_clause2_builder()
        .add_card(opp_digimon("OPP-D", 4, 5000))
        .start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(1, "OPP-D", Some(0));
    push_to_security_top(&mut runner, 1, "OPP-D");
    let security_before = runner.security_count(1);

    digivolve_ex7_073(&mut runner, base);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "no sources to trash → the delete never fires"
    );
    assert_eq!(
        runner.security_count(1),
        security_before,
        "no sources to trash → the security trash never fires"
    );
}
