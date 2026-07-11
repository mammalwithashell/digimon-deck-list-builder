//! BT24-084 Inori Misono — Tamer, Cost 3, Yellow. Traits: [TS].
//!
//! # Printed card text (official Bandai DB — data/card_bundles/BT24-084.md)
//!
//! **[Start of Your Main Phase]** If you have 4 or less memory, gain 1 memory.
//! **[All Turns]** When your security stack is removed from, by suspending
//! this Tamer, 1 of your \[Aegiomon\] may digivolve into a Digimon card with
//! \[Aegiochusmon\] in its name in the hand without paying the cost.
//!
//! Inherited **[Security]**: Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_084.cs
//!
//! DCGO `SetUpActivateClass(CanActivateCondition, ActivateCoroutine, -1, true, ...)`
//! for the security-removed clause: `maxCountPerTurn = -1` (NOT capped —
//! no `[Once Per Turn]` tag is printed; the natural once-per-turn ceiling
//! comes from the suspend cost itself, since an already-suspended Tamer
//! can't pay it again until it untaps), `isOptional = true` ("may digivolve").
//! `CanSelectPermanentCondition` filters to permanents whose top card equals
//! the name "Aegiomon"; `CanSelectCardCondition` filters hand cards whose
//! name contains "Aegiochusmon". `DigivolveIntoHandOrTrashCard(payCost: false,
//! ignoreDigivolutionRequirementFixedCost: -1, isHand: true)` — cost is
//! waived but digivolution requirements (level/color match) are NOT ignored.
//!
//! # Patterns this test covers
//! - `start_of_your_main_phase` + `condition: { memory_lte: 4 }` + `gain_memory: 1`
//!   (BT24-083/BT24-088 Start-of-Turn memory-gain shape, adapted to Start of
//!   Main Phase per DCGO `EffectTiming.OnStartMainPhase`, engine-side
//!   `EffectTiming::StartOfYourMainPhase`).
//! - `on_own_security_removed` + `active_when: { all_turns: true }` (AD1-017 /
//!   BT20-083 idiom) + `condition: { source_is_unsuspended: true }` +
//!   `activation_cost: { suspend_self: true }` (AD1-019 suspend-cost idiom).
//! - `select_own_permanent` filtered to `name_contains: "Aegiomon"`, then
//!   `select_hand` filtered to `name_contains: "Aegiochusmon"`, then
//!   `effect_initiated_digivolve` with `cost: free`, `ignore_requirements: false`
//!   (BT17-097 / BT24-089 idiom — digivolution requirements ARE validated).
//! - Inherited `on_security` + `play_from_security` (canonical Tamer idiom).
//!
//! # Gap status
//! No new DSL vocabulary or engine primitives required — every verb used here
//! (`start_of_your_main_phase`, `on_own_security_removed`, `activation_cost:
//! suspend_self`, `select_own_permanent`, `select_hand`,
//! `effect_initiated_digivolve`, `play_from_security`) is already shipped and
//! exercised by sibling cards (BT24-083, BT24-088, AD1-017, AD1-019, BT17-097,
//! BT24-089, BT20-083).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCostDelta, CompiledPlayerRef, CompiledPredicate, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt24/BT24-084.yaml");

// ─── Fixtures ──────────────────────────────────────────────────────────────

/// A generic filler card (used for deck padding).
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A level-4 Yellow [Aegiomon]-named Digimon on the field (the digivolve
/// base). Mirrors the real BT24-034 Aegiomon: Lv.4, Yellow.
fn make_aegiomon(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Aegiomon", 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A Digimon that is NOT named Aegiomon — must be excluded from the
/// permanent selection. Named to avoid accidentally containing the
/// "Aegiomon" substring.
fn make_non_aegiomon(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Some Other Digimon", 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c
}

/// An [Aegiochusmon]-in-name Digimon hand card with a VALID digivolve
/// requirement from level-4 Yellow (matches `make_aegiomon`'s identity) —
/// mirrors BT24-014/P-213 Aegiochusmon's real alt-path
/// ("[Digivolve] [Aegiomon]: Cost 3" → level 4, Yellow evo_costs entry).
fn make_aegiochusmon_evo(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Aegiochusmon: Valid", 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(8000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Yellow, CardColor::Purple];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 3,
    }];
    c
}

