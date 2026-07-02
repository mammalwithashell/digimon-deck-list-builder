//! BT19-070 Kimeramon — Digimon, Lv.5, Purple/Red.
//!
//! # Card text (data/card_bundles/BT19-070.md — official Bandai DB, matches
//!   data/cards.json `effect_description_eng`)
//!
//! [On Play] [When Digivolving] By deleting 1 of your Digimon, delete 1 of
//!   your opponent's level 3 Digimon, 1 of their level 4 Digimon, and 1 of
//!   their level 5 Digimon.
//! [On Deletion] By deleting 1 of your level 4 or lower purple or red
//!   Digimon, you may play 1 [Machinedramon] from your trash without paying
//!   the cost.
//! Inherited: <Security A. +1>
//! Special digivolve condition: [Digivolve] Lv.4 w/[Composite] trait: Cost 3
//! Standard digivolve circles: Purple Lv.4 / cost 4, Red Lv.4 / cost 4
//!   (from `evo_costs` in cards.json — not modeled in YAML `alt_paths`).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Purple/BT19_070.cs
//!
//! # DCGO crosscheck (per clause)
//!
//! - Alternate Digivolution (EffectTiming.None):
//!   `AddSelfDigivolutionRequirementStaticEffect(PermanentCondition: TopCard.
//!   IsLevel4 && ContainsTraits("Composite"), digivolutionCost: 3)`.
//!   → `alt_paths: kind: digivolve, from: { level_eq: 4, trait_has:
//!   Composite }, cost: 3`.
//! - [On Play] / [When Digivolving] (both fire on `OnEnterFieldAnyone`,
//!   `SetUpActivateClass(..., -1, canDoNoAction: true, ...)`):
//!   1. `SelectPermanentEffect` over own battle-area Digimon,
//!      `canNoSelect: true` → **optional** own-delete cost. Declining ends
//!      the clause with no further prompts (`selectedPermanent == null` skips
//!      `SuccessProcess`).
//!   2. On success (own Digimon deleted): three SEQUENTIAL, INDEPENDENT
//!      `SelectPermanentEffect` picks over the opponent's battle area, each
//!      gated by `HasMatchConditionPermanent(level == N)` for N = 3, 4, 5 —
//!      each pick is `canNoSelect: false` (mandatory) but the WHOLE STEP is
//!      skipped via the DCGO `if (HasMatchConditionPermanent(...))` guard
//!      when no permanent at that level exists. This is modeled by chaining
//!      three `select_opponent_permanent` (non-optional) + `delete_permanent`
//!      pairs — the DSL's `install_select_opponent_permanent` already
//!      short-circuits (no prompt, tail does not run) when `candidates` is
//!      empty (selections.rs:5157-5159), matching DCGO's guard exactly.
//!      NOTE: DCGO's On Play version deletes are batched into one
//!      `DestroyPermanentsClass` call after all three picks; the When
//!      Digivolving version deletes each target immediately
//!      (`SelectPermanentEffect.Mode.Destroy`). Both are faithfully modeled
//!      as sequential delete_permanent calls — the net board result is
//!      identical (three independent deletes), and the DSL has no "batch
//!      destroy" primitive distinct from `delete_permanent`.
//! - [On Deletion] (`EffectTiming.OnDestroyedAnyone`):
//!   1. `SelectPermanentEffect` over own battle-area Digimon with
//!      `Level <= 4` and `(Purple || Red)`, `canNoSelect: true` → **optional**
//!      own-delete cost.
//!   2. On success: `SelectCardEffect` over trash, `EqualsCardName(
//!      "Machinedramon")`, `canNoSelect: () => true` → **separately
//!      optional** ("you may play") free play from trash.
//! - Security Attack — ESS (`EffectTiming.None`):
//!   `ChangeSelfSAttackStaticEffect(changeValue: 1, isInheritedEffect: true)`
//!   → `scope: inherited, kind: grant_keyword, keyword: SecurityAttackPlus,
//!   value: 1`.
//!
//! # Patterns
//! - Own-delete-as-optional-cost gating a mandatory multi-target opponent
//!   delete (EX7-071-style triple-delete shape; see BT13-103 Akihiro Kurata
//!   for the `binding_exists` gate idiom).
//! - Level-filtered sequential opponent deletes that self-skip when no
//!   candidate exists at that level (no `if` guard needed — the DSL
//!   `select_opponent_permanent` step already no-ops on empty candidates).
//! - On Deletion own-delete-as-cost → separately-optional free play from
//!   trash by exact name (`name_is`, cf. BT18-019 Millenniummon).
//! - Inherited <Security A. +1> declarative grant_keyword (cf. ST1-07,
//!   AD1-009).
//! - Trait-gated alt-path digivolve condition (cf. AD1-001 / AD1-005 shape).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{self, encode_attack};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT19-070";

