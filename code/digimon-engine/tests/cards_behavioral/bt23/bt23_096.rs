//! BT23-096 Comet Hammer — Option, Black, Cost 5, traits: [CS].
//!
//! # Card text (cards.json)
//!
//! While you have a Digimon or Tamer with the [CS] trait on the field, you can
//! ignore this card's color requirements.
//!
//! **[Main]** ＜De-Digivolve 4＞ 1 of your opponent's Digimon. (Trash up to 4
//! cards from the top. You can't trash past level 3 cards.) Then, place this
//! card in the battle area.
//!
//! **[Your Turn]** When one of your [CS] trait Digimon attacks, ＜Delay＞ (By
//! trashing this card after the placing turn, activate the effect below.)
//! ・＜De-Digivolve 4＞ 1 of your opponent's Digimon.
//!
//! **Inherited (Security):** [Security] ＜De-Digivolve 4＞ 1 of your opponent's
//! Digimon. Then, place this card in the battle area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT23/Black/BT23_096.cs`
//!
//! # Patterns this test covers
//!
//! - **Color bypass via `kind: flood_gate` + `IgnoreColorRequirement`** gated
//!   by an `any_permanent` predicate over (CS Digimon | CS Tamer) — same shape
//!   as BT22-099 Clause 0. G-IGNORE-COLOR-MASK was resolved 2026-05-02.
//!
//! - **[Main] mandatory `select_opponent_permanent` + `de_digivolve` (amount 4,
//!   stop_at_level 3) + `place_self_as_delay_option`** — combines the EX9-013
//!   `de_digivolve` shape with the BT17-095 `place_self_as_delay_option` tail.
//!   The explicit place-step matches the printed "Then, place this card in the
//!   battle area" after the [Main] body resolves.
//!
//! - **[Your Turn] CS-attack `<Delay>`** — `kind: delay`, `trigger:
//!   on_ally_attack`, and `attacker_trait_has: CS`. This covers the
//!   event-gated Delay lowering, attack-event fan-out, and attacker context.
//!
//! - **Inherited [Security] same-shape body + `place_self_as_delay_option`** —
//!   `scope: inherited` + `when: on_security` mirrors BT22-099 Clause 3 with
//!   the BT17-095 placement tail.
//!
//! # Faithfulness audit (per clause)
//!
//! 0. **Color bypass** — `flood_gate` + `IgnoreColorRequirement` +
//!    `target: { card_number_is: "BT23-096" }` mirrors DCGO
//!    `IgnoreColorConditionClass.SetUpIgnoreColorConditionClass(cardCondition:
//!    cardSource == card)`. The `any_permanent` gate matches DCGO's
//!    `HasMatchConditionPermanent((permanent) => permanent.TopCard.Owner == card.Owner
//!    && (permanent.IsTamer || permanent.IsDigimon) && permanent.TopCard.HasCSTraits)`.
//!
//! 1. **[Main] mandatory <De-Digivolve 4> + place self** — outer trigger is
//!    NOT optional (DCGO `SetUpActivateClass(..., false, ...)`; no "you may"
//!    in printed text). Outer condition gates on opponent having any Digimon
//!    (silent no-op when none). `de_digivolve` amount: 4 + stop_at_level: 3
//!    matches DCGO `IDegeneration(selectedPermanent, 4, ...)`. Trailing
//!    `place_self_as_delay_option: {}` matches DCGO
//!    `CardEffectCommons.PlaceDelayOptionCards(card, activateClass)`.
//!
//! 2. **[Your Turn] CS-attack <Delay>** — event-gated Delay lowers to
//!    `DelayTrigger::OnEvent(OnAllyAttack)`, rejects non-CS attackers, and
//!    trashes BT23-096 before the de-digivolve body resolves.
//!
//! 3. **[Security] (inherited) same body** — `scope: inherited` + `when:
//!    on_security` + same select/de-digi/place body. Mandatory (no canNoSelect
//!    on DCGO `SetIsSecurityEffect(true)` factory; `optional: false` here).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT23-096";
const YAML: &str = include_str!("../../../cards/bt23/BT23-096.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card — no traits, default Digimon shape.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon-kind card with the CS trait — for color-bypass gate (Clause 0)
/// and as an attacker filter target (Clause 2).
fn make_cs_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["CS".to_string()];
    c
}

/// A Tamer-kind card with the CS trait — alternative color-bypass gate.
fn make_cs_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["CS".to_string()];
    c
}