/// An [Aegiochusmon]-in-name Digimon hand card whose evo_costs do NOT match
/// the Aegiomon base (wrong level) — digivolution requirements must reject
/// this pick when `ignore_requirements: false`.
fn make_aegiochusmon_invalid_evo(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Aegiochusmon: Invalid", 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(8000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 6, // mismatched — Aegiomon base is level 4
        memory_cost: 3,
    }];
    c
}

/// A hand card that is a Digimon but does NOT have "Aegiochusmon" in its
/// name — must be excluded from the hand-card selection regardless of its
/// evo_costs.
fn make_non_aegiochusmon(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Some Other Digimon", 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(8000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 3,
    }];
    c
}

/// Recursively search a CompiledPredicate tree for a name-matching node.
fn predicate_contains_name(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_is.as_deref() == Some(needle)
        || predicate.name_contains.as_deref() == Some(needle)
        || predicate
            .name_in
            .iter()
            .flatten()
            .any(|name| name == needle)
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
}

fn hand_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .any(|source| source.card_id(&runner.game.card_data) == card_id)
}

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .any(|permanent| {
            permanent
                .card_sources
                .iter()
                .any(|source| source.card_id(&runner.game.card_data) == card_id)
        })
}

// ═══════════════════════════════════════════════════════════════════════════
// §1 — Structural: YAML parses, clause shapes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_084_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-084 YAML must parse and compile without errors");
}

#[test]
fn bt24_084_is_tamer_cost_3_yellow_ts() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(compiled.cost, Some(3));
}

#[test]
fn bt24_084_has_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (start_of_your_main_phase, on_own_security_removed, \
         on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0: [Start of Your Main Phase] gate is `memory_lte: 4`, body gains 1.
#[test]
fn bt24_084_clause0_start_of_main_gate_and_gain() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    let clause0 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("must have a start_of_your_main_phase clause");

    let condition = clause0
        .condition
        .as_ref()
        .expect("clause must be conditioned on memory <= 4");
    assert_eq!(
        condition.memory_lte,
        Some(digimon_dsl::compiled::CompiledDpConstraint::Literal(4)),
        "gate must be 'if you have 4 or less memory'"
    );

    assert!(
        clause0
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemory(n) if *n == 1)),
        "clause body must gain 1 memory"
    );
}

/// Clause 1: [All Turns] own-security-removed, suspend-cost, name-filtered
/// digivolve.
#[test]
fn bt24_084_clause1_is_own_security_removed_all_turns() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("must have an on_own_security_removed clause");

    assert!(
        clause1.optional,
        "'may digivolve' — the effect must be declinable"
    );

    let active_when = clause1
        .active_when
        .as_ref()
        .expect("[All Turns] tag must be encoded");
    assert_eq!(
        active_when.all_turns,
        Some(true),
        "printed [All Turns] scope must be explicit"
    );
}

/// Clause 1 requires the Tamer be unsuspended (to pay the suspend cost).
/// The `activation_cost: { suspend_self: true }` step is lifted out of
/// `process` at compile time (bound onto `EffectBuilder::activation_cost`
/// instead — see `compiled.rs`'s `ActivationCost` variant doc comment), so
/// the suspend-cost itself is verified behaviorally in §3 below, not
/// structurally here.
#[test]
fn bt24_084_clause1_requires_unsuspended_source() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("must have an on_own_security_removed clause");

    let condition = clause1
        .condition
        .as_ref()
        .expect("clause must gate on the Tamer being unsuspended");
    assert_eq!(
        condition.source_is_unsuspended,
        Some(true),
        "the suspend cost can only be paid while unsuspended"
    );
}

