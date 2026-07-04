//! BT18-073 Machinedramon — Digimon, Lv.6, Black/Purple, DP 11000, Cost 11.
//! Traits: Machine, (Rule) Composite. Attribute: Virus.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT18-073.md;
//! card image confirmed)
//!
//! When this card would be played, by deleting 1 of your Digimon with the
//! [Composite] trait, reduce the play cost by 4.
//! [On Play] [When Digivolving] ＜De-Digivolve 1＞ all of your opponent's
//! Digimon.
//! [On Deletion] 1 of your [Kimeramon] and 1 [Machinedramon] in the trash may
//! DNA digivolve into [Millenniummon] in the hand.
//! (Rule) Trait: Has [Composite] Type.
//!
//! Inherited Effect: [Opponent's Turn] [Once Per Turn] When any of your
//! opponent's Digimon attack, you may change the attack target to 1 of your
//! Digimon with the [Composite] or [Wicked God] trait.
//!
//! Digivolution: special condition (xros_req) "[Digivolve] Lv.5
//! w/[Composite] trait: Cost 3".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Black/BT18_073.cs
//!
//! # Role resolution (DCGO disambiguates the card-face wording)
//!
//! DCGO's `HasKimeramon` scans the OWNER's BATTLE AREA
//! (`IsPermanentExistsOnOwnerBattleAreaDigimon`) for a Digimon named
//! "Kimeramon"; `HasMachinedramon` scans the OWNER's TRASH (`IsExistOnTrash`)
//! for a card named "Machinedramon" (no self-exclusion — the just-deleted
//! Machinedramon itself, already moved to trash per the batched deletion
//! flow, CLAUDE.md rule 25, is itself a legal material). So: Kimeramon =
//! FIELD partner, Machinedramon = TRASH partner. This is the role-swapped
//! mirror of BT18-015 Kimeramon's own [On Deletion] clause. Recipe target:
//! BT18-019 Millenniummon (`dna_costs`: requirement1 name_contains
//! "Kimeramon", requirement2 name_contains "Machinedramon", memory_cost 0).
//!
//! # Patterns this test file covers
//! - Structural: 1 cost_reduction declarative clause, 2 triggered clauses
//!   (on_play+when_digivolving shared, on_deletion), 1 inherited triggered
//!   clause (redirect), 1 digivolve alt-path, traits include [Composite].
//! - BeforePayCost cost reduction: STATIC amount:4 (not
//!   delete_for_cost_reduction's variable credit) + mandatory
//!   `select_own_permanent{Composite}` pay_cost with automatic
//!   `pay_cost_guard` (no offer without an eligible Composite Digimon) and
//!   outer accept/decline gate.
//! - on_play/when_digivolving: `for_each` De-Digivolve-1 over ALL opponent
//!   battle-area Digimon (BT9-112 idiom), gated on >=1 opponent Digimon
//!   existing.
//! - on_deletion clause: `effect_initiated_dna_digivolve_trash_partner`
//!   (G-DSL-DNA-TRASH-PARTNER) with Kimeramon=field, Machinedramon=trash
//!   (role-swap of BT18-015). Condition gating on all 3 candidate pools;
//!   outer "may"; order of picks (hand result -> field partner -> trash
//!   partner).
//! - Inherited [Opponent's Turn][OPT] redirect attack to Composite/Wicked
//!   God Digimon (`scope: inherited`, BT19-065 corrected-pattern
//!   precedent, shared OPT hash "SwitchTarget_BT18-073").
//!
//! # Known gaps
//! (none)

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{AttackTarget, SelectionKind, TriggerSource};

const CARD_ID: &str = "BT18-073";
const BT18_073_YAML: &str = include_str!("../../../cards/bt18/BT18-073.yaml");

// ─── Runner builders ──────────────────────────────────────────────────────────

/// BT18-073 loaded from inline YAML (this card is brand-new — using
/// `from_dsl_yaml` avoids depending on the embedded pack having been rebuilt
/// since this file was authored) plus its real DSL siblings: BT18-019
/// Millenniummon (the DNA result) and BT18-015 Kimeramon (the field
/// material). Per §11 anti-patterns: prefer real shipped cards over
/// synthetic fixtures when they exist.
fn machinedramon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(BT18_073_YAML)
        .expect("BT18-073 YAML must parse")
        .dsl_card("BT18-019")
        .expect("BT18-019 Millenniummon must be in the embedded DSL pack")
        .dsl_card("BT18-015")
        .expect("BT18-015 Kimeramon must be in the embedded DSL pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_composite_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_digimon(id, level, dp);
    card.traits = vec!["Composite".to_string()];
    card
}