/// An opponent-side Digimon with a level-5 base + extra digivolution materials
/// for de_digivolve testing. Used as the [Main] / [Security] target.
fn make_opp_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Red];
    c
}

fn make_level_digimon(card_id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Red];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

/// BT23-096 YAML must parse and compile without errors.
#[test]
fn bt23_096_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT23-096 YAML must parse and compile without errors");
}

/// BT23-096 must compile as an Option card with cost 5 and the CS trait.
#[test]
fn bt23_096_is_option_cost_5_with_cs_trait() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT23-096 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "BT23-096 must be an Option card"
    );
    assert_eq!(card.cost, Some(5), "BT23-096 prints Cost 5");
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("CS")),
        "BT23-096 must carry the CS trait (printed type_eng = CS)"
    );
}

/// Four clauses total:
///   [0] flood_gate (declarative, IgnoreColorRequirement)
///   [1] main_from_hand (triggered)
///   [2] event-gated Delay on CS ally attack
///   [3] inherited on_security (triggered, scope: Inherited)
#[test]
fn bt23_096_has_four_clauses_in_expected_order() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "expected 4 clauses (flood_gate, main_from_hand, Delay on ally attack, inherited on_security); got {}",
        card.effects.len()
    );
}

/// Clause 0: flood_gate declarative carrying the IgnoreColorRequirement modifier.
#[test]
fn bt23_096_clause_0_is_flood_gate_with_ignore_color_modifier() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    let is_flood_gate_with_modifier = match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. }) => {
            modifier.eq_ignore_ascii_case("IgnoreColorRequirement")
        }
        _ => false,
    };
    assert!(
        is_flood_gate_with_modifier,
        "clause 0 must be a flood_gate carrying IgnoreColorRequirement; got {:?}",
        card.effects[0]
    );
}

/// Clause 1: main_from_hand triggered, FaceUp scope, NOT optional (DCGO
/// `SetUpActivateClass(..., false, ...)`; no "you may" in printed text).
#[test]
fn bt23_096_clause_1_main_from_hand_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    match &card.effects[1] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 1 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "clause 1 must have FaceUp scope"
            );
            assert!(
                !t.optional,
                "clause 1 outer trigger is NOT optional — DCGO SetUpActivateClass(..., false, ...); \
                 no 'you may' in printed text"
            );
            assert!(
                !t.once_per_turn,
                "clause 1 carries no [Once Per Turn] in printed text"
            );
        }
        other => panic!(
            "clause 1 must be Triggered(main_from_hand); got {:?}",
            other
        ),
    }
}

/// Clause 1's process must contain a DeDigivolve step with amount=4, stop_at_level=3.
#[test]
fn bt23_096_clause_1_process_has_dedigivolve_4_stop_at_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    match &card.effects[1] {
        CompiledClause::Triggered(t) => {
            // Look for a DeDigivolve step with the right shape.
            let has_de_digivolve_4 = t.process.iter().any(|s| {
                let dbg = format!("{:?}", s);
                dbg.contains("DeDigivolve")
                    && (dbg.contains("amount: Some(4)") || dbg.contains("amount: 4"))
                    && (dbg.contains("stop_at_level: Some(3)") || dbg.contains("stop_at_level: 3"))
            });
            assert!(
                has_de_digivolve_4,
                "clause 1 process must contain a DeDigivolve(amount=4, stop_at_level=3) step; \
                 got {:?}",
                t.process
            );
        }
        other => panic!("clause 1 must be Triggered; got {:?}", other),
    }
}

/// Clause 1's process must end with PlaceSelfAsDelayOption (the explicit
/// "Then, place this card in the battle area" tail).
#[test]
fn bt23_096_clause_1_process_ends_with_place_self_as_delay_option() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    match &card.effects[1] {
        CompiledClause::Triggered(t) => {
            let has_place_step = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption));
            assert!(
                has_place_step,
                "clause 1 process must contain PlaceSelfAsDelayOption (the printed 'Then, \
                 place this card in the battle area' tail); got {:?}",
                t.process
            );
        }
        other => panic!("clause 1 must be Triggered; got {:?}", other),
    }
}