/// Clause 1's permanent selection is filtered to `name_contains: "Aegiomon"`
/// and its hand-card selection is filtered to `name_contains: "Aegiochusmon"`,
/// with `effect_initiated_digivolve` at `cost: free` and
/// `ignore_requirements: false` (requirements ARE validated).
#[test]
fn bt24_084_clause1_selects_aegiomon_then_aegiochusmon_free_digivolve() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("must have an on_own_security_removed clause");

    let (perm_filter, perm_optional) = clause1
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, *optional)),
            _ => None,
        })
        .expect("clause must select an own permanent");
    assert!(
        predicate_contains_name(perm_filter, "Aegiomon"),
        "permanent selection must be filtered to [Aegiomon]"
    );
    assert!(
        perm_optional,
        "'1 of your [Aegiomon] MAY digivolve' — permanent pick must be declinable"
    );

    let hand_filter = clause1
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("clause must select a hand card");
    assert!(
        predicate_contains_name(hand_filter, "Aegiochusmon"),
        "hand-card selection must be filtered to [Aegiochusmon] in name"
    );

    assert!(
        clause1.process.iter().any(|step| matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Free,
                ignore_requirements: false,
                ..
            }
        )),
        "digivolve must waive the cost but still validate digivolution requirements"
    );
}

/// Clause 2: inherited [Security] play-self-free.
#[test]
fn bt24_084_clause2_is_on_security_play_from_security() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-084")
        .expect("BT24-084 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert!(
        !security_clause.optional,
        "the security trigger always activates (mandatory per rules §16)"
    );
    assert!(
        security_clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayFromSecurity)),
        "[Security] Play this card without paying the cost"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 — Clause 0 behavioral: [Start of Your Main Phase] memory gain
// ═══════════════════════════════════════════════════════════════════════════

/// At 4 memory (<= 4), the Start-of-Main trigger gains 1 memory.
#[test]
fn bt24_084_start_of_main_gains_memory_at_4_or_less() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(PermanentHandle {
            player: 0,
            index: 0,
        }),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("clause resolves");

    assert_eq!(runner.memory(), 5, "memory must increase by 1 at 4 or less");
}