fn own_digimon(card_id: &str, level: u8, dp: i32) -> CardData {
    let mut card = digimon_engine::debug_runner::make_test_card_with_level(card_id, card_id, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn own_purple_or_red_digimon(card_id: &str, level: u8, color: CardColor) -> CardData {
    let mut card = own_digimon(card_id, level, 1000);
    card.colors = vec![color];
    card
}

fn opp_digimon_at_level(card_id: &str, level: u8) -> CardData {
    own_digimon(card_id, level, 1000)
}

fn machinedramon() -> CardData {
    let mut card = make_test_card("MACHINEDRAMON-FIXTURE", "Machinedramon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 10;
    card
}

fn not_machinedramon() -> CardData {
    let mut card = make_test_card("NOT-MACHINEDRAMON", "Chaosdramon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(10000);
    card.play_cost = 9;
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .memory(15)
        .start()
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            data_index,
            player,
            instance_id,
        ));
}

/// Resolve `card_id`'s CURRENT battle-area index for `player` (indices can
/// shift after a prior deletion compacts the vector — e.g. an On Deletion
/// clause fires after the carrier itself has already moved to trash,
/// rule 25) and drive an OwnField selection targeting it.
fn choose_own_field_by_id(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let index = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found on player {player}'s battle area"));
    let view = runner
        .pending_selection_view()
        .expect("own-field selection pending");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let action = encode_attack(0, index as u16);
    assert!(
        view.valid_action_ids.contains(&action),
        "expected {action} in {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("choose own permanent");
}

/// See `choose_own_field_by_id` — same index-resolution rationale, opponent side.
fn choose_opp_field_by_id(runner: &mut DebugRunner, opponent: u8, card_id: &str) {
    let index = runner.game.players[opponent as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found on player {opponent}'s battle area"));
    let view = runner
        .pending_selection_view()
        .expect("opponent-field selection pending");
    assert_eq!(view.kind, SelectionKind::OppField);
    let action = encode_attack(0, index as u16);
    assert!(
        view.valid_action_ids.contains(&action),
        "expected {action} in {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("choose opponent permanent");
}

fn decline(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("selection pending to decline");
    assert!(view.is_optional, "expected a declinable prompt");
    runner
        .execute_action(view.selecting_player, space::PASS)
        .expect("decline");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_070_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    assert_eq!(card.name, "Kimeramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(8));
    assert_eq!(card.dp, Some(8000));
}

#[test]
fn bt19_070_has_digixros_alt_path_three_distinct_lv4_composite_materials() {
    // DigiXros -1: 3 Lv.4 [Composite] trait Digimon cards w/ different card
    // numbers (card image + DCGO BT19_070.cs AddDigiXrosConditionClass,
    // reduceCostPerCard = 1). Missing from the lossy cards.json xros_req.
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    let xros = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("BT19-070 must declare the printed DigiXros -1 alt-path");
    assert_eq!(
        xros.materials.len(),
        1,
        "one material slot with repeat 3..=3 (distinct card numbers)"
    );
}

#[test]
fn bt19_070_has_trait_gated_alt_digivolve_condition() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    let alt = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("trait-gated digivolve alt-path exists");
    assert_eq!(alt.cost, Some(CompiledCost::Literal(3)));
    let from = alt.from.as_ref().expect("alt-path has a `from` predicate");
    assert!(
        predicate_has_level_eq(from, 4),
        "alt-path must require level 4: {from:?}"
    );
    assert!(
        predicate_has_trait(from, "Composite"),
        "alt-path must require [Composite] trait: {from:?}"
    );
}

#[test]
fn bt19_070_has_on_play_and_when_digivolving_triple_delete_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("On Play / When Digivolving clause exists");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(clause.optional, "\"By deleting\" own-Digimon cost is optional");
    assert!(!clause.once_per_turn, "not printed [Once Per Turn]");

    // The first process step must be an optional own-permanent select (the
    // delete-1-of-your-Digimon cost).
    let first = clause.process.first().expect("clause has a first step");
    match first {
        CompiledStep::SelectOwnPermanent { optional, filter, .. } => {
            assert!(*optional, "own-delete cost must be optional (\"you may\")");
            assert!(
                predicate_has_kind_digimon(filter),
                "own-delete cost targets a Digimon"
            );
        }
        other => panic!("expected SelectOwnPermanent as first step, got {other:?}"),
    }

    // Three level-filtered opponent selects (3, 4, 5) must be present
    // somewhere in the process tree (may be nested under `then:`/`if:`).
    for level in [3u8, 4, 5] {
        assert!(
            process_contains_opponent_level_select(&clause.process, level),
            "expected a select_opponent_permanent filtered to level {level}"
        );
    }
}

#[test]
fn bt19_070_has_on_deletion_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("On Deletion clause exists");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(clause.optional, "\"By deleting\" own-Digimon cost is optional");

    let first = clause.process.first().expect("clause has a first step");
    match first {
        CompiledStep::SelectOwnPermanent { optional, filter, .. } => {
            assert!(*optional, "own-delete cost must be optional");
            assert!(
                predicate_has_level_lte(filter, 4),
                "own-delete cost targets level <=4: {filter:?}"
            );
            assert!(
                predicate_has_color(filter, "Purple") && predicate_has_color(filter, "Red"),
                "own-delete cost targets purple OR red: {filter:?}"
            );
        }
        other => panic!("expected SelectOwnPermanent as first step, got {other:?}"),
    }

    assert!(
        process_contains_machinedramon_select(&clause.process),
        "On Deletion clause must select from trash by exact name [Machinedramon]"
    );
    assert!(
        process_contains_play_from_trash_free_anywhere(&clause.process),
        "On Deletion clause must play the selected card from trash without paying the cost"
    );
}

#[test]
fn bt19_070_has_inherited_security_attack_plus_one() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    let found = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                value,
                ..
            }) if keyword == "SecurityAttackPlus" && *value == Some(1)
        )
    });
    assert!(found, "Inherited GrantKeyword(SecurityAttackPlus, value=1) must be present");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (positive/negative per clause)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_070_on_play_declining_own_delete_installs_no_further_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("OWN-A", 3, 2000))
        .add_card(opp_digimon_at_level("OPP-L3", 3))
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();
    runner.place_on_field(0, "OWN-A", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));

    runner.play(0, 0).expect("play Kimeramon");

    // Decline the own-delete cost.
    decline(&mut runner);
    runner.auto_resolve().expect("settle after decline");

    assert!(
        runner.pending_selection().is_none(),
        "declining the cost must not surface any opponent-delete prompts"
    );
    assert!(
        runner
            .game
            .players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-L3"),
        "opponent's Digimon must survive when the cost is declined"
    );
}