fn make_wicked_god_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_digimon(id, level, dp);
    card.traits = vec!["Wicked God".to_string()];
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, next));
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

fn has_named_on_field(runner: &DebugRunner, player: u8, name: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_name(&runner.game.card_data) == name)
}

/// Drive every installed selection by picking the first non-PASS action,
/// falling back to PASS if that's the only legal action. Used for tests that
/// only care about the eventual post-state, not which branch is exercised.
fn resolve_all_picking_first_non_pass(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let (player, action) = {
            let sel = runner.game.pending_selection.as_ref().unwrap();
            let action = sel
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(sel.valid_action_ids[0]);
            (sel.selecting_player, action)
        };
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_073_yaml_parses_without_error() {
    let _runner = machinedramon_runner();
}

#[test]
fn bt18_073_has_printed_metadata() {
    let runner = machinedramon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT18-073 must be in compiled_cards");

    assert_eq!(card.name, "Machinedramon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    for trait_name in ["Machine", "Composite"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "BT18-073 metadata must include trait {trait_name} (Composite is Rule-granted)"
        );
    }
}

#[test]
fn bt18_073_declares_one_digivolve_alt_path_lv5_composite() {
    let runner = machinedramon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT18-073 must be in compiled_cards");

    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();

    assert_eq!(
        digivolve_paths.len(),
        1,
        "BT18-073 must declare exactly 1 digivolve alt-path (Lv.5 w/[Composite] trait, Cost 3)"
    );
    assert_eq!(digivolve_paths[0].cost, Some(CompiledCost::Literal(3)));
}

#[test]
fn bt18_073_has_cost_reduction_clause_with_static_amount_four() {
    let runner = machinedramon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT18-073 must be in compiled_cards");

    let found = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        found,
        "BT18-073 must declare a cost_reduction declarative clause"
    );
}

#[test]
fn bt18_073_has_shared_on_play_when_digivolving_clause_and_on_deletion_clause() {
    let runner = machinedramon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT18-073 must be in compiled_cards");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let on_play_wd = triggered
        .iter()
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("shared On Play/When Digivolving clause must exist");
    assert!(
        !on_play_wd.once_per_turn,
        "the De-Digivolve-all clause has no [OPT] tag in the printed text"
    );
    assert_eq!(on_play_wd.scope, CompiledScope::FaceUp);

    let on_deletion = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnDeletion))
        .expect("on_deletion clause must exist");
    assert!(
        on_deletion.optional,
        "BT18-073's [On Deletion] DNA rider is 'may' — the outer gate must be optional"
    );
    assert_eq!(on_deletion.scope, CompiledScope::FaceUp);
}

#[test]
fn bt18_073_has_inherited_redirect_clause() {
    let runner = machinedramon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT18-073 must be in compiled_cards");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnOpponentAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT18-073 must have an [Opponent's Turn] redirect clause");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "the redirect clause is printed in the Inherited Effect box, so it must use scope: inherited"
    );
    assert!(clause.optional, "you MAY change the attack target");
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] must be enforced structurally"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RedirectAttackTarget { .. })),
        "inherited clause must redirect the attack target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — BeforePayCost cost reduction: condition gating
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: with no [Composite] Digimon in play, the reducer must NOT be
/// offered at all (DCGO `CanActivateCondition = HasMatchConditionPermanent`)
/// — the card plays at full cost 11.
///
/// NOTE: asserts via the `MemoryChange` event log rather than a raw
/// before/after gauge diff — paying 11 from a clamped starting memory of 10
/// crosses zero, and any `resolve_selection` call in between would trigger
/// `Game::check_turn_end`'s turn-flip (see the sibling declining-reducer
/// test's note). This test never installs a selection, so raw comparison
/// happens to be safe here too, but the event-log form is used for
/// consistency and to stay correct if scaffolding changes.
#[test]
fn bt18_073_no_composite_no_reducer_offered_full_cost_paid() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("FILLER", 3, 3000))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &[CARD_ID])
        .memory(20)
        .start();
    // No Composite Digimon anywhere on the controller's field.
    runner.place_on_field(0, "FILLER", None);

    let events_before = runner.game.events.len();
    let _ = runner.play(0, 0);
    // No Composite Digimon -> reducer not offered; the play completes at
    // full cost (any subsequent selections belong to the On Play De-Digivolve
    // clause, which has no eligible opponent Digimon here, so nothing
    // installs and auto_resolve is a no-op).
    let _ = runner.auto_resolve();

    let paid: i16 = runner.game.events[events_before..]
        .iter()
        .filter_map(|e| match e {
            digimon_engine::events::GameEvent::MemoryChange { delta, .. } => Some(*delta),
            _ => None,
        })
        .sum();
    assert_eq!(
        paid, -11,
        "no [Composite] Digimon in play -> reducer not offered, full cost 11 paid"
    );
}