/// Clause 2: event-gated Delay on allied CS attack.
#[test]
fn bt23_096_clause_2_is_on_ally_attack_delay() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    match &card.effects[2] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger,
            active_when,
            process,
            ..
        }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::OnAllyAttack,
                "Clause 2 must be a Delay keyed to on_ally_attack"
            );
            let active_when = active_when
                .as_ref()
                .expect("Delay clause must gate on [Your Turn] and CS attacker");
            let active_dbg = format!("{:?}", active_when);
            assert!(
                active_dbg.contains("your_turn: Some(true)")
                    && active_dbg.contains("attacker_trait_has: Some(\"CS\")"),
                "Delay active_when must require your_turn and attacker_trait_has: CS; got {active_dbg}"
            );
            let process_dbg = format!("{:?}", process);
            assert!(
                process_dbg.contains("DeDigivolve")
                    && (process_dbg.contains("amount: Some(4)")
                        || process_dbg.contains("amount: 4")),
                "Delay process must perform <De-Digivolve 4>; got {process_dbg}"
            );
        }
        other => panic!("clause 2 must be Declarative(Delay); got {:?}", other),
    }
}

/// Clause 3 (inherited on_security): inherited scope, OnSecurity timing,
/// process contains the same de_digivolve + place_self body. Mandatory
/// (DCGO has no canNoSelect on PlaceDelayOptionCards security path).
#[test]
fn bt23_096_clause_3_inherited_security_dedigivolve_and_place_self() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    match &card.effects[3] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 3 must have Inherited scope"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 3 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert!(
                !t.optional,
                "Security clause must be mandatory ([Security] effects are not opt-out per \
                 RULES_CONTEXT.md §16; DCGO PlaceDelayOptionCards security path has no canNoSelect)"
            );
            // Verify the de_digivolve and placement steps are present.
            let dbg_proc = format!("{:?}", t.process);
            assert!(
                dbg_proc.contains("DeDigivolve")
                    && (dbg_proc.contains("amount: Some(4)") || dbg_proc.contains("amount: 4")),
                "clause 3 process must contain a DeDigivolve(amount=4) step; got {:?}",
                t.process
            );
            let has_place_step = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption));
            assert!(
                has_place_step,
                "clause 3 process must contain PlaceSelfAsDelayOption; got {:?}",
                t.process
            );
        }
        other => panic!(
            "clause 3 must be Triggered(inherited on_security); got {:?}",
            other
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 [Main] de_digivolve target prompt
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative: no opponent Digimon → outer condition blocks the [Main] body;
/// no selection prompt installs.
#[test]
fn bt23_096_main_no_opp_digimon_no_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    // Even if the activation fires, the outer condition gate (any opponent
    // Digimon must exist) must skip the body when none does.
    let _ = fired;
    assert!(
        runner.pending_selection().is_none(),
        "no selection prompt should install when opponent has no Digimon"
    );
}

