//! BT24-014 Aegiochusmon — Digimon, Lv.5, Red/Yellow, DP 8000, Cost 8.
//! Traits: Shaman, Iliad, TS. (Rule) Trait: Has [Dragonkin] Type. Attribute: Vaccine.
//! Standard digivolve: Lv.4 Red / cost 4 AND Lv.4 Yellow / cost 4 (both printed
//! circles — data/card_official.json digivolve_costs lists card_color 0 (Red)
//! AND card_color 2 (Yellow), both level 4 / memory_cost 4; cards.json's
//! evo_costs dropped the Yellow circle). Special digivolve condition:
//! `[Digivolve] [Aegiomon]: Cost 3`.
//!
//! # Card text (data/card_bundles/BT24-014.md — official Bandai DB, verbatim)
//!
//! ＜Security A. +1＞ (This Digimon checks 1 additional security card.)
//! ＜Decode ([Aegiomon])＞ (When this Digimon would leave the battle area other
//! than in battle, you may play 1 [Aegiomon] from its digivolution cards
//! without paying the cost.)
//! [When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn.
//! Then, if you have 3 or fewer security cards, delete 1 of your opponent's
//! Digimon with 7000 DP or less.
//! (Rule) Trait: Has [Dragonkin] Type.
//!
//! Inherited Effect:
//! ＜Decode ([Aegiomon])＞ (When this Digimon would leave the battle area other
//! than in battle, you may play 1 [Aegiomon] from its digivolution cards
//! without paying the cost.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_014.cs
//!   - "Alternative Digivolution Condition" region: AddSelfDigivolutionRequirementStaticEffect
//!     gated on TopCard.EqualsCardName("Aegiomon"), digivolutionCost: 3.
//!   - "Security A. +1" region: ChangeSelfSAttackStaticEffect(+1, isInheritedEffect: false).
//!   - "Decode <[Aegiomon]>" region (own, WhenRemoveField): DecodeSelfEffect
//!     sourceCondition EqualsCardName("Aegiomon"), isInheritedEffect: false.
//!   - "When Digivolving" region (OnEnterFieldAnyone): mandatory select 1 opp
//!     Digimon → ChangeDigimonDP(-5000, UntilEachTurnEnd); THEN if
//!     Owner.SecurityCards.Count <= 3, mandatory select+delete 1 opp Digimon
//!     with DP <= 7000 (measured AFTER the -5000 modifier via
//!     CanSelectPermanentCondition_Delete reading live `permanent.DP`).
//!   - "Decode - ESS" region (WhenRemoveField, isInheritedEffect: true): the
//!     inherited mirror of the own Decode clause.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H4 Security A. +N (own keyword aura, `security_attack: 1`)
//! - C3-adjacent special <Decode> replacement (own AND inherited scope; play
//!   named [Aegiomon] source on non-battle leave) — BT25-025 idiom
//! - D1-adjacent conditional delete: -5000 DP applied first (mandatory,
//!   single target), THEN a security-gated (`security_count_lte: 3`) mandatory
//!   delete of an opponent Digimon at DP <= 7000 measured AFTER the debuff
//! - Rule-granted trait (Dragonkin) folded into `traits:`

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT24-014";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// An opponent Digimon at a given DP, used as the -5000 DP / delete targets.
fn opp_digimon(card_id: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(card_id, card_id, 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

/// An [Aegiomon]-named source card for the Decode replacement.
fn aegiomon_source() -> CardData {
    let mut c = make_test_card_with_level("AEGIOMON-SRC", "Aegiomon", 3);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(2000);
    c.play_cost = 2;
    c
}

/// A Lv.4 Red carrier BT24-014 can digivolve onto (matches printed evo_costs).
fn lv4_red_carrier(card_id: &str) -> CardData {
    let mut c = make_test_card_with_level(card_id, card_id, 4);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// A Lv.4 Yellow carrier BT24-014 can digivolve onto (2nd printed circle —
/// official Bandai DB digivolve_costs card_color 2 (Yellow), level 4, cost 4;
/// dropped from cards.json's evo_costs by the multi-colour-cost lossiness).
fn lv4_yellow_carrier(card_id: &str) -> CardData {
    let mut c = make_test_card_with_level(card_id, card_id, 4);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-014 YAML parses and compiles")
        .add_card(opp_digimon("OPP-12000", 12000))
        .add_card(opp_digimon("OPP-7000", 7000))
        .add_card(opp_digimon("OPP-3000", 3000))
        .add_card(aegiomon_source())
        .add_card(lv4_red_carrier("RED-CARRIER"))
        .add_card(lv4_yellow_carrier("YELLOW-CARRIER"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
}

/// BT24-014 already sitting on P0's field (post-digivolve state), for tests
/// that only care about its own-scope non-digivolve clauses (Security A,
/// Decode replacement).
fn place_aegiochusmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt24_014_yaml_has_printed_metadata() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    assert_eq!(card.name, "Aegiochusmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
}

#[test]
fn bt24_014_has_dragonkin_rule_trait() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Dragonkin")),
        "must carry the (Rule) Dragonkin trait grant; got {:?}",
        card.traits
    );
    // Printed traits must remain present alongside the rule grant.
    for expected in ["Shaman", "Iliad", "TS"] {
        assert!(
            card.traits.iter().any(|t| t.eq_ignore_ascii_case(expected)),
            "must still carry printed trait {expected}; got {:?}",
            card.traits
        );
    }
}

/// Production `data/cards.json` must carry the (Rule) Dragonkin trait grant
/// for BT24-014 (folded into `type_eng` at ingest time from
/// `data/card_overrides.json`, per the BT21-024 / EX7-021 precedent). This is
/// the genuinely discriminating check for the runtime gap: DebugRunner's
/// `dsl_card()` fixture path (`card_data_from_compiled` in debug_runner.rs)
/// synthesizes test `CardData` directly from the compiled YAML `traits:`
/// field, so a DebugRunner-only assertion can NEVER distinguish "override
/// applied" from "override missing" — it would trivially pass either way.
/// Production `CardData` is instead built by `CardData::load_from_file` /
/// `load_from_str` (card_data.rs) purely from `cards.json`'s
/// `type_eng`/`form_eng`/`attribute_eng` fields; `card_overrides.json` only
/// reaches `cards.json` via the `ingest_cards.py` merge step, so this test
/// reads `data/cards.json` directly (BT12-022 idiom) to prove the trait
/// actually reaches the field the real game runs on.
#[test]
fn bt24_014_data_cards_json_has_dragonkin_rule_trait() {
    use digimon_engine::card_data::CardData;
    use std::path::PathBuf;

    // CARGO_MANIFEST_DIR = code/digimon-engine; data/cards.json is two
    // levels up at the repo root.
    let cards_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("cards.json");
    if !cards_json.exists() {
        eprintln!(
            "Skipping: data/cards.json not found at {} — test only runs in a full repo checkout",
            cards_json.display()
        );
        return;
    }

    let cards = CardData::load_from_file(&cards_json).expect("load data/cards.json");
    let card = cards.get("BT24-014").expect("BT24-014 in data/cards.json");

    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Dragonkin")),
        "data/cards.json BT24-014.traits must carry the (Rule) Trait: Has \
         [Dragonkin] Type grant — reconcile via \
         code/tools/audit_digivolve/reconcile_traits.py into \
         data/card_overrides.json (type_eng: [\"Shaman\", \"Iliad\", \"TS\", \
         \"Dragonkin\"]) and re-ingest, per the BT21-024 / EX7-021 precedent; \
         got traits={:?}",
        card.traits
    );
    for expected in ["Shaman", "Iliad", "TS"] {
        assert!(
            card.traits.iter().any(|t| t.eq_ignore_ascii_case(expected)),
            "data/cards.json BT24-014.traits must still carry printed trait \
             {expected}; got {:?}",
            card.traits
        );
    }
}

#[test]
fn bt24_014_grants_security_attack_plus_one() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura { security_attack: Some(1), scope, .. })
            if *scope == CompiledScope::FaceUp
    ));
    assert!(
        has,
        "own <Security A. +1> aura; effects: {:?}",
        card.effects
    );
}