/// Positive: with a [Composite] Digimon in play, the reducer offers the
/// delete-as-cost prompt (declinable outer gate).
#[test]
fn bt18_073_composite_present_installs_cost_reduction_prompt() {
    let mut runner = machinedramon_runner()
        .add_card(make_composite_digimon("COMP-OWN", 4, 5000))
        .deck(0, &["COMP-OWN", "COMP-OWN", "COMP-OWN"])
        .deck(1, &["COMP-OWN"])
        .hand(0, &[CARD_ID])
        .memory(20)
        .start();
    runner.place_on_field(0, "COMP-OWN", None);

    let _ = runner.play(0, 0);

    let sel = runner
        .pending_selection()
        .expect("cost-reduction prompt must install when a Composite Digimon is in play");
    assert!(
        sel.is_optional,
        "the delete-as-cost pick is behind an accept/decline gate (You may decline)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — BeforePayCost cost reduction: behavioral outcome
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: accepting the reducer deletes the chosen Composite Digimon and
/// reduces the play cost by a FLAT 4 (11 -> 7), regardless of the deleted
/// Digimon's own printed cost.
#[test]
fn bt18_073_accepting_reducer_deletes_composite_and_reduces_cost_by_flat_four() {
    let mut runner = machinedramon_runner()
        .add_card(make_composite_digimon("COMP-OWN", 4, 5000))
        .deck(0, &["COMP-OWN", "COMP-OWN", "COMP-OWN"])
        .deck(1, &["COMP-OWN"])
        .hand(0, &[CARD_ID])
        .memory(20)
        .start();
    runner.place_on_field(0, "COMP-OWN", None);

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);

    let sel = runner
        .pending_selection()
        .expect("cost-reduction prompt must install");
    let action = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("Composite Digimon must be a selectable cost target");
    let player = sel.selecting_player;
    runner.game.resolve_selection(player, action).ok();
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 7,
        "Machinedramon cost 11 reduced by a FLAT 4 (not the deleted Digimon's own cost) -> 7; got {cost_paid}"
    );
    assert!(
        !has_named_on_field(&runner, 0, "COMP-OWN"),
        "the Composite Digimon must have been deleted as the cost"
    );
}

/// Negative: declining the reducer plays the card at full cost 11 and the
/// Composite Digimon survives.
///
/// NOTE: cost 11 paid from a clamped starting memory of 10 (`rules.
/// memory_range` caps at +/-10 regardless of the builder's `.memory(20)`)
/// necessarily crosses zero, which triggers `Game::check_turn_end` ->
/// `end_turn()` on the `resolve_selection` call that declines the prompt --
/// the memory gauge's value AFTER the flip reflects the new turn player's
/// perspective, not a simple "before minus paid" arithmetic. So this test
/// asserts the actual payment via the `MemoryChange` event log (the
/// structural cost-payment entry, `source_card_id: None`) rather than a
/// naive `mem_before - mem_after` diff, which the turn-end sign flip would
/// corrupt.
#[test]
fn bt18_073_declining_reducer_pays_full_cost_and_composite_survives() {
    let mut runner = machinedramon_runner()
        .add_card(make_composite_digimon("COMP-OWN", 4, 5000))
        .deck(0, &["COMP-OWN", "COMP-OWN", "COMP-OWN"])
        .deck(1, &["COMP-OWN"])
        .hand(0, &[CARD_ID])
        .memory(20)
        .start();
    runner.place_on_field(0, "COMP-OWN", None);

    let events_before = runner.game.events.len();
    let _ = runner.play(0, 0);

    let sel = runner
        .pending_selection()
        .expect("cost-reduction prompt must install");
    let player = sel.selecting_player;
    runner.game.resolve_selection(player, PASS).ok();
    let _ = runner.auto_resolve();

    let paid: i16 = runner.game.events[events_before..]
        .iter()
        .filter_map(|e| match e {
            digimon_engine::events::GameEvent::MemoryChange { delta, .. } => Some(*delta),
            _ => None,
        })
        .sum();
    assert_eq!(
        paid, -11,
        "declining the reducer must pay the full printed cost of 11 (event-log delta)"
    );
    assert!(
        has_named_on_field(&runner, 0, "COMP-OWN"),
        "declining must leave the Composite Digimon on the field"
    );
}