/// At 5 memory (> 4), the Start-of-Main trigger does NOT fire the gain.
#[test]
fn bt24_084_start_of_main_no_gain_above_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(PermanentHandle {
            player: 0,
            index: 0,
        }),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("clause resolves (no-op)");

    assert_eq!(
        runner.memory(),
        5,
        "memory must NOT increase when starting above the 4-memory threshold"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 — Clause 1 behavioral: own-security-removed suspend digivolve
// ═══════════════════════════════════════════════════════════════════════════

fn fire_own_security_removed(runner: &mut DebugRunner, affected_player: u8) {
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player,
            observer_player: affected_player,
            source_player: 1 - affected_player,
            card: digimon_engine::card_source::CardHandle(0),
            cause: digimon_engine::trigger_context::EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

/// Clause 1 pairs `optional: true` with an `activation_cost` (suspend), so
/// the drainer installs a pre-cost `TriggerOrder` accept/decline confirm
/// BEFORE the suspend cost runs (see `effect_queue.rs`'s
/// `needs_pre_cost_prompt` — "Single trigger with activation_cost_fn +
/// optional: true → expose a TriggerOrder selection... before running the
/// cost closure"). Accepting proceeds into the suspend cost and then the
/// permanent selection; this is a real, distinct step from the later
/// `select_own_permanent` PASS.
fn accept_trigger_order_confirm(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("pre-cost TriggerOrder confirm must be pending");
    assert_eq!(
        view.kind,
        SelectionKind::TriggerOrder,
        "an optional trigger with an activation cost must pre-confirm before paying it"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the pre-cost trigger confirm");
}

/// When security is removed and an unsuspended Inori Misono + a battle-area
/// Aegiomon + a valid Aegiochusmon hand card are present, the trigger offers
/// the optional permanent selection (OwnField, only the Aegiomon eligible).
#[test]
fn bt24_084_security_removed_offers_aegiomon_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_non_aegiomon("NOTAEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));
    let aegiomon = runner.place_on_field(0, "AEGIOMON", Some(0));
    let non_aegiomon = runner.place_on_field(0, "NOTAEGIOMON", Some(0));

    fire_own_security_removed(&mut runner, 0);
    accept_trigger_order_confirm(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("own-security removal must offer the optional suspend-and-digivolve");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "'may digivolve' must be declinable");

    let want_aegiomon = digimon_engine::action::space::encode_attack(0, aegiomon.index as u16);
    let want_non_aegiomon =
        digimon_engine::action::space::encode_attack(0, non_aegiomon.index as u16);
    assert!(
        view.valid_action_ids.contains(&want_aegiomon),
        "the [Aegiomon] permanent must be selectable"
    );
    assert!(
        !view.valid_action_ids.contains(&want_non_aegiomon),
        "a non-[Aegiomon] permanent must NOT be selectable"
    );
}

/// Full path: selecting the Aegiomon, then the valid Aegiochusmon hand card,
/// suspends Inori Misono, digivolves Aegiomon into Aegiochusmon for free
/// (memory unchanged), and the hand card leaves hand for the stack.
#[test]
fn bt24_084_full_digivolve_path_suspends_and_digivolves_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-084", Some(0));
    let aegiomon = runner.place_on_field(0, "AEGIOMON", Some(0));

    let memory_before = runner.memory();
    fire_own_security_removed(&mut runner, 0);
    accept_trigger_order_confirm(&mut runner);

    // Select the Aegiomon permanent.
    let view = runner
        .pending_selection_view()
        .expect("permanent selection pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Aegiomon");

    // Select the Aegiochusmon hand card.
    let view = runner
        .pending_selection_view()
        .expect("hand-card selection pending after choosing the permanent");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Aegiochusmon hand card");

    runner.auto_resolve().expect("digivolve resolves");

    assert!(
        !hand_contains(&runner, 0, "EVO"),
        "the digivolved-in card must leave hand"
    );
    assert!(
        battle_area_contains(&runner, 0, "EVO"),
        "the Aegiochusmon card must join the field stack"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the digivolve must be free — no memory paid"
    );
    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Inori Misono must be suspended to pay the activation cost"
    );
    let _ = aegiomon;
}

/// Declining the pre-cost TriggerOrder confirm ("may digivolve") entirely
/// skips the effect: the suspend cost is never paid (Tamer stays
/// unsuspended) and the hand card remains untouched.
#[test]
fn bt24_084_security_removed_outer_confirm_declinable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));

    fire_own_security_removed(&mut runner, 0);

    let view = runner
        .pending_selection_view()
        .expect("pre-cost TriggerOrder confirm pending");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "the outer confirm must expose PASS");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the outer confirm");
    runner
        .auto_resolve()
        .expect("clause resolves after decline");

    assert!(
        hand_contains(&runner, 0, "EVO"),
        "declining must leave the Aegiochusmon card in hand"
    );
    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "declining the outer confirm must never pay the suspend cost"
    );
}

/// After ACCEPTING the outer confirm (suspend cost paid), the Aegiomon
/// permanent pick itself is still declinable ('1 of your [Aegiomon] MAY
/// digivolve') — declining at that point leaves the hand untouched, but the
/// already-paid suspend cost is NOT refunded.
#[test]
fn bt24_084_security_removed_permanent_pick_declinable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));

    fire_own_security_removed(&mut runner, 0);
    accept_trigger_order_confirm(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("permanent selection pending");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "permanent pick must expose PASS");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the permanent pick");
    runner
        .auto_resolve()
        .expect("clause resolves after decline");

    assert!(
        hand_contains(&runner, 0, "EVO"),
        "declining must leave the Aegiochusmon card in hand"
    );
    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "the suspend cost was already paid before this later decline"
    );
}

/// A hand card that is a Digimon but does NOT carry [Aegiochusmon] in its
/// name must never appear in the hand-card selection, even though it has a
/// matching evo_costs entry.
#[test]
fn bt24_084_hand_selection_excludes_non_aegiochusmon_named_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_non_aegiochusmon("OTHER"))
        .add_card(make_filler("FILL"))
        .hand(0, &["OTHER"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));

    fire_own_security_removed(&mut runner, 0);
    accept_trigger_order_confirm(&mut runner);

    // Select the Aegiomon permanent.
    let view = runner
        .pending_selection_view()
        .expect("permanent selection pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Aegiomon");

    // No hand-card selection should be offered (no eligible hand target) —
    // the clause resolves as a no-op instead of panicking.
    runner
        .auto_resolve()
        .expect("clause resolves with no eligible hand target");
    assert!(
        hand_contains(&runner, 0, "OTHER"),
        "the non-[Aegiochusmon]-named card must remain untouched in hand"
    );
}