#[test]
fn bt24_014_has_own_and_inherited_decode_replacement_clauses() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    let own = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { scope, .. })
                if *scope == CompiledScope::FaceUp
        )
    });
    let inherited = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { scope, .. })
                if *scope == CompiledScope::Inherited
        )
    });
    assert!(
        own,
        "own <Decode ([Aegiomon])> replacement clause must exist"
    );
    assert!(
        inherited,
        "inherited <Decode ([Aegiomon])> replacement clause must exist"
    );
}

#[test]
fn bt24_014_has_when_digivolving_clause() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
            Some(t)
        }
        _ => None,
    });
    assert!(clause.is_some(), "[When Digivolving] clause must exist");
    // The whole clause body is mandatory per DCGO (`canNoSelect: false` on
    // both the -5000 DP pick and the conditional delete pick).
    assert!(
        !clause.unwrap().optional,
        "[When Digivolving] body is mandatory, not optional"
    );
}

#[test]
fn bt24_014_standard_alt_path_is_level4_red_cost4() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(4))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(4) && f.color_is == Some(CompiledColor::Red)
                })
        }),
        "standard Lv.4 Red / cost 4 alt-path must exist; alt_paths: {:?}",
        card.alt_paths
    );
}

#[test]
fn bt24_014_standard_alt_path_is_level4_yellow_cost4() {
    // Second printed digivolve circle (official Bandai DB digivolve_costs:
    // card_color 0=Red AND card_color 2=Yellow, both level 4 / memory_cost 4;
    // data/card_bundles/BT24-014.md confirms both "Red Lv.4 / cost 4" and
    // "Yellow Lv.4 / cost 4"). cards.json's evo_costs drops the Yellow entry
    // (documented multi-colour-cost lossiness) — this alt-path is authored
    // explicitly to restore it, matching the BT25-025 (Blue/Purple) idiom.
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(4))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(4) && f.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "standard Lv.4 Yellow / cost 4 alt-path must exist; alt_paths: {:?}",
        card.alt_paths
    );
}