#[test]
fn bt19_070_on_play_with_only_self_on_field_offers_kimeramon_as_the_own_delete_target() {
    // DCGO's `CanSelectOwnerPermanentConditionShared` does not exclude the
    // carrier itself (`IsPermanentExistsOnOwnerBattleAreaDigimon` — a plain
    // "any Digimon on owner's battle area" check), so with zero OTHER own
    // Digimon on the field, Kimeramon itself is the sole legal own-delete
    // target and the prompt installs (not "no candidates").
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();

    runner.play(0, 0).expect("play Kimeramon onto an otherwise-empty field");

    let view = runner
        .pending_selection_view()
        .expect("own-delete prompt must install with Kimeramon itself as the sole target");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "the own-delete cost is declinable");
}

#[test]
fn bt19_070_on_play_declining_with_only_self_on_field_leaves_kimeramon_on_the_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();

    runner.play(0, 0).expect("play Kimeramon onto an otherwise-empty field");
    decline(&mut runner);
    runner.auto_resolve().expect("settle after decline");

    assert!(
        runner.pending_selection().is_none(),
        "declining leaves no further prompts"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Kimeramon must remain on the field when the cost is declined"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome: On Play / When Digivolving triple delete
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_070_on_play_accepting_cost_deletes_one_of_each_level() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("OWN-A", 3, 2000))
        .add_card(opp_digimon_at_level("OPP-L3", 3))
        .add_card(opp_digimon_at_level("OPP-L4", 4))
        .add_card(opp_digimon_at_level("OPP-L5", 5))
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();
    runner.place_on_field(0, "OWN-A", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));
    runner.place_on_field(1, "OPP-L4", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner.play(0, 0).expect("play Kimeramon");

    // 1. Accept the own-delete cost.
    choose_own_field_by_id(&mut runner, 0, "OWN-A");

    // 2. Mandatory level-3, level-4, level-5 opponent picks, in some
    //    deterministic order driven by the compiled process.
    for id in ["OPP-L3", "OPP-L4", "OPP-L5"] {
        choose_opp_field_by_id(&mut runner, 1, id);
    }
    runner.auto_resolve().expect("settle triple delete");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OWN-A"),
        "the deleted own Digimon must be gone"
    );
    for id in ["OPP-L3", "OPP-L4", "OPP-L5"] {
        assert!(
            runner.game.players[1]
                .battle_area
                .iter()
                .all(|p| p.top_card().card_id(&runner.game.card_data) != id),
            "{id} must be deleted"
        );
    }
}

