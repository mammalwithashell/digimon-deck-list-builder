//! BT19-065 Machinedramon — Digimon, Lv.6, Black/Purple. DP 11000, Play cost 11.
//!
//! # Card text (data/card_bundles/BT19-065.md — official Bandai DB, verbatim)
//!
//! [On Play] [When Digivolving] Delete 1 level 5 or lower Digimon.
//! [On Deletion] You may play 1 level 5 or lower Digimon card with the
//! [Cyborg] or [Composite] trait from your trash without paying the cost.
//! (Rule) Trait: Has [Composite] Type.
//!
//! DigiXros Requirements [DigiXros -1]: 5 Lv.5 or lower [Cyborg]/[Composite]
//! trait Digimon cards w/different card numbers. When this would be played,
//! you may place specified cards from your hand/battle area under it. Each
//! one reduces the play cost.
//!
//! Inherited Effect: [Opponent's Turn] [Once Per Turn] When any of your
//! opponent's Digimon attack, you may change the attack target to 1 of your
//! Digimon with the [Composite] or [Wicked God] trait.
//!
//! Official Q&A: Yes, you can delete a Digimon (On Play targets ANY owner,
//! not just the opponent's).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Black/BT19_065.cs
//! - `#region DigiXros` (EffectTiming.None): `AddDigiXrosConditionClass` with 5
//!   repeated `DigiXrosConditionElement`s, each `CanSelectCardCondition` =
//!   owner-controlled Digimon, `HasLevel && Level <= 5`, trait Cyborg OR
//!   Composite (via `EqualsTraits`). `CanTargetCondition_ByPreSelecetedList`
//!   rejects a card whose `CardID` (card number) was already picked —
//!   "different card numbers". `maxSelect = 1` per element slot (the
//!   DigiXrosCondition ctor's last arg), 5 slots total.
//! - `#region On Play` / `#region When Digivolving` (both
//!   `EffectTiming.OnEnterFieldAnyone`): `IsLevel5Digimon` =
//!   `IsPermanentExistsOnBattleAreaDigimon` (no owner filter — ANY battle-area
//!   Digimon) && `HasLevel && Level <= 5`. `SelectPermanentEffect.SetUp` with
//!   `canNoSelect: false`, `mode: Destroy` — MANDATORY delete when a valid
//!   target exists.
//! - `#region On Deletion` (`EffectTiming.OnDestroyedAnyone` +
//!   `CanTriggerOnDeletion`, i.e. self-deletion): `IsTargetableDigimon` =
//!   owner's trash Digimon, `Level <= 5`, trait Cyborg OR Composite,
//!   `CanPlayAsNewPermanent`. `canNoSelect: () => false` on the SELECT, but
//!   the whole activation is optional (`SetUpActivateClass(..., true, ...)`)
//!   — the printed "You may" gates at the outer accept/decline, not the
//!   inner pick.
//! - `#region Opponents Turn - ESS` (`EffectTiming.OnAllyAttack`, inherited,
//!   `SetHashString("SwitchTarget_BT18-073")` — OPT identity SHARED with
//!   BT18-073's inherited clause, confirming the printed text is identical):
//!   `CanSelectPermanentCondition` = owner's Digimon with trait Composite OR
//!   Wicked God. Redirects the attack target via `SwitchDefender`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - F1-adjacent: mandatory delete-any-owner Digimon via `select_any_permanent`
//!   (no auto-select; surfaced through `pending_selection`).
//! - F5 OnDeletion → free play from trash (`play_from_trash_free`), gated
//!   optional at the outer accept/decline.
//! - C4 DigiXros source play: 5 repeated identical-filter materials,
//!   `distinct_by: card_number`, `cost_delta: -1` each (mirrors EX3-014
//!   Dorbickmon's `repeat: { min: 5, max: 5 }` shape).
//! - F2 Redirect attack, `scope: inherited` (opponent's turn, OPT) — printed
//!   in Machinedramon's INHERITED effect box (DCGO `SetIsInheritedEffect(true)`,
//!   shared OPT hash "SwitchTarget_BT18-073" with BT18-073's identical
//!   clause), so it fires only while BT19-065 is a buried digivolution
//!   source (RULES 15-3-1; `is_under != effect.inherited`, game/mod.rs:2824)
//!   — NOT BT19-072 LordKnightmon's shape, whose twin-text redirect is
//!   printed in its MAIN effect box and is correctly `scope: FaceUp`.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledDistinctBy,
    CompiledPredicate, CompiledRepeat, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{AttackTarget, SelectionKind, TriggerSource};