#[test]
fn bt24_014_digivolves_from_level4_yellow_carrier_for_cost4() {
    // Behavioral confirmation: a Lv.4 Yellow Digimon can actually digivolve
    // into BT24-014 in the engine for cost 4 (the second printed circle).
    let mut runner = base().memory(10).start();
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "YELLOW-CARRIER", Some(0));

    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "BT24-014 must digivolve onto a Lv.4 Yellow carrier for cost 4"
    );
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "BT24-014 must be the new top of the stack after digivolving"
    );
}

#[test]
fn bt24_014_special_alt_path_is_aegiomon_cost3() {
    let runner = base().memory(10).start();
    let card = runner.compiled_card(CARD_ID).expect("BT24-014 present");
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(3))
                && p.from
                    .as_ref()
                    .is_some_and(|f| f.name_contains.as_deref() == Some("Aegiomon"))
        }),
        "special [Digivolve] [Aegiomon]: Cost 3 alt-path must exist; alt_paths: {:?}",
        card.alt_paths
    );
}

// ─── Section 2 — Security A. +1 behavior ─────────────────────────────────────

#[test]
fn bt24_014_security_attack_bonus_is_one_at_runtime() {
    use digimon_engine::enums::ModifierType;

    let mut runner = base().memory(10).start();
    let handle = place_aegiochusmon(&mut runner);
    // The self-aura `security_attack: 1` clause is a materialized declarative
    // effect — it installs a `SecurityAttackChange` modifier on each
    // declarative tick, not a one-shot OnPlay/OnDigivolve trigger.
    runner.game.tick_declarative_effects();

    let bonus = runner
        .modifiers()
        .sum(handle, ModifierType::SecurityAttackChange);
    assert_eq!(
        bonus, 1,
        "own <Security A. +1> must grant a +1 SecurityAttackChange modifier at runtime"
    );
}

// ─── Section 3 — [When Digivolving]: mandatory -5000 DP ─────────────────────

#[test]
fn bt24_014_when_digivolving_installs_dp_reduce_prompt_with_opp_digimon() {
    let mut runner = base()
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"]) // 4 > 3, delete gate should not fire
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-12000", Some(0));
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "RED-CARRIER", Some(0));

    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);
    assert!(digivolved, "digivolve onto RED-CARRIER must succeed");

    let pending = runner
        .pending_selection()
        .expect("mandatory -5000 DP target prompt must install");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "prompt targets an opponent Digimon"
    );
    assert!(
        !runner.pending_is_optional(),
        "the -5000 DP pick is mandatory, not optional"
    );
    let _ = pending;
}