/// A Digimon with a HIGH printed cost, when deleted as the reducer's cost,
/// still only credits a flat 4 (proves the amount is NOT
/// `delete_for_cost_reduction`'s variable deleted-cost credit).
#[test]
fn bt18_073_reduction_is_flat_four_not_deleted_digimons_own_cost() {
    let mut runner = machinedramon_runner()
        .add_card({
            let mut c = make_composite_digimon("EXPENSIVE-COMP", 6, 9000);
            c.play_cost = 9;
            c
        })
        .deck(0, &["EXPENSIVE-COMP", "EXPENSIVE-COMP", "EXPENSIVE-COMP"])
        .deck(1, &["EXPENSIVE-COMP"])
        .hand(0, &[CARD_ID])
        .memory(20)
        .start();
    runner.place_on_field(0, "EXPENSIVE-COMP", None);

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);

    let sel = runner
        .pending_selection()
        .expect("cost-reduction prompt must install");
    let action = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("Composite Digimon selectable");
    let player = sel.selecting_player;
    runner.game.resolve_selection(player, action).ok();
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 7,
        "even deleting a 9-cost Composite Digimon only credits the flat -4 (11 - 4 = 7), \
         not the deleted Digimon's own printed cost; got {cost_paid}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — On Play / When Digivolving: De-Digivolve all opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: with no opponent Digimon on the board, the clause does not fire
/// (no selection installs, nothing to de-digivolve).
#[test]
fn bt18_073_on_play_no_opponent_digimon_installs_nothing() {
    let mut runner = machinedramon_runner().memory(20).start();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    // No opponent Digimon anywhere.

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no selection should install when the opponent has no battle-area Digimon"
    );
}

/// Positive: On Play de-digivolves EVERY opponent Digimon by 1 (no target
/// selection — the printed text is "all", not a pick).
#[test]
fn bt18_073_on_play_de_digivolves_all_opponent_digimon() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("BASE-OPP-A", 3, 3000))
        .add_card(make_digimon("BASE-OPP-B", 3, 3000))
        .add_card(make_digimon("TOP-OPP-A", 5, 6000))
        .add_card(make_digimon("TOP-OPP-B", 5, 6000))
        .memory(20)
        .start();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_stack(1, &["BASE-OPP-A", "TOP-OPP-A"]);
    let opp_b = runner.place_stack(1, &["BASE-OPP-B", "TOP-OPP-B"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    resolve_all_picking_first_non_pass(&mut runner);

    // Both opponent Digimon must have been de-digivolved (top card trashed,
    // base card now on top).
    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE-OPP-A",
        "opponent Digimon A must be de-digivolved to its base card"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE-OPP-B",
        "opponent Digimon B must be de-digivolved to its base card"
    );
}

/// De-digivolve stops at level 3 (does not trash past a level-3 card) — a
/// standalone level-3 opponent Digimon with no stack is unaffected in stack
/// shape (no source to trash), consistent with the printed reminder text.
#[test]
fn bt18_073_on_play_de_digivolve_does_not_affect_standalone_level_three_stack_shape() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("LV3-OPP", 3, 2000))
        .memory(20)
        .start();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "LV3-OPP", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    resolve_all_picking_first_non_pass(&mut runner);

    // A standalone level-3 Digimon has no digivolution source to trash — it
    // must remain on the field, still itself (De-Digivolve can't go past
    // level 3, and there is nothing beneath it to pop regardless).
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "LV3-OPP",
        "a standalone level-3 opponent Digimon survives de-digivolve unaffected"
    );
}