const CARD_ID: &str = "BT19-065";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn level5_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, 5);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.colors = vec![CardColor::Blue];
    card
}

fn level6_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, 6);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.colors = vec![CardColor::Blue];
    card
}

fn trash_digimon(id: &str, name: &str, level: u8, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(3000);
    card.colors = vec![CardColor::Black];
    card.traits = vec![trait_name.to_string()];
    card
}

fn xros_material(id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, id, 5);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(2000);
    card.colors = vec![CardColor::Black];
    card.traits = vec![trait_name.to_string()];
    card
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-065 YAML loads")
        .add_card(level5_digimon("OPP-L5", "OppFive", 5000))
        .add_card(level6_digimon("OPP-L6", "OppSix", 6000))
}

fn runner() -> DebugRunner {
    base_builder().memory(15).start()
}

fn push_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

fn trash_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s trash"))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt19_065_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-065 present");

    assert_eq!(card.name, "Machinedramon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.color, vec![CompiledColor::Black, CompiledColor::Purple]);
    assert_eq!(card.attribute.as_deref(), Some("Virus"));
    for trait_name in ["Machine", "Composite"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "BT19-065 metadata must include trait {trait_name} (Composite is Rule-granted)"
        );
    }
}

#[test]
fn bt19_065_declares_standard_digivolve_alt_path() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-065 present");
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(5) && f.color_is == Some(CompiledColor::Black)
                })
        }),
        "BT19-065 must declare the standard Lv.5 Black digivolve alt path"
    );
}

#[test]
fn bt19_065_declares_digixros_five_distinct_card_number_materials_each_minus_one() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-065 present");

    let digixros = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("BT19-065 must declare a DigiXros alt path");

    assert_eq!(
        digixros.materials.len(),
        1,
        "DigiXros recipe is a single repeated material slot"
    );
    let mat = &digixros.materials[0];
    assert_eq!(
        mat.repeat,
        Some(CompiledRepeat::Range { min: 5, max: 5 }),
        "DigiXros -1 recipe must require exactly 5 material cards"
    );
    assert_eq!(
        mat.distinct_by,
        Some(CompiledDistinctBy::CardNumber),
        "DigiXros materials must have different card numbers (not just different names)"
    );
    assert_eq!(
        mat.cost_delta,
        Some(-1),
        "each DigiXros material must reduce play cost by 1 (DigiXros -1)"
    );

    fn predicate_has_level_lte(predicate: &CompiledPredicate, level: u8) -> bool {
        predicate.level_lte == Some(digimon_dsl::compiled::CompiledDpConstraint::Literal(
            level as i32,
        )) || predicate
            .all_of
            .iter()
            .any(|p| predicate_has_level_lte(p, level))
            || predicate
                .any_of
                .iter()
                .any(|p| predicate_has_level_lte(p, level))
    }
    fn predicate_has_trait(predicate: &CompiledPredicate, trait_name: &str) -> bool {
        predicate.trait_has.as_deref() == Some(trait_name)
            || predicate
                .all_of
                .iter()
                .any(|p| predicate_has_trait(p, trait_name))
            || predicate
                .any_of
                .iter()
                .any(|p| predicate_has_trait(p, trait_name))
    }
    assert!(
        predicate_has_level_lte(&mat.filter, 5),
        "DigiXros material filter must cap level at 5 or lower"
    );
    assert!(
        predicate_has_trait(&mat.filter, "Cyborg") && predicate_has_trait(&mat.filter, "Composite"),
        "DigiXros material filter must accept [Cyborg] or [Composite] trait"
    );
}

#[test]
fn bt19_065_has_shared_on_play_when_digivolving_delete_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-065 present");

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
        .expect("BT19-065 must have a shared OnPlay/WhenDigivolving delete clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "the delete is MANDATORY per DCGO (canNoSelect: false) when a valid target exists"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "OnPlay/WhenDigivolving clause must delete a permanent"
    );
}

#[test]
fn bt19_065_has_on_deletion_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-065 present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT19-065 must have an OnDeletion clause");

    assert!(
        clause.optional,
        "[On Deletion] You MAY play free from trash — optional"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromTrashFree { .. })),
        "OnDeletion clause must play a card from trash for free"
    );
}