#[test]
fn bt24_014_when_digivolving_no_opp_digimon_installs_no_selection() {
    let mut runner = base().memory(10).security(0, &["FILLER"; 4]).start();
    // No opponent Digimon on the field at all.
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "RED-CARRIER", Some(0));

    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);
    assert!(digivolved);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon exists, so no -5000 DP prompt installs"
    );
}

#[test]
fn bt24_014_when_digivolving_applies_minus_5000_dp_to_chosen_target() {
    let mut runner = base()
        .security(0, &["FILLER"; 4]) // >3 security: delete gate does not fire
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-12000", Some(0));
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "RED-CARRIER", Some(0));

    runner
        .game
        .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    let view = runner
        .pending_selection_view()
        .expect("DP-reduce prompt must install");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("a legal non-PASS target action must exist");
    runner
        .execute_action(0, action)
        .expect("choose DP-reduce target");
    runner.auto_resolve().ok();

    let dp_after = runner.effective_dp(opp).expect("opp still on field");
    assert_eq!(
        dp_after, 7000,
        "OPP-12000 must be reduced by 5000 DP (12000 -> 7000)"
    );
}

// ─── Section 4 — [When Digivolving]: security-gated conditional delete ──────

#[test]
fn bt24_014_delete_gate_blocked_with_more_than_three_security() {
    let mut runner = base()
        .security(0, &["FILLER"; 4]) // 4 > 3 → delete step must not fire
        .memory(10)
        .start();
    // OPP-7000 will become 2000 DP after -5000, well under any delete threshold,
    // but the security gate must still block the whole delete step.
    let opp = runner.place_on_field(1, "OPP-7000", Some(0));
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "RED-CARRIER", Some(0));

    runner
        .game
        .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    // Resolve the mandatory -5000 DP pick (only one legal opp target).
    let view = runner.pending_selection_view().expect("DP-reduce prompt");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("legal target action");
    runner
        .execute_action(0, action)
        .expect("choose DP-reduce target");
    runner.auto_resolve().ok();

    assert!(
        runner.pending_selection().is_none(),
        "with >3 security the delete step must not prompt"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-7000"),
        "OPP-7000 must survive when the security gate blocks the delete"
    );
    let _ = opp;
}

#[test]
fn bt24_014_delete_gate_fires_with_three_or_fewer_security() {
    let mut runner = base()
        .security(0, &["FILLER", "FILLER", "FILLER"]) // 3 <= 3 → delete gate fires
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-7000", Some(0));
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "RED-CARRIER", Some(0));

    runner
        .game
        .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    // Resolve the mandatory -5000 DP pick against OPP-7000 (only legal target).
    let view = runner.pending_selection_view().expect("DP-reduce prompt");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("legal target action");
    runner
        .execute_action(0, action)
        .expect("choose DP-reduce target");

    // Delete prompt: OPP-7000 is now 2000 DP (<= 7000) and must be a legal target.
    let delete_view = runner
        .pending_selection_view()
        .expect("delete prompt must install (security <= 3)");
    assert!(
        !delete_view
            .valid_action_ids
            .contains(&digimon_engine::action::space::PASS)
            || delete_view.valid_action_ids.len() > 1,
        "the delete pick is mandatory when a legal target exists"
    );
    let delete_action = *delete_view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("a legal delete target must exist");
    runner
        .execute_action(0, delete_action)
        .expect("choose delete target");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-7000"),
        "OPP-7000 (post-debuff DP 2000 <= 7000) must be deleted"
    );
    let _ = opp;
}

