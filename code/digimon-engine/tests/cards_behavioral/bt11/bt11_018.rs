//! BT11-018 Shoutmon DX — Digimon, Lv.6, Red/Blue, DP 12000, Cost 13.
//! Traits: Composite / Xros Heart / Blue Flare. Attribute: Data.
//!
//! # Card text (data/card_bundles/BT11-018.md — official Bandai DB, verbatim)
//!
//! The name of this card/Digimon is also treated as [OmniShoutmon] and
//! [ZeigGreymon]. ＜Material Save 2＞ (When this Digimon would be deleted,
//! you may place 2 cards in this Digimon's DigiXros requirements from this
//! Digimon's digivolution cards under 1 of your Tamers.)
//! [On Play] Delete 1 of your opponent's Digimon with 8000 DP or less. 1 of
//! your opponent's Digimon can't attack until the end of their turn.
//! [End of Attack] By deleting this Digimon, gain 1 memory.
//!
//! Special Play Condition:
//! DigiXros -3: [OmniShoutmon] x [ZeigGreymon] When you would play this
//! card, you may place specified cards from your hand/battle area under it.
//! Each placed card reduces the play cost.
//!
//! Standard digivolve (cards.json evo_costs): Lv.5 Red / Cost 4.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT11/Red/BT11_018.cs
//!
//! Crosscheck:
//! - `EffectTiming.None` — `ChangeCardNamesClass` adds "OmniShoutmon" and
//!   "ZeigGreymon" to this card's names (name-rule alias, unconditional).
//! - `EffectTiming.WhenPermanentWouldBeDeleted` —
//!   `CardEffectFactory.MaterialSaveEffect(card, materialSaveCount: 2)`.
//! - `EffectTiming.OnEnterFieldAnyone` [On Play] — `SetUpActivateClass(...,
//!   -1, false, ...)` = MANDATORY top-level. Two INDEPENDENT sub-pools, each
//!   `canNoSelect: false` (mandatory) when a legal target exists in that
//!   pool:
//!     (a) `CanSelectPermanentCondition`: opp battle-area Digimon with
//!         `DP <= 8000` → `SelectPermanentEffect(..., mode: Destroy)`.
//!     (b) `CanSelectPermanentCondition1`: ANY opp battle-area Digimon (no
//!         DP filter) → custom select → `GainCanNotAttack(...,
//!         EffectDuration.UntilOpponentTurnEnd)`.
//!   Each pool is offered/resolved independently — pool (b) is not gated on
//!   pool (a) succeeding.
//! - `EffectTiming.OnEndAttack` [End of Attack] — `SetUpActivateClass(...,
//!   -1, true, ...)` = OPTIONAL ("By deleting this Digimon, gain 1
//!   memory" is a may-cost-for-effect). `CanActivateCondition` gates on
//!   `CanBeDestroyedBySkill`. Deletes self via
//!   `DeletePeremanentAndProcessAccordingToResult`, then `AddMemory(1)` in
//!   the success continuation.
//! - `EffectTiming.None` (2nd block) — `AddDigiXrosConditionClass`:
//!   `DigiXrosCondition(elements: [OmniShoutmon(1), ZeigGreymon(1)], null,
//!   3)` — trailing ctor arg `3` = `reduceCostPerCard` (per-material cost
//!   delta), matching the JSON `digixros_costs[0].reduce_cost_per_card: 3`
//!   and `max_materials: 2` (one of each named material).
//!
//! # Patterns
//! - G1 also_treated_as / name aliases (BT23-077 idiom)
//! - Xros Heart Material Save (BT10-009/BT10-013 idiom, grant_keyword
//!   MaterialSave value: 2)
//! - C4 DigiXros source play (named materials, BT10-009/EX10-031 idiom)
//! - On Play: two independent mandatory sub-pools (delete DP<=8000 +
//!   CannotAttack any) — BT25-071 / BT12-028 idiom for `CannotAttack` +
//!   `end_of_opponents_turn`
//! - End of Attack: optional self-delete-for-memory (BT21-021-style "By
//!   deleting this Digimon" may-cost)

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::events::GameEvent;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn opp_digimon(card_id: &str, name: &str, level: u8, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.dp = Some(dp);
    card
}

fn xros_material(card_id: &str, name: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn test_tamer(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 0);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT11-018")
        .expect("BT11-018 YAML loads")
        .add_card(opp_digimon("OPP-LOW-DP", "Weak Foe", 4, 6000))
        .add_card(opp_digimon("OPP-HIGH-DP", "Strong Foe", 6, 12000))
        .add_card(xros_material("BT11-MAT-OMNI", "OmniShoutmon", 5))
        .add_card(xros_material("BT11-MAT-ZEIG", "ZeigGreymon", 5))
        .add_card(test_tamer("XROS-TAMER", "Xros Heart Tamer"))
}