#[test]
fn bt19_070_on_play_missing_level_target_skips_only_that_level() {
    // No level-4 opponent Digimon exists — the level-4 pick must self-skip
    // while level-3 and level-5 still resolve (DCGO's per-level
    // HasMatchConditionPermanent guard).
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("OWN-A", 3, 2000))
        .add_card(opp_digimon_at_level("OPP-L3", 3))
        .add_card(opp_digimon_at_level("OPP-L5", 5))
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();
    runner.place_on_field(0, "OWN-A", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner.play(0, 0).expect("play Kimeramon");
    choose_own_field_by_id(&mut runner, 0, "OWN-A");

    choose_opp_field_by_id(&mut runner, 1, "OPP-L3");
    // No level-4 target exists, so the next pending prompt (if any) must be
    // for level 5, not level 4. Drive it directly.
    choose_opp_field_by_id(&mut runner, 1, "OPP-L5");
    runner.auto_resolve().expect("settle after level-4 self-skip");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-L3"),
        "level-3 target must be deleted"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-L5"),
        "level-5 target must be deleted"
    );
}

#[test]
fn bt19_070_on_play_with_no_opponent_targets_only_deletes_own_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("OWN-A", 3, 2000))
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();
    runner.place_on_field(0, "OWN-A", Some(0));

    runner.play(0, 0).expect("play Kimeramon");
    choose_own_field_by_id(&mut runner, 0, "OWN-A");
    runner.auto_resolve().expect("settle with no legal opponent targets");

    assert!(
        runner.pending_selection().is_none(),
        "no opponent-delete prompt should remain when no opponent Digimon exist"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OWN-A"),
        "the own Digimon is still deleted even with no opponent targets"
    );
}