/// Positive: opponent has a Digimon → MainFromHand activation installs an
/// OppField selection (mandatory, exactly 1 valid target).
#[test]
fn bt23_096_main_opp_digimon_prompts_oppfield_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_opp_digimon("OPP-DIG", "OppDigi"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DIG", None);
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT23-096");

    let kind = runner
        .pending_kind()
        .expect("MainFromHand must install an OppField selection");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "select_opponent_permanent installs OppField selection"
    );

    let view = runner
        .pending_selection_view()
        .expect("selection view must be available");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "exactly 1 opponent Digimon should be a valid target; got {} valid action(s)",
        view.valid_action_ids.len()
    );
    assert!(
        !view.is_optional,
        "[Main] target selection is mandatory — DCGO canNoSelect: false"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 1 + Clause 2 share the same body shape
// ═══════════════════════════════════════════════════════════════════════════════

/// Both [Main] (Clause 1) and [Security] (Clause 2) bodies share the same
/// triplet (select_opponent_permanent + de_digivolve + place_self). This test
/// checks that the [Security] clause's process step shape mirrors [Main].
#[test]
fn bt23_096_security_clause_shares_body_shape_with_main() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT23-096 compiled");

    let main_proc_dbg = match &card.effects[1] {
        CompiledClause::Triggered(t) => format!("{:?}", t.process),
        _ => panic!("clause 1 must be Triggered"),
    };

    let sec_proc_dbg = match &card.effects[3] {
        CompiledClause::Triggered(t) => format!("{:?}", t.process),
        _ => panic!("clause 3 must be Triggered"),
    };

    // Both bodies must mention the same key step shapes (select-opp + de-digi-4
    // + place-self). We don't require byte-for-byte equality (prompts may
    // differ), only structural parity in the three load-bearing steps.
    for marker in ["DeDigivolve", "amount: Some(4)", "PlaceSelfAsDelayOption"] {
        assert!(
            main_proc_dbg.contains(marker),
            "[Main] process must contain {marker:?}; got {main_proc_dbg}"
        );
        assert!(
            sec_proc_dbg.contains(marker),
            "[Security] process must contain {marker:?}; got {sec_proc_dbg}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 2 [Your Turn] CS-attack <Delay>
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt23_096_your_turn_cs_attack_delay_dedigi4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_cs_digimon("CS-ATK", "CS Attacker"))
        .add_card(make_level_digimon("MAIN-L3", "Main Lv3", 3, 3000))
        .add_card(make_level_digimon("MAIN-L4", "Main Lv4", 4, 4000))
        .add_card(make_level_digimon("MAIN-L5", "Main Lv5", 5, 7000))
        .add_card(make_level_digimon("DELAY-L3", "Delay Lv3", 3, 3000))
        .add_card(make_level_digimon("DELAY-L4", "Delay Lv4", 4, 4000))
        .add_card(make_level_digimon("DELAY-L5", "Delay Lv5", 5, 7000))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let main_target = runner.place_stack(1, &["MAIN-L3", "MAIN-L4", "MAIN-L5"]);
    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("resolve [Main] placement");
    runner
        .game
        .delete_permanent_with_cause(main_target, ReplacementCause::OpponentEffect);
    runner.auto_resolve().expect("clear main target deletion");

    let comet = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT23-096 should be placed in battle area as Delay");
    assert!(matches!(
        runner.game.players[0].battle_area[comet].option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyAttack),
            ..
        }
    ));

    runner.game.turn_count += 2;
    let attacker = runner.place_on_field(0, "CS-ATK", Some(0));
    let delay_target = runner.place_stack(1, &["DELAY-L3", "DELAY-L4", "DELAY-L5"]);

    runner.attack_digimon(attacker, delay_target, false);
    runner
        .auto_resolve()
        .expect("resolve Comet Hammer Delay target selection");

    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT23-096 should trash itself as the Delay cost"
    );
    let target = &runner.game.players[1].battle_area[delay_target.index as usize];
    assert_eq!(
        target.stack_size(),
        1,
        "Delay target should be de-digivolved down to its level-3 base"
    );
}

#[test]
fn bt23_096_your_turn_delay_does_not_fire_for_non_cs_attacker() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_level_digimon("NON-CS", "Plain Attacker", 4, 4000))
        .add_card(make_level_digimon("MAIN-L3", "Main Lv3", 3, 3000))
        .add_card(make_level_digimon("MAIN-L4", "Main Lv4", 4, 4000))
        .add_card(make_level_digimon("MAIN-L5", "Main Lv5", 5, 7000))
        .add_card(make_level_digimon("DELAY-L3", "Delay Lv3", 3, 3000))
        .add_card(make_level_digimon("DELAY-L4", "Delay Lv4", 4, 4000))
        .add_card(make_level_digimon("DELAY-L5", "Delay Lv5", 5, 7000))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let main_target = runner.place_stack(1, &["MAIN-L3", "MAIN-L4", "MAIN-L5"]);
    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("resolve [Main] placement");
    runner
        .game
        .delete_permanent_with_cause(main_target, ReplacementCause::OpponentEffect);
    runner.auto_resolve().expect("clear main target deletion");
    runner.game.turn_count += 2;

    let attacker = runner.place_on_field(0, "NON-CS", Some(0));
    let delay_target = runner.place_stack(1, &["DELAY-L3", "DELAY-L4", "DELAY-L5"]);

    runner.attack_digimon(attacker, delay_target, false);
    runner.auto_resolve().expect("resolve attack flow");

    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT23-096 must not trash itself when the attacker lacks the CS trait"
    );
    assert!(
        runner.game.players[0].battle_area.iter().any(|p| p
            .top_card()
            .card_id(&runner.game.card_data)
            == CARD_ID
            && matches!(
                p.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyAttack),
                    ..
                }
            )),
        "BT23-096 must not trash itself when the attacker lacks the CS trait"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Anti-helpers (silence unused-warning across test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_cs_tamer("X", "X");
    let _ = make_cs_digimon("Y", "Y");
}