#[test]
fn bt19_065_has_inherited_redirect_attack_clause() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("BT19-065 present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOpponentAttack) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT19-065 must have an [Opponent's Turn] redirect clause");

    // Printed in Machinedramon's "Inherited Effect" text box, and DCGO flags
    // the ActivateClass `SetIsInheritedEffect(true)` (BT19_065.cs; identical
    // to sibling BT18-073's inherited clause, same shared OPT hash string
    // "SwitchTarget_BT18-073"). DCGO's `EffectList_ForCard` fires
    // inherited-flagged TRIGGERED effects ONLY while the card is a
    // digivolution SOURCE beneath another permanent (RULES 15-3-1), which the
    // Rust engine mirrors via `Game::activation_counts_for_permanent`'s
    // `is_under != effect.inherited` gate (game/mod.rs:2824) — so this clause
    // must compile with `scope: inherited` and fires ONLY while Machinedramon
    // is buried, NOT while it stands alone as the top of its own stack.
    // (BT19-072 LordKnightmon's identical-shape redirect is correctly FaceUp
    // because its clause is printed in BT19-072's MAIN effect box, not an
    // Inherited box — it is not a valid precedent for this card.)
    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "the redirect clause is printed in the Inherited Effect box, so it must use scope: inherited and fire only while Machinedramon is a buried digivolution source"
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

// ─── Section 2 — On Play / When Digivolving delete: condition gating ────────

#[test]
fn bt19_065_on_play_delete_installs_when_valid_target_exists() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("delete selection must install when a Lv.5-or-lower Digimon exists");
    assert_eq!(pending.kind, SelectionKind::AnyField);
    assert!(
        !pending.is_optional,
        "delete is mandatory when a valid target exists"
    );
}

#[test]
fn bt19_065_on_play_delete_no_target_installs_nothing() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    // Only a level-6 Digimon exists anywhere — no Lv.5-or-lower target.
    runner.place_on_field(1, "OPP-L6", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when no Lv.5-or-lower Digimon exists"
    );
}

#[test]
fn bt19_065_on_play_delete_offers_own_side_targets_too() {
    // Official Q&A: "Yes, you can delete a Digimon" — confirms the delete is
    // NOT restricted to the opponent's side. Own Lv.5-or-lower Digimon must
    // also be a legal target.
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    let own_target = runner.place_on_field(0, "OPP-L5", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("delete selection must install");
    let own_action = encode_attack(0, own_target.index as u16);
    assert!(
        pending.valid_action_ids.contains(&own_action),
        "own-side Lv.5-or-lower Digimon must be a legal delete target (any-owner, per Q&A)"
    );
}

// ─── Section 3 — On Play / When Digivolving delete: behavioral outcome ──────

#[test]
fn bt19_065_on_play_delete_removes_chosen_target() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let trash_before = runner.game.players[1].trash.len();
    let action_id = runner
        .pending_selection()
        .expect("delete selection installed")
        .valid_action_ids
        .first()
        .copied()
        .expect("at least one eligible delete target");
    runner
        .execute_action(0, action_id)
        .expect("delete the level 5 opponent Digimon");
    runner.auto_resolve().expect("resolve the delete");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the level 5 opponent Digimon must be deleted"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        trash_before + 1,
        "the deleted Digimon's top card moves to trash"
    );
}

#[test]
fn bt19_065_on_play_delete_excludes_level_six_targets() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));
    runner.place_on_field(1, "OPP-L6", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("delete selection installed");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the level 5 Digimon is eligible; the level 6 Digimon must be excluded"
    );
}

#[test]
fn bt19_065_when_digivolving_delete_fires_on_digivolve() {
    let mut runner = base_builder()
        .add_card(level5_digimon("BASE-L5", "BaseFive", 5000))
        .memory(15)
        .start();
    // Stack: BASE-L5 (bottom) -> BT19-065 (top), simulating a digivolve.
    let carrier = runner.place_stack(0, &["BASE-L5", CARD_ID]);
    runner.place_on_field(1, "OPP-L5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("[When Digivolving] must also surface the mandatory delete selection");
    assert_eq!(pending.kind, SelectionKind::AnyField);
}

// ─── Section 4 — On Deletion: condition gating + behavior ───────────────────

#[test]
fn bt19_065_on_deletion_no_eligible_trash_card_installs_nothing() {
    let mut runner = runner();
    let target = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "with no eligible trash card, no free-play prompt should install"
    );
}