fn base_runner() -> DebugRunner {
    // Memory gauge clamps to [-10, 10]; start well below the ceiling so
    // `gain_memory: 1` tests have headroom.
    builder().memory(0).start()
}

fn fire_on_play(runner: &mut DebugRunner, handle: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn fire_end_of_attack(runner: &mut DebugRunner, handle: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ─── Structural assertions ─────────────────────────────────────────────────

#[test]
fn bt11_018_declares_material_save_two_keyword() {
    let runner = base_runner();
    let card = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 compiled card present");

    let has_material_save = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
                    keyword,
                    value,
                    ..
                }
            ) if keyword == "MaterialSave" && *value == Some(2)
        )
    });
    assert!(
        has_material_save,
        "BT11-018 must grant <Material Save 2> via a declarative GrantKeyword clause"
    );
}

#[test]
fn bt11_018_also_treated_as_omnishoutmon_and_zeiggreymon() {
    let runner = base_runner();
    let card = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 compiled card present");
    let aliases: Vec<&str> = card.also_treated_as.iter().map(|s| s.as_str()).collect();
    assert!(
        aliases.contains(&"OmniShoutmon"),
        "must be also treated as OmniShoutmon"
    );
    assert!(
        aliases.contains(&"ZeigGreymon"),
        "must be also treated as ZeigGreymon"
    );
}

#[test]
fn bt11_018_declares_digixros_minus_three_recipe() {
    let runner = base_runner();
    let card = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 compiled card present");

    let digixros = card
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::DigiXros)
        .expect("BT11-018 must declare a DigiXros alt path");

    for required_name in ["OmniShoutmon", "ZeigGreymon"] {
        let mat = digixros
            .materials
            .iter()
            .find(|mat| {
                mat.filter
                    .name_contains
                    .as_deref()
                    .map(|n| n == required_name)
                    .unwrap_or(false)
                    || mat.filter.name_is.as_deref() == Some(required_name)
            })
            .unwrap_or_else(|| panic!("DigiXros recipe must include {required_name}"));
        assert_eq!(
            mat.cost_delta,
            Some(-3),
            "{required_name} material must reduce cost by 3 (reduce_cost_per_card)"
        );
    }
}

#[test]
fn bt11_018_has_on_play_and_end_of_attack_clauses() {
    let runner = base_runner();
    let card = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 compiled card present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let on_play = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("BT11-018 must have an On Play clause");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(
        !on_play.optional,
        "[On Play] delete/cannot-attack body is mandatory at the top level"
    );

    let end_of_attack = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfAttack))
        .expect("BT11-018 must have an End of Attack clause");
    assert_eq!(end_of_attack.scope, CompiledScope::FaceUp);
    assert!(
        end_of_attack.optional,
        "[End of Attack] 'By deleting this Digimon, gain 1 memory' is optional (may-cost)"
    );
}

// ─── [On Play] condition gating ────────────────────────────────────────────

#[test]
fn bt11_018_on_play_no_selection_when_opponent_has_no_digimon() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", None);

    fire_on_play(&mut runner, dx);

    assert!(
        runner.pending_selection().is_none(),
        "no On Play selection should install when opponent has no Digimon"
    );
}

#[test]
fn bt11_018_on_play_offers_delete_selection_when_low_dp_opponent_present() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", None);
    runner.place_on_field(1, "OPP-LOW-DP", Some(0));

    fire_on_play(&mut runner, dx);

    let prompt = runner
        .pending_selection()
        .expect("delete pool must prompt when a DP<=8000 opponent Digimon exists");
    assert_eq!(prompt.selecting_player, 0);
    assert!(
        !prompt.is_optional,
        "delete-pool select is mandatory when a legal target exists"
    );
}

// ─── [On Play] behavioral — delete pool ────────────────────────────────────

#[test]
fn bt11_018_on_play_deletes_low_dp_opponent_digimon() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", None);
    runner.place_on_field(1, "OPP-LOW-DP", Some(0));
    let field_before = runner.battle_area_size(1);

    fire_on_play(&mut runner, dx);

    let prompt = runner
        .pending_selection()
        .expect("delete pool prompt must install");
    let action = prompt.valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select the low-DP opponent Digimon for deletion");

    // Resolve any follow-up (the independent cannot-attack pool) with the
    // first available legal action so we can assert the deletion outcome.
    if runner.pending_selection().is_some() {
        runner
            .auto_resolve()
            .expect("resolve remaining On Play sub-effect");
    }

    assert_eq!(
        runner.battle_area_size(1),
        field_before - 1,
        "the selected low-DP opponent Digimon must be deleted"
    );
}