/// [When Digivolving] independently fires the same body — no [OPT] tag
/// shared with [On Play].
#[test]
fn bt18_073_when_digivolving_de_digivolves_all_opponent_digimon() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("BASE-MDRA", 5, 8000))
        .add_card(make_digimon("BASE-OPP", 3, 3000))
        .add_card(make_digimon("TOP-OPP", 5, 6000))
        .memory(20)
        .start();
    // Stack: BASE-MDRA (bottom) -> BT18-073 (top), simulating a digivolve.
    let carrier = runner.place_stack(0, &["BASE-MDRA", CARD_ID]);
    let opp = runner.place_stack(1, &["BASE-OPP", "TOP-OPP"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    resolve_all_picking_first_non_pass(&mut runner);

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE-OPP",
        "[When Digivolving] must also de-digivolve all opponent Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — On Deletion: condition gating (all 3 pools required)
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: Machinedramon deleted with NO Kimeramon on field -> the rider
/// must not fire (no selection installs).
#[test]
fn bt18_073_on_deletion_does_not_fire_without_kimeramon_on_field() {
    let mut runner = machinedramon_runner().memory(20).start();
    let machinedramon = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_hand(&mut runner, 0, "BT18-019"); // Millenniummon in hand
                                               // NO Kimeramon on field.

    runner.game.delete_permanent_with_cause(
        machinedramon,
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "on_deletion rider must NOT fire without a Kimeramon on the controller's field"
    );
}

/// Negative: Kimeramon on field, Machinedramon in trash (via self-deletion),
/// but NO Millenniummon in hand -> the rider must not fire.
#[test]
fn bt18_073_on_deletion_does_not_fire_without_millenniummon_in_hand() {
    let mut runner = machinedramon_runner().memory(20).start();
    let machinedramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BT18-015", None); // Kimeramon on field
                                                 // No Millenniummon in hand.

    runner.game.delete_permanent_with_cause(
        machinedramon,
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "on_deletion rider must NOT fire without a Millenniummon in the controller's hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — On Deletion: behavioral outcome (the DNA merge)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: Machinedramon dies with a Kimeramon on field and a Millenniummon
/// in hand. Per rule 25 (OnDeletion fires post-trash), the deleted
/// Machinedramon itself is now IN the trash and satisfies the "1
/// [Machinedramon] in the trash" material — no separate trash seed needed.
/// Driving the full flow (accept -> pick Millenniummon -> pick Kimeramon ->
/// pick the trashed Machinedramon) must merge Kimeramon + Machinedramon into
/// a single permanent topped by Millenniummon.
#[test]
fn bt18_073_on_deletion_dna_digivolves_into_millenniummon_using_self_as_trash_material() {
    let mut runner = machinedramon_runner().memory(20).start();
    let machinedramon = runner.place_on_field(0, CARD_ID, Some(0));
    let machinedramon_card = runner.top_card(machinedramon);
    runner.place_on_field(0, "BT18-015", None); // Kimeramon on field
    push_to_hand(&mut runner, 0, "BT18-019"); // Millenniummon in hand

    runner.game.delete_permanent_with_cause(
        machinedramon,
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    // Machinedramon must now be in the trash (post-trash OnDeletion timing).
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == machinedramon_card),
        "precondition: the deleted Machinedramon must be in the trash when on_deletion fires"
    );

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("on_deletion outer accept/decline prompt must install");
    assert!(
        sel.is_optional,
        "the [On Deletion] rider is 'may' — PASS must be legal"
    );

    resolve_all_picking_first_non_pass(&mut runner);

    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-019")
        .expect("merged Millenniummon permanent must exist after DNA digivolve");

    assert!(
        merged
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == "BT18-015"),
        "Kimeramon must be a digivolution source under the merged Millenniummon"
    );
    assert!(
        merged
            .card_sources
            .iter()
            .any(|s| s.handle() == machinedramon_card),
        "the deleted Machinedramon (now in trash) must be the trash material consumed into the merge"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "the Millenniummon result must leave the hand"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == machinedramon_card),
        "the trash Machinedramon material must be consumed out of the trash by the merge"
    );
}

/// Positive, alternate material: with TWO Machinedramon-named cards
/// available (the just-deleted one AND a separately-seeded trash copy), the
/// trash pick prompt must expose a real choice (both are legal trash
/// materials).
#[test]
fn bt18_073_on_deletion_offers_trash_pick_among_multiple_machinedramon_copies() {
    let mut runner = machinedramon_runner().memory(20).start();
    let machinedramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BT18-015", None);
    push_to_hand(&mut runner, 0, "BT18-019");
    push_to_trash(&mut runner, 0, CARD_ID); // a second Machinedramon-named copy already in trash

    runner.game.delete_permanent_with_cause(
        machinedramon,
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    // Accept the outer "may".
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("outer accept prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("accept action available");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
    }
    // Pick the Millenniummon result (only one in hand).
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("hand result prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("Millenniummon selectable");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
    }
    // Pick the Kimeramon field partner (only one on field).
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("field partner prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("Kimeramon selectable");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
    }

    // Now the trash-partner prompt must expose 2 distinct Machinedramon-named
    // choices (the just-deleted one + the seeded copy).
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("trash partner prompt must install");
    let non_pass_count = sel.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(
        non_pass_count, 2,
        "both Machinedramon-named trash cards must be selectable as the DNA material; got {non_pass_count}"
    );
}

/// Declining the outer [On Deletion] "may" leaves the board untouched (no
/// merge; Machinedramon stays in trash, Kimeramon stays on field,
/// Millenniummon stays in hand).
#[test]
fn bt18_073_on_deletion_declining_leaves_board_untouched() {
    let mut runner = machinedramon_runner().memory(20).start();
    let machinedramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BT18-015", None);
    push_to_hand(&mut runner, 0, "BT18-019");

    runner.game.delete_permanent_with_cause(
        machinedramon,
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("outer accept prompt");
    let player = sel.selecting_player;
    runner.game.resolve_selection(player, PASS).ok();
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "declining the outer 'may' must end the clause with no further prompts"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "Millenniummon must remain in hand when the rider is declined"
    );
    assert!(
        has_named_on_field(&runner, 0, "Kimeramon"),
        "Kimeramon must remain on the field when the rider is declined"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-019"),
        "no Millenniummon permanent must appear on the field when declined"
    );
}

/// Cost firing (event log): the on_deletion rider's DNA merge pays the
/// result's printed DNA cost (0 for BT18-019) — memory must be unchanged by
/// the merge.
#[test]
fn bt18_073_on_deletion_merge_pays_printed_dna_cost_of_zero() {
    let mut runner = machinedramon_runner().memory(20).start();
    let machinedramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BT18-015", None);
    push_to_hand(&mut runner, 0, "BT18-019");

    let memory_before = runner.memory();

    runner.game.delete_permanent_with_cause(
        machinedramon,
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();
    resolve_all_picking_first_non_pass(&mut runner);

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-019"),
        "merge must have completed"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "BT18-019's printed DNA cost is 0 -- memory must be unchanged by the merge"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 7 — Inherited redirect: condition gating + behavior
// ═══════════════════════════════════════════════════════════════════════════
//
// The redirect clause is printed in Machinedramon's INHERITED effect text
// box (`scope: inherited`), so it is only active while BT18-073 is buried as
// a digivolution SOURCE beneath another permanent (RULES 15-3-1; engine
// `Game::activation_counts_for_permanent`'s `is_under != effect.inherited`
// gate). Every positive test below therefore buries BT18-073 via
// `place_stack(player, &["BT18-073", "<HOST>"])` and fires the attack
// against the resulting stack handle, mirroring the proven BT19-065 pattern.

#[test]
fn bt18_073_inherited_redirect_installs_when_buried_and_composite_target_exists() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("HOST", 6, 10000))
        .add_card(make_digimon("ATK", 6, 12000))
        .add_card(make_composite_digimon("COMP-TARGET", 5, 3000))
        .add_card(make_digimon("SEC", 5, 2000))
        .security(0, &["SEC"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    // Stack: BT18-073 (buried source) -> HOST (top card).
    let _stack = runner.place_stack(0, &[CARD_ID, "HOST"]);
    let composite_target = runner.place_on_field(0, "COMP-TARGET", Some(0));
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt (buried Machinedramon's inherited clause fires)");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("buried Machinedramon should prompt for a Composite/Wicked God retarget");
    assert_eq!(pending.kind, SelectionKind::OwnField);

    let choose_composite = encode_attack(0, composite_target.index as u16);
    assert!(
        pending.valid_action_ids.contains(&choose_composite),
        "the Composite-trait Digimon should be a legal retarget"
    );
    runner
        .game
        .resolve_selection(0, choose_composite)
        .expect("resolve the Composite retarget");

    assert_eq!(
        runner.game.players[0].security.len(),
        security_before,
        "redirected attack must not check security"
    );
}

/// A [Wicked God] trait Digimon is also a legal retarget (the printed text
/// covers both [Composite] and [Wicked God]).
#[test]
fn bt18_073_inherited_redirect_accepts_wicked_god_target() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("HOST", 6, 10000))
        .add_card(make_digimon("ATK", 6, 12000))
        .add_card(make_wicked_god_digimon("WG-TARGET", 5, 3000))
        .add_card(make_digimon("SEC", 5, 2000))
        .security(0, &["SEC"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let _stack = runner.place_stack(0, &[CARD_ID, "HOST"]);
    let wg_target = runner.place_on_field(0, "WG-TARGET", Some(0));
    let attacker = runner.place_on_field(1, "ATK", Some(0));

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("redirect prompt must install");
    let choose_wg = encode_attack(0, wg_target.index as u16);
    assert!(
        pending.valid_action_ids.contains(&choose_wg),
        "the Wicked-God-trait Digimon should be a legal retarget"
    );
}

/// Negative control: with BT18-073 standing ALONE as its own top card (not
/// buried under a host), its inherited clause must NOT fire.
#[test]
fn bt18_073_inherited_redirect_does_not_fire_when_standalone_top_card() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("ATK", 6, 12000))
        .add_card(make_digimon("SEC", 5, 2000))
        .security(0, &["SEC"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    // BT18-073 stands alone as its own top card -- NOT buried under a host.
    runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert!(
        runner.pending_selection().is_none(),
        "the inherited redirect clause must NOT fire while Machinedramon is its own standalone top card"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        security_before - 1,
        "with no redirect installed, the attack proceeds and resolves security against the original defender"
    );
}

/// Negative: with no [Composite]/[Wicked God] Digimon anywhere on the
/// controller's own field, the redirect clause does not fire (condition
/// gate), even while buried.
#[test]
fn bt18_073_inherited_redirect_does_not_fire_without_eligible_target() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("HOST", 6, 10000))
        .add_card(make_digimon("ATK", 6, 12000))
        .add_card(make_digimon("PLAIN", 5, 3000))
        .add_card(make_digimon("SEC", 5, 2000))
        .security(0, &["SEC"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let _stack = runner.place_stack(0, &[CARD_ID, "HOST"]);
    runner.place_on_field(0, "PLAIN", Some(0)); // no Composite/Wicked God trait
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert!(
        runner.pending_selection().is_none(),
        "no [Composite]/[Wicked God] Digimon on own field -> the redirect clause must not fire"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        security_before - 1,
        "the attack proceeds unimpeded to security"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 8 — Inherited redirect: OPT lockout
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_073_inherited_redirect_opt_blocks_second_activation_same_turn() {
    let mut runner = machinedramon_runner()
        .add_card(make_digimon("HOST", 6, 10000))
        .add_card(make_digimon("ATK", 6, 12000))
        .add_card(make_digimon("ATK2", 6, 12000))
        .add_card(make_composite_digimon("COMP-TARGET", 5, 3000))
        .add_card(make_digimon("SEC", 5, 2000))
        .security(0, &["SEC"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let _stack = runner.place_stack(0, &[CARD_ID, "HOST"]);
    let composite_target = runner.place_on_field(0, "COMP-TARGET", Some(0));
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    let attacker2 = runner.place_on_field(1, "ATK2", Some(0));

    // First activation this turn.
    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    runner
        .accept_optional_trigger()
        .expect("accept the first outer optional-trigger prompt");
    let choose_composite = encode_attack(0, composite_target.index as u16);
    runner
        .game
        .resolve_selection(0, choose_composite)
        .expect("resolve the first Composite retarget");
    runner.auto_resolve().ok();

    // Second attack, same turn: OPT must lock it out -- no outer prompt installs.
    runner.game.begin_attack_open(AttackOpen {
        attacker: attacker2,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must lock a second redirect attempt in the same turn"
    );
}