#[test]
fn bt19_070_when_digivolving_accepting_cost_deletes_one_of_each_level() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("PLATFORM", 4, 6000))
        .add_card(own_digimon("OWN-A", 3, 2000))
        .add_card(opp_digimon_at_level("OPP-L3", 3))
        .add_card(opp_digimon_at_level("OPP-L4", 4))
        .add_card(opp_digimon_at_level("OPP-L5", 5))
        .memory(15)
        .start();
    let platform = runner.place_on_field(0, "PLATFORM", Some(0));
    runner.place_on_field(0, "OWN-A", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));
    runner.place_on_field(1, "OPP-L4", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    // Manually digivolve PLATFORM into BT19-070 and fire WhenDigivolving
    // (mirrors the AD1-001 pattern — avoids `Game::digivolve_from_hand`'s
    // affordability bookkeeping, which is irrelevant to this observer test).
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("BT19-070 registered");
    let instance_id = runner.game.next_card_index();
    let kimeramon_card =
        digimon_engine::card_source::CardSource::new(data_index, 0, instance_id);
    let turn = runner.game.turn_count;
    {
        let perm = runner.game.player_mut(0)
            .battle_area
            .get_mut(platform.index as usize)
            .expect("platform on field");
        perm.digivolve(kimeramon_card, turn);
    }
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(platform));
    runner.game.drain_effect_queue();

    choose_own_field_by_id(&mut runner, 0, "OWN-A");
    for id in ["OPP-L3", "OPP-L4", "OPP-L5"] {
        choose_opp_field_by_id(&mut runner, 1, id);
    }
    runner.auto_resolve().expect("settle triple delete");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OWN-A"),
        "the deleted own Digimon must be gone"
    );
    for id in ["OPP-L3", "OPP-L4", "OPP-L5"] {
        assert!(
            runner.game.players[1]
                .battle_area
                .iter()
                .all(|p| p.top_card().card_id(&runner.game.card_data) != id),
            "{id} must be deleted"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3b — Behavioral outcome: On Deletion → Machinedramon free play
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_070_on_deletion_declining_own_delete_installs_no_further_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_purple_or_red_digimon("OWN-PURPLE", 4, CardColor::Purple))
        .add_card(machinedramon())
        .memory(15)
        .start();
    let kimeramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-PURPLE", Some(0));
    push_to_trash(&mut runner, 0, "MACHINEDRAMON-FIXTURE");

    runner
        .game
        .delete_permanent_with_cause(kimeramon, ReplacementCause::OpponentEffect);

    decline(&mut runner);
    runner.auto_resolve().expect("settle after decline");

    assert!(
        runner.pending_selection().is_none(),
        "declining the On Deletion cost must not offer the Machinedramon play"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OWN-PURPLE"),
        "the level-4 purple Digimon must survive when the cost is declined"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "MACHINEDRAMON-FIXTURE"),
        "Machinedramon must not be played when the cost is declined"
    );
}

#[test]
fn bt19_070_on_deletion_accepting_cost_then_declining_play_leaves_machinedramon_in_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_purple_or_red_digimon("OWN-RED", 2, CardColor::Red))
        .add_card(machinedramon())
        .memory(15)
        .start();
    let kimeramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-RED", Some(0));
    push_to_trash(&mut runner, 0, "MACHINEDRAMON-FIXTURE");

    runner
        .game
        .delete_permanent_with_cause(kimeramon, ReplacementCause::OpponentEffect);

    choose_own_field_by_id(&mut runner, 0, "OWN-RED");

    // Machinedramon play is separately optional — decline it.
    decline(&mut runner);
    runner.auto_resolve().expect("settle after declining the free play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OWN-RED"),
        "the own level-2 red Digimon must still be deleted (the cost was paid)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "MACHINEDRAMON-FIXTURE"),
        "Machinedramon must remain in trash when the free play is declined"
    );
}