/// [No-Approximations §17] Digivolution requirements are validated
/// (`ignore_requirements: false`): an [Aegiochusmon]-named hand card whose
/// `evo_costs` do NOT match the selected Aegiomon's level must be REJECTED
/// by the engine — the digivolve does not complete and the card stays in hand.
#[test]
fn bt24_084_digivolve_requirements_are_validated_not_ignored() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_invalid_evo("BADEVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BADEVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));

    fire_own_security_removed(&mut runner, 0);
    accept_trigger_order_confirm(&mut runner);

    // Select the Aegiomon permanent.
    let view = runner
        .pending_selection_view()
        .expect("permanent selection pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Aegiomon");

    // The mismatched-evo_costs card is still offered by name filter (the
    // hand-card `select_hand` filter only checks the name, per DCGO
    // `CanSelectCardCondition`) but the subsequent `effect_initiated_digivolve`
    // with `ignore_requirements: false` must reject the actual digivolve.
    if let Some(view) = runner.pending_selection_view() {
        if view.kind == SelectionKind::Hand {
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("choose the mismatched-evo Aegiochusmon card");
        }
    }
    runner
        .auto_resolve()
        .expect("clause resolves without panic even when requirements reject the digivolve");

    assert!(
        hand_contains(&runner, 0, "BADEVO"),
        "a digivolve rejected by unmet requirements must leave the card in hand"
    );
    assert!(
        !battle_area_contains(&runner, 0, "BADEVO"),
        "the rejected card must NOT join the field stack"
    );
}

/// [All Turns]: the trigger fires identically on the OPPONENT's turn (not
/// restricted to the controller's own turn).
#[test]
fn bt24_084_security_removed_fires_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));

    // Advance to the opponent's turn.
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);

    fire_own_security_removed(&mut runner, 0);

    let view = runner.pending_selection_view();
    assert!(
        view.is_some(),
        "[All Turns] must fire the own-security-removed trigger even on the \
         opponent's turn"
    );
}

/// The suspend cost cannot be paid twice: if Inori Misono is ALREADY
/// suspended when security is removed, the trigger does not offer the
/// permanent selection (DCGO `source_is_unsuspended` gate).
#[test]
fn bt24_084_security_removed_no_trigger_when_already_suspended() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));
    runner.game.suspend(tamer);

    fire_own_security_removed(&mut runner, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "an already-suspended Tamer cannot pay the suspend cost again — \
         the trigger must not open a selection"
    );
}

/// The trigger must NOT fire off the OPPONENT's security removal (this is
/// "your security stack", i.e. the controller's own).
#[test]
fn bt24_084_does_not_fire_on_opponent_security_removed() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_aegiomon("AEGIOMON"))
        .add_card(make_aegiochusmon_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT24-084", Some(0));
    runner.place_on_field(0, "AEGIOMON", Some(0));

    // Player 1's security is removed, not player 0's — Inori Misono (owned by
    // player 0) must not react.
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 1,
            source_player: 0,
            card: digimon_engine::card_source::CardHandle(0),
            cause: digimon_engine::trigger_context::EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "BT24-084 belongs to player 0 and must not react to player 1's \
         security being removed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 — Clause 2 behavioral: [Security] play self free
// ═══════════════════════════════════════════════════════════════════════════

/// Attacking into a security stack containing BT24-084 checks it and, per
/// its inherited [Security] effect, plays it into the battle area without
/// paying its cost (canonical `play_from_security` idiom — BT21-015 shape).
#[test]
fn bt24_084_security_effect_plays_self_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .security(1, &["BT24-084"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let memory_before = runner.memory();
    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security check resolves");

    assert_eq!(
        runner.security_count(1),
        0,
        "BT24-084 must be checked and removed from the security stack"
    );
    assert!(
        battle_area_contains(&runner, 1, "BT24-084"),
        "[Security] Play this card without paying the cost must place it \
         in the battle area"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the security play must not charge its play cost"
    );
}