#[test]
fn bt19_065_on_deletion_installs_optional_prompt_with_eligible_trash_card() {
    let mut runner = base_builder()
        .add_card(trash_digimon("CYBORG-L4", "CyborgFour", 4, "Cyborg"))
        .memory(15)
        .start();
    let target = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "CYBORG-L4");

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection()
        .expect("an eligible trash card must surface the OnDeletion prompt");
    assert!(
        pending.is_optional,
        "the free-play is optional (You may play...)"
    );
}

#[test]
fn bt19_065_on_deletion_excludes_trash_card_above_level_five() {
    let mut runner = base_builder()
        .add_card(trash_digimon("CYBORG-L6", "CyborgSix", 6, "Cyborg"))
        .memory(15)
        .start();
    let target = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "CYBORG-L6");

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a level 6 trash card must NOT be offered even with the right trait"
    );
}

#[test]
fn bt19_065_on_deletion_excludes_trash_card_without_matching_trait() {
    let mut runner = base_builder()
        .add_card(trash_digimon("BEAST-L4", "BeastFour", 4, "Beast"))
        .memory(15)
        .start();
    let target = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "BEAST-L4");

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a trash card without [Cyborg] or [Composite] trait must not be offered"
    );
}

#[test]
fn bt19_065_on_deletion_accepting_plays_the_selected_card_free() {
    let mut runner = base_builder()
        .add_card(trash_digimon("COMPOSITE-L3", "CompositeThree", 3, "Composite"))
        .memory(15)
        .start();
    let target = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "COMPOSITE-L3");

    let memory_before = runner.memory();
    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    // Outer accept/decline prompt (SetUpActivateClass ..., true, ...).
    runner
        .accept_optional_trigger()
        .expect("accept the OnDeletion outer optional prompt");

    let trash_action = TRASH_EFFECT_START + trash_index(&runner, 0, "COMPOSITE-L3") as u16;
    runner
        .execute_action(0, trash_action)
        .expect("select the eligible trash card to play free");
    runner.auto_resolve().expect("resolve the free play");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the free-played card must land on the battle area"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "COMPOSITE-L3"),
        "the specific selected trash card must be the one played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play-from-trash-free must not change memory (no cost paid)"
    );
}

#[test]
fn bt19_065_on_deletion_declining_leaves_trash_untouched() {
    let mut runner = base_builder()
        .add_card(trash_digimon("COMPOSITE-L3", "CompositeThree", 3, "Composite"))
        .memory(15)
        .start();
    let target = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "COMPOSITE-L3");

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_is_optional(),
        "outer accept/decline prompt must expose PASS"
    );
    let selecting_player = runner
        .pending_selection()
        .expect("outer prompt installed")
        .selecting_player;
    runner
        .execute_action(selecting_player, PASS)
        .expect("decline the OnDeletion free play");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(0),
        0,
        "declining must leave the battle area empty (nothing played)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "COMPOSITE-L3"),
        "declining must leave the eligible card in trash"
    );
}

// ─── Section 5 — DigiXros source play ────────────────────────────────────────