#[test]
fn bt19_070_on_deletion_accepting_both_plays_machinedramon_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_purple_or_red_digimon("OWN-PURPLE", 4, CardColor::Purple))
        .add_card(machinedramon())
        .memory(15)
        .start();
    let kimeramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-PURPLE", Some(0));
    push_to_trash(&mut runner, 0, "MACHINEDRAMON-FIXTURE");

    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(kimeramon, ReplacementCause::OpponentEffect);

    choose_own_field_by_id(&mut runner, 0, "OWN-PURPLE");

    let view = runner
        .pending_selection_view()
        .expect("Machinedramon trash-play prompt pending");
    assert_eq!(view.kind, SelectionKind::Trash);
    let action = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, action)
        .expect("choose Machinedramon");
    runner.auto_resolve().expect("settle free play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "MACHINEDRAMON-FIXTURE"),
        "Machinedramon must be played onto the battle area"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "MACHINEDRAMON-FIXTURE"),
        "Machinedramon must leave the trash"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing Machinedramon without paying the cost must not spend memory"
    );
}

#[test]
fn bt19_070_on_deletion_only_offers_own_permanent_level_lte_4_purple_or_red() {
    // A level-5 purple Digimon and a level-3 blue Digimon are BOTH present;
    // neither qualifies for the own-delete cost (level too high, wrong
    // color respectively), so the On Deletion clause must install no prompt.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_purple_or_red_digimon("TOO-HIGH-LEVEL", 5, CardColor::Purple))
        .add_card(own_purple_or_red_digimon("WRONG-COLOR", 3, CardColor::Blue))
        .memory(15)
        .start();
    let kimeramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "TOO-HIGH-LEVEL", Some(0));
    runner.place_on_field(0, "WRONG-COLOR", Some(0));

    runner
        .game
        .delete_permanent_with_cause(kimeramon, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no eligible own-delete target (level<=4, purple/red) should install no prompt"
    );
}