#[test]
fn bt11_018_on_play_delete_pool_excludes_high_dp_opponent() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", None);
    runner.place_on_field(1, "OPP-HIGH-DP", Some(0));

    fire_on_play(&mut runner, dx);

    // Delete pool must NOT install for the 12000 DP body, but the
    // cannot-attack pool (any opponent Digimon, no DP filter) still may.
    if runner.pending_selection().is_some() {
        runner
            .auto_resolve()
            .expect("resolve On Play sub-effect(s)");
    }
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the 12000 DP opponent Digimon must survive — delete pool excludes DP>8000"
    );
}

// ─── [On Play] behavioral — cannot-attack pool ─────────────────────────────

#[test]
fn bt11_018_on_play_applies_cannot_attack_until_end_of_opponents_turn() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", None);
    let target = runner.place_on_field(1, "OPP-HIGH-DP", Some(0));

    fire_on_play(&mut runner, dx);

    let prompt = runner
        .pending_selection()
        .expect("cannot-attack pool must prompt when an opponent Digimon exists");
    assert!(
        !prompt.is_optional,
        "cannot-attack select is mandatory when a legal target exists"
    );
    let action = prompt.valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select opponent Digimon to be locked from attacking");
    if runner.pending_selection().is_some() {
        runner
            .auto_resolve()
            .expect("resolve remaining On Play sub-effect");
    }

    assert!(
        runner.modifiers().has(target, ModifierType::CannotAttack),
        "selected opponent Digimon must gain CannotAttack"
    );
}

#[test]
fn bt11_018_on_play_no_cannot_attack_selection_with_no_opponent_digimon() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", None);

    fire_on_play(&mut runner, dx);

    assert!(
        runner.pending_selection().is_none(),
        "no target pool exists — no selection should install"
    );
}

// ─── [End of Attack] behavioral ────────────────────────────────────────────

#[test]
fn bt11_018_end_of_attack_prompt_is_optional() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", Some(0));

    fire_end_of_attack(&mut runner, dx);

    let prompt = runner
        .pending_selection()
        .expect("End of Attack must prompt whether to delete self for memory");
    assert!(
        prompt.is_optional,
        "'By deleting this Digimon, gain 1 memory' is optional"
    );
}

#[test]
fn bt11_018_end_of_attack_accepting_deletes_self_and_gains_memory() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", Some(0));

    let memory_before = runner.memory();
    let field_before = runner.battle_area_size(0);

    fire_end_of_attack(&mut runner, dx);

    let prompt = runner
        .pending_selection()
        .expect("End of Attack prompt must install");
    let accept_action = prompt.valid_action_ids[0];
    runner
        .execute_action(0, accept_action)
        .expect("accept the self-delete for memory");
    runner
        .auto_resolve()
        .expect("resolve remaining follow-ups");

    assert_eq!(
        runner.battle_area_size(0),
        field_before - 1,
        "Shoutmon DX must be deleted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "controller must gain 1 memory after the self-deletion"
    );
}

#[test]
fn bt11_018_end_of_attack_declining_keeps_self_and_no_memory_gain() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", Some(0));

    let memory_before = runner.memory();
    let field_before = runner.battle_area_size(0);

    fire_end_of_attack(&mut runner, dx);

    let prompt = runner
        .pending_selection()
        .expect("End of Attack prompt must install");
    assert!(prompt.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline the self-delete for memory");
    runner
        .auto_resolve()
        .expect("resolve remaining follow-ups");

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "Shoutmon DX must remain on the field when declined"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the self-deletion is declined"
    );
}

#[test]
fn bt11_018_end_of_attack_self_deletion_fires_deletion_event() {
    let mut runner = base_runner();
    let dx = runner.place_on_field(0, "BT11-018", Some(0));

    fire_end_of_attack(&mut runner, dx);

    let cp = runner.event_checkpoint();
    let prompt = runner
        .pending_selection()
        .expect("End of Attack prompt must install");
    let accept_action = prompt.valid_action_ids[0];
    runner
        .execute_action(0, accept_action)
        .expect("accept the self-delete for memory");
    runner
        .auto_resolve()
        .expect("resolve remaining follow-ups");

    let events = runner.events_since(cp);
    assert!(
        events.iter().any(|e| matches!(
            e,
            GameEvent::Trash { card_id, .. } if card_id == "BT11-018"
        )),
        "a Trash event for BT11-018 must fire when Shoutmon DX deletes itself"
    );
}