#[test]
fn bt19_065_digixros_five_field_materials_reduce_cost_and_attach_sources() {
    let mut runner = base_builder()
        .add_card(xros_material("XR-1", "Cyborg"))
        .add_card(xros_material("XR-2", "Composite"))
        .add_card(xros_material("XR-3", "Cyborg"))
        .add_card(xros_material("XR-4", "Composite"))
        .add_card(xros_material("XR-5", "Cyborg"))
        .memory(15)
        .hand(0, &[CARD_ID])
        .start();

    for id in ["XR-1", "XR-2", "XR-3", "XR-4", "XR-5"] {
        runner.place_on_field(0, id, Some(0));
    }

    let memory_before = runner.memory();
    let _ = runner.play(0, 0); // BT19-065 is the only hand card

    let prompt = runner
        .pending_selection()
        .expect("DigiXros play must ask for recipe materials before paying cost");
    assert!(
        prompt.is_optional,
        "DigiXros material selection must allow stopping (you MAY place)"
    );

    for slot in 0..5u16 {
        let pending = runner
            .pending_selection()
            .expect("material prompt should re-install per material");
        assert!(
            pending.valid_action_ids.contains(&slot),
            "field material slot {slot} must be selectable"
        );
        runner
            .execute_action(0, slot)
            .unwrap_or_else(|_| panic!("select DigiXros material slot {slot}"));
    }
    runner
        .execute_action(0, PASS)
        .expect("stop selecting DigiXros materials");

    // Play cost 11 reduced by 5 * -1 = -5 -> effective cost 6.
    assert_eq!(
        memory_before - runner.memory(),
        6,
        "BT19-065 cost 11 reduced by five DigiXros -1 materials should cost 6"
    );

    let machinedramon = &runner.game.players[0].battle_area[0];
    assert_eq!(
        machinedramon.top_card().card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(
        machinedramon.card_sources.len(),
        6,
        "top card plus five attached DigiXros materials"
    );
}

/// BLOCKED on G-ENGINE-DIGIXROS-DISTINCT-BY: `DigiXrosRecipeSlot.distinct_by`
/// is compiled from the YAML (`distinct_by: card_number`) but never consulted
/// by the runtime candidate filter (`slot_accepts_card` /
/// `resolve_material_origin` in `code/digimon-engine/src/digixros.rs` check
/// only name/trait requirements and exact-same-`CardHandle` dedup, not
/// "same printed card number as an already-picked material"). Two copies of
/// the identical card are therefore currently BOTH offered as legal DigiXros
/// materials, which violates the printed "5 ... cards w/different card
/// numbers" requirement. Proven via the assertion below, which fails today
/// (the second copy stays selectable) — see docs/RUST_ENGINE_GAPS.md.
#[test]
#[ignore = "pending: G-ENGINE-DIGIXROS-DISTINCT-BY from docs/RUST_ENGINE_GAPS.md"]
fn bt19_065_digixros_rejects_duplicate_card_number() {
    // Two copies of the SAME card id ("different card numbers" constraint):
    // only one of the pair may be selected as a material.
    let mut runner = base_builder()
        .add_card(xros_material("XR-DUP", "Cyborg"))
        .memory(15)
        .hand(0, &[CARD_ID])
        .start();

    let _first = runner.place_on_field(0, "XR-DUP", Some(0));
    let _second = runner.place_on_field(0, "XR-DUP", Some(0));

    let _ = runner.play(0, 0);

    let prompt = runner
        .pending_selection()
        .expect("DigiXros material prompt installs");
    // Both XR-DUP copies occupy field slots 0 and 1 at prompt-install time.
    let first_slot: u16 = 0;
    assert_eq!(
        prompt.valid_action_ids.iter().filter(|&&a| a == 0 || a == 1).count(),
        2,
        "both same-card-number copies start out selectable"
    );
    runner
        .execute_action(0, first_slot)
        .expect("select the first XR-DUP copy");

    // Picking slot 0 removes that permanent from the battle area, so the
    // second copy (originally slot 1) shifts down to slot 0. The engine
    // SHOULD now exclude it entirely (same printed card number already
    // committed to this recipe slot) — instead it re-offers slot 0 as the
    // (still-legal, per the gap) second copy.
    let after_first = runner
        .pending_selection()
        .expect("material prompt continues after first pick");
    assert!(
        !after_first.valid_action_ids.contains(&0),
        "a second card sharing the SAME card number must be excluded (different card numbers rule)"
    );
}

// ─── Section 6 — Inherited redirect: condition gating + behavior ────────────
//
// The redirect clause is printed in Machinedramon's INHERITED effect text
// box (`scope: inherited`), so it is only active while BT19-065 is buried as
// a digivolution SOURCE beneath another permanent (RULES 15-3-1; DCGO
// `EffectList_ForCard` inherited-timing gate; engine
// `Game::activation_counts_for_permanent`'s `is_under != effect.inherited`,
// game/mod.rs:2824). Every positive test below therefore buries BT19-065 via
// `place_stack(player, &["BT19-065", "<HOST>"])` (source first, host/top
// last) and fires the attack against the resulting stack handle, mirroring
// the proven EX11-020 inherited on_opponent_attack pattern
// (tests/cards_behavioral/ex11/ex11_020.rs).

/// The host on top of the stack itself carries the Rule-granted [Composite]
/// trait (mirroring Machinedramon's own grant — see BT19-065.yaml header),
/// so the condition's `any_permanent { trait_has: Composite/Wicked God }`
/// scan over the controller's own battle area is satisfied by the host
/// alone — the printed "1 of your Digimon with the [Composite] or [Wicked
/// God] trait" legally includes retargeting to the buried carrier's own top
/// card. A true negative (zero eligible targets) is therefore impossible
/// while a Composite/Wicked God host is on the field; this test instead
/// pins the eligible positive case: with ONLY the Composite host present (no
/// other Composite/Wicked God ally), the outer prompt still installs and the
/// host is the sole legal retarget.
#[test]
fn bt19_065_inherited_redirect_condition_passes_via_self_composite_trait() {
    let mut runner = base_builder()
        .add_card({
            let mut c = level6_digimon("HOST", "Host", 10000);
            c.traits = vec!["Composite".to_string()];
            c
        })
        .add_card(level6_digimon("ATK", "Attacker", 12000))
        .add_card({
            let mut c = level5_digimon("PLAIN", "Plain", 3000);
            c.traits = vec!["Beast".to_string()];
            c
        })
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // Stack: BT19-065 (buried source) -> HOST (top card, Composite trait).
    let stack = runner.place_stack(0, &[CARD_ID, "HOST"]);
    runner.place_on_field(0, "PLAIN", Some(0));
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
        .expect("accept the outer optional-trigger prompt (buried Machinedramon's inherited clause fires; host's own Composite trait satisfies the condition)");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("buried Machinedramon should prompt for a Composite/Wicked God retarget");
    assert_eq!(pending.kind, SelectionKind::OwnField);
    let choose_host = encode_attack(0, stack.index as u16);
    assert!(
        pending.valid_action_ids.contains(&choose_host),
        "the Composite-trait host must be able to redirect the attack to itself"
    );
    assert!(
        !pending.valid_action_ids.iter().any(|&a| a != choose_host),
        "PLAIN (Beast-trait) must not be offered as a retarget"
    );
}

#[test]
fn bt19_065_inherited_redirect_switches_attack_to_composite_digimon() {
    let mut runner = base_builder()
        .add_card(level6_digimon("HOST", "Host", 10000))
        .add_card(level6_digimon("ATK", "Attacker", 12000))
        .add_card({
            let mut c = level5_digimon("COMP-TARGET", "CompositeTarget", 3000);
            c.traits = vec!["Composite".to_string()];
            c
        })
        .add_card(level5_digimon("SEC", "Security", 2000))
        .security(0, &["SEC"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // Stack: BT19-065 (buried source) -> HOST (top card, no relevant trait).
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
    assert_eq!(pending.selecting_player, 0);

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

/// Negative control for the fix: with BT19-065 standing ALONE as its own top
/// card (not buried under a host), its inherited clause must NOT fire. Prior
/// to the fix this redirect was mis-modeled as `scope: FaceUp`, which is the
/// exact inverse of the printed "Inherited Effect" semantics — it would have
/// fired here (standalone) and stayed silent while buried. With
/// `scope: inherited` restored, `is_under` is false for a standalone
/// top-card permanent, so `Game::activation_counts_for_permanent`'s
/// `is_under != effect.inherited` gate excludes the clause entirely and no
/// selection installs; the attack must proceed to security uninterrupted.
#[test]
fn bt19_065_inherited_redirect_does_not_fire_when_standalone_top_card() {
    let mut runner = base_builder()
        .add_card(level6_digimon("ATK", "Attacker", 12000))
        .add_card(level5_digimon("SEC", "Security", 2000))
        .security(0, &["SEC"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // BT19-065 stands alone as its own top card — NOT buried under a host.
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
        "the inherited redirect clause must NOT fire while Machinedramon is its own standalone top card (scope: inherited requires it to be buried)"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        security_before - 1,
        "with no redirect installed, the attack proceeds and resolves security against the original defender uninterrupted by this clause"
    );
}

// ─── Section 7 — OPT lockout ─────────────────────────────────────────────────

#[test]
fn bt19_065_inherited_redirect_opt_blocks_second_activation_same_turn() {
    let mut runner = base_builder()
        .add_card(level6_digimon("HOST", "Host", 10000))
        .add_card(level6_digimon("ATK", "Attacker", 12000))
        .add_card(level6_digimon("ATK2", "Attacker2", 12000))
        .add_card({
            let mut c = level5_digimon("COMP-TARGET", "CompositeTarget", 3000);
            c.traits = vec!["Composite".to_string()];
            c
        })
        .add_card(level5_digimon("SEC", "Security", 2000))
        .security(0, &["SEC"])
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // Stack: BT19-065 (buried source) -> HOST (top card).
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

    // Second attack, same turn: OPT must lock it out — no outer prompt installs.
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