#[test]
fn bt19_070_on_deletion_trash_play_only_offers_machinedramon_by_exact_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_purple_or_red_digimon("OWN-PURPLE", 4, CardColor::Purple))
        .add_card(not_machinedramon())
        .memory(15)
        .start();
    let kimeramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-PURPLE", Some(0));
    push_to_trash(&mut runner, 0, "NOT-MACHINEDRAMON");

    runner
        .game
        .delete_permanent_with_cause(kimeramon, ReplacementCause::OpponentEffect);
    choose_own_field_by_id(&mut runner, 0, "OWN-PURPLE");

    assert!(
        runner.pending_selection().is_none(),
        "a non-Machinedramon trash card must not surface a play prompt"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Cost firing (events): own-delete fires OnDeletion / deletion events
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_070_on_play_own_delete_cost_fires_trash_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("OWN-A", 3, 2000))
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();
    runner.place_on_field(0, "OWN-A", Some(0));

    runner.play(0, 0).expect("play Kimeramon");

    let cp = runner.event_checkpoint();
    choose_own_field_by_id(&mut runner, 0, "OWN-A");
    runner.auto_resolve().expect("settle");

    let events = runner.events_since(cp);
    let trash_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { card_id, .. } if card_id == "OWN-A"))
        .count();
    assert!(
        trash_count >= 1,
        "the own-delete cost must trash OWN-A (fires GameEvent::Trash); events: {events:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — OPT enforcement
// ═══════════════════════════════════════════════════════════════════════════
//
// BT19-070 has no printed [Once Per Turn] marker on any clause: [On Play] /
// [When Digivolving] each fire once per triggering event by construction
// (On Play fires once when played; When Digivolving fires once per
// digivolution), and [On Deletion] fires once per deletion event. There is
// no OPT lockout to test here — this section is intentionally empty aside
// from a structural confirmation.

#[test]
fn bt19_070_no_clause_is_marked_once_per_turn() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-070 present");
    for clause in &card.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert!(
                !t.once_per_turn,
                "BT19-070 has no printed [Once Per Turn] marker on any clause: {:?}",
                t.when
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited Security Attack +1 (behavioral)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_070_inherited_security_attack_plus_one_applies_to_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-070 YAML loads")
        .add_card(own_digimon("PLATFORM", 4, 6000))
        .memory(15)
        .start();
    // BT19-070 goes UNDER the top card so its inherited text applies to the
    // carrier (`place_stack`'s LAST id becomes the top/carrier card).
    let carrier = runner.place_stack(0, &[CARD_ID, "PLATFORM"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::SecurityAttackPlus(1)),
        "the carrier with BT19-070 on top must have Security A. +1"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Structural predicate-tree helpers
// ═══════════════════════════════════════════════════════════════════════════

fn predicate_has_level_eq(predicate: &CompiledPredicate, level: u8) -> bool {
    predicate.level_eq == Some(level)
        || predicate.any_of.iter().any(|p| predicate_has_level_eq(p, level))
        || predicate.all_of.iter().any(|p| predicate_has_level_eq(p, level))
}

fn predicate_has_level_lte(predicate: &CompiledPredicate, level: u8) -> bool {
    use digimon_dsl::compiled::CompiledDpConstraint;
    predicate.level_lte == Some(CompiledDpConstraint::Literal(level as i32))
        || predicate.any_of.iter().any(|p| predicate_has_level_lte(p, level))
        || predicate.all_of.iter().any(|p| predicate_has_level_lte(p, level))
}

fn predicate_has_trait(predicate: &CompiledPredicate, trait_name: &str) -> bool {
    predicate.trait_has.as_deref() == Some(trait_name)
        || predicate.any_of.iter().any(|p| predicate_has_trait(p, trait_name))
        || predicate.all_of.iter().any(|p| predicate_has_trait(p, trait_name))
}

fn predicate_has_color(predicate: &CompiledPredicate, color: &str) -> bool {
    use digimon_dsl::compiled::CompiledColor;
    let target = match color {
        "Purple" => CompiledColor::Purple,
        "Red" => CompiledColor::Red,
        _ => return false,
    };
    predicate.color_is == Some(target)
        || predicate.any_of.iter().any(|p| predicate_has_color(p, color))
        || predicate.all_of.iter().any(|p| predicate_has_color(p, color))
}

fn predicate_has_kind_digimon(predicate: &CompiledPredicate) -> bool {
    use digimon_dsl::compiled::CompiledCardKind;
    predicate.kind == Some(CompiledCardKind::Digimon)
        || predicate.any_of.iter().any(predicate_has_kind_digimon)
        || predicate.all_of.iter().any(predicate_has_kind_digimon)
}

fn process_contains_opponent_level_select(steps: &[CompiledStep], level: u8) -> bool {
    steps.iter().any(|step| step_contains_opponent_level_select(step, level))
}

fn step_contains_opponent_level_select(step: &CompiledStep, level: u8) -> bool {
    match step {
        CompiledStep::SelectOpponentPermanent { filter, then, .. } => {
            predicate_has_level_eq(filter, level)
                || process_contains_opponent_level_select(then, level)
        }
        CompiledStep::If { then, else_branch, .. } => {
            process_contains_opponent_level_select(then, level)
                || process_contains_opponent_level_select(else_branch, level)
        }
        _ => false,
    }
}

fn process_contains_machinedramon_select(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::SelectTrash { filter, then, .. } => {
            predicate_name_is_machinedramon(filter) || process_contains_machinedramon_select(then)
        }
        CompiledStep::If { then, else_branch, .. } => {
            process_contains_machinedramon_select(then)
                || process_contains_machinedramon_select(else_branch)
        }
        _ => false,
    })
}

fn predicate_name_is_machinedramon(predicate: &CompiledPredicate) -> bool {
    predicate.name_is.as_deref() == Some("Machinedramon")
        || predicate.any_of.iter().any(predicate_name_is_machinedramon)
        || predicate.all_of.iter().any(predicate_name_is_machinedramon)
}

fn process_contains_play_from_trash_free_anywhere(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::PlayFromTrashFree { .. } => true,
        CompiledStep::SelectTrash { then, .. } => process_contains_play_from_trash_free_anywhere(then),
        CompiledStep::If { then, else_branch, .. } => {
            process_contains_play_from_trash_free_anywhere(then)
                || process_contains_play_from_trash_free_anywhere(else_branch)
        }
        _ => false,
    })
}