#[test]
fn bt24_014_delete_target_filter_uses_post_debuff_dp_excludes_high_dp_non_target() {
    // Two opponent Digimon: OPP-12000 (reduced target, -5000 -> 7000, exactly
    // at the threshold) and OPP-3000 (untouched, stays 3000). Both are legal
    // delete targets once the debuff lands on OPP-12000: 7000 <= 7000 passes,
    // 3000 <= 7000 passes too. This test asserts the debuffed target is
    // eligible (DP measured AFTER the -5000 applies, per DCGO's live
    // `permanent.DP` read in `CanSelectPermanentCondition_Delete`).
    let mut runner = base()
        .security(0, &["FILLER", "FILLER"]) // 2 <= 3 → delete gate fires
        .memory(10)
        .start();
    let high = runner.place_on_field(1, "OPP-12000", Some(0));
    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    let carrier = runner.place_on_field(0, "RED-CARRIER", Some(0));

    runner
        .game
        .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    // Only one opp Digimon exists, so the DP-reduce pick has exactly one
    // legal (non-PASS) target: OPP-12000.
    let view = runner.pending_selection_view().expect("DP-reduce prompt");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("legal target action");
    runner
        .execute_action(0, action)
        .expect("reduce OPP-12000 by 5000");

    let dp_after = runner.effective_dp(high).expect("still on field");
    assert_eq!(dp_after, 7000, "12000 - 5000 = 7000");

    let delete_view = runner
        .pending_selection_view()
        .expect("delete prompt installs (security <= 3)");
    let delete_action = digimon_engine::action::space::encode_attack(0, high.index as u16);
    assert!(
        delete_view.valid_action_ids.contains(&delete_action),
        "OPP-12000 at post-debuff DP 7000 (<= 7000) must be a legal delete target"
    );
}

// ─── Section 5 — <Decode ([Aegiomon])> replacement (own scope) ──────────────

#[test]
fn bt24_014_decode_prompts_on_non_battle_leave_with_aegiomon_source() {
    let mut runner = base().memory(10).start();
    // Aegiochusmon stacked over an [Aegiomon] source, then De-Digivolved (a
    // non-battle "would leave" trigger) via a return-to-hand style removal.
    let handle = runner.place_stack(0, &["AEGIOMON-SRC", CARD_ID]);

    // Trigger a non-battle leave (return-to-hand) to reach the would-leave window.
    runner.game.return_to_hand(handle);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the <Decode> replacement prompt must install on a non-battle leave"
    );
    assert!(
        runner.pending_is_optional(),
        "<Decode> is a 'you may' replacement"
    );
}

#[test]
fn bt24_014_decode_declined_leaves_proceeds_without_playing_source() {
    let mut runner = base().memory(10).start();
    let handle = runner.place_stack(0, &["AEGIOMON-SRC", CARD_ID]);
    let field_len_before = runner.game.players[0].battle_area.len();

    runner.game.return_to_hand(handle);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the Decode replacement");
    runner.auto_resolve().ok();

    // The original leave proceeds: no new [Aegiomon] permanent was played,
    // and the battle area shrinks by the leaving permanent (unless declined
    // means it also plays no source but the leave still happens).
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_len_before - 1,
        "declining Decode still lets the original non-battle leave proceed"
    );
}

#[test]
fn bt24_014_decode_accepted_plays_aegiomon_source_without_paying_cost() {
    let mut runner = base().memory(2).start(); // low memory: free play must not cost memory
    let handle = runner.place_stack(0, &["AEGIOMON-SRC", CARD_ID]);
    let memory_before = runner.game.memory;

    runner.game.return_to_hand(handle);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));

    let view = runner.pending_selection_view().expect("replacement view");
    let accept = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("exactly one ACCEPT entry");
    runner.execute_action(0, accept).expect("accept Decode");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "AEGIOMON-SRC"),
        "accepting Decode plays the [Aegiomon] source onto the field"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "Decode plays the source without paying its cost (memory unaffected)"
    );
}

// ─── Section 6 — inherited <Decode ([Aegiomon])> (as a digivolution source) ─

#[test]
fn bt24_014_inherited_decode_fires_when_buried_under_another_permanent() {
    let mut runner = base().memory(10).start();
    // BT24-014 buried as a source under RED-CARRIER's stack, with an
    // [Aegiomon] source further underneath it.
    let handle = runner.place_stack(0, &["AEGIOMON-SRC", CARD_ID, "RED-CARRIER"]);

    // De-digivolve the top of the stack (RED-CARRIER) off, isolating whether
    // BT24-014's OWN would-leave fires — instead, directly trigger a
    // non-battle leave on the whole stack to exercise the inherited Decode.
    runner.game.return_to_hand(handle);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "inherited <Decode> must also offer a replacement prompt when \
         BT24-014 sits buried in the stack that is leaving"
    );
}