// ─── DigiXros play — behavioral ────────────────────────────────────────────

#[test]
fn bt11_018_digixros_two_field_materials_reduce_cost_and_attach_sources() {
    let mut runner = builder()
        .memory(20)
        .hand(0, &["BT11-018"])
        .start();
    runner.place_on_field(0, "BT11-MAT-OMNI", Some(0));
    runner.place_on_field(0, "BT11-MAT-ZEIG", Some(0));

    let memory_before = runner.memory();
    let hand_index = runner.hand_size(0) - 1;
    let _ = runner.play(0, hand_index);

    // DigiXros material selection precedes the On Play resolution.
    let mut picked = 0;
    while matches!(runner.pending_kind(), Some(SelectionKind::Material)) {
        let action = runner
            .pending_selection()
            .expect("Material selection must be pending")
            .valid_action_ids[0];
        runner
            .execute_action(0, action)
            .expect("select a DigiXros material");
        picked += 1;
        if picked >= 2 {
            break;
        }
    }
    assert_eq!(picked, 2, "both OmniShoutmon and ZeigGreymon materials should be selectable");

    // Stop selecting further materials if the transaction prompts again.
    if matches!(runner.pending_kind(), Some(SelectionKind::Material)) {
        runner
            .execute_action(0, PASS)
            .expect("stop selecting DigiXros materials");
    }

    if runner.pending_selection().is_some() {
        runner
            .auto_resolve()
            .expect("resolve remaining On Play sub-effects with no opponent Digimon");
    }

    assert_eq!(
        memory_before - runner.memory(),
        13 - 3 - 3,
        "cost 13 reduced by two DigiXros -3 materials should cost 7"
    );

    let dx = &runner.game.players[0].battle_area[0];
    assert_eq!(dx.top_card().card_id(&runner.game.card_data), "BT11-018");
    assert!(
        dx.card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT11-MAT-OMNI"),
        "OmniShoutmon source should be attached"
    );
    assert!(
        dx.card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT11-MAT-ZEIG"),
        "ZeigGreymon source should be attached"
    );
}

// ─── Material Save (would-be-deleted) — behavioral ─────────────────────────

#[test]
fn bt11_018_material_save_two_is_optional_and_filters_recipe_sources_under_tamer() {
    let mut runner = base_runner();
    let dx = runner.place_stack(0, &["BT11-MAT-OMNI", "BT11-MAT-ZEIG", "BT11-018"]);
    runner.place_on_field(0, "XROS-TAMER", Some(0));

    runner.game.delete_permanent_with_cause(
        dx,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    let tamer_prompt = runner
        .pending_selection()
        .expect("Material Save must ask which Tamer receives saved sources");
    assert_eq!(tamer_prompt.selecting_player, 0);
    assert!(
        tamer_prompt.is_optional,
        "Material Save is 'you may' and must be declinable at the Tamer destination prompt"
    );

    let tamer_action = tamer_prompt.valid_action_ids[0];
    runner
        .execute_action(0, tamer_action)
        .expect("choose destination Tamer");

    for _ in 0..2 {
        let source_prompt = runner
            .pending_selection()
            .expect("Material Save should ask for up to 2 eligible recipe sources");
        let action = source_prompt.valid_action_ids[0];
        runner
            .execute_action(0, action)
            .expect("select eligible Material Save source");
    }

    let tamer_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "XROS-TAMER")
        .expect("chosen Tamer remains on the field");
    let tamer_source_ids: Vec<_> = tamer_perm
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    for expected in ["BT11-MAT-OMNI", "BT11-MAT-ZEIG"] {
        assert!(
            tamer_source_ids.iter().any(|id| id == expected),
            "{expected} should be saved under the chosen Tamer"
        );
    }
}

#[test]
fn bt11_018_material_save_declining_leaves_sources_in_trash() {
    let mut runner = base_runner();
    let dx = runner.place_stack(0, &["BT11-MAT-OMNI", "BT11-MAT-ZEIG", "BT11-018"]);
    runner.place_on_field(0, "XROS-TAMER", Some(0));

    let trash_before = runner.trash_size(0);
    runner.game.delete_permanent_with_cause(
        dx,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    let tamer_prompt = runner
        .pending_selection()
        .expect("Material Save prompt must install");
    assert!(tamer_prompt.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline Material Save");
    runner
        .auto_resolve()
        .expect("resolve any remaining follow-up");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 3,
        "declining Material Save leaves all 3 cards (top card + 2 sources) in trash"
    );
}
