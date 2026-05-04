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
//!   The explicit place-step is required here because Clause 2 is BLOCKED and
//!   not authored as `kind: delay`, so `classify_option_subtype` would treat
//!   this card as Standard (not Delay) and would NOT auto-seat it.
//!
//! - **Inherited [Security] same-shape body + `place_self_as_delay_option`** —
//!   `scope: inherited` + `when: on_security` mirrors BT22-099 Clause 3 with
//!   the BT17-095 placement tail.
//!
//! # Known gaps (BLOCKED clauses)
//!
//! - **G-DSL-ON-ALLY-ATTACK-TIMING** (existing — BT21-102, 2026-05-03):
//!   `digimon_dsl::clause::Timing` has no `OnAllyAttack` / `OnOpponentAttack`
//!   variants. The engine has the dispatch (`combat.rs §Phase 9 Task 8`) and
//!   `Effect::on_ally_attack`, but no YAML token reaches them.
//!
//! - **G-ATK-TRAIT-FILTER** (existing — BT21-025, 2026-04-27): no
//!   `attacker_trait_has` BoolPredicate to gate observers on the attacker's
//!   traits. `TriggerContext.source_permanent` carries the attacker handle for
//!   `PlayerBattleArea` fan-outs, but the DSL has no predicate leaf to read it.
//!
//! - **G-DSL-DELAY-ON-ATTACK-EVENT** (NEW — this card): `kind: delay` lowering
//!   in `code/digimon-engine/src/dsl_cards/lower_delay.rs:55-65` only maps
//!   `EndOfYourTurn` / `StartOfYourTurn` / `EndOfYourNextTurn` / `OnSuspend` /
//!   `OnUnsuspend` to specific `DelayTrigger` variants — every other timing
//!   silently degrades to `EndOfYourNextTurn`. Even if G-DSL-ON-ALLY-ATTACK-
//!   TIMING closed, a `kind: delay` with `trigger: on_ally_attack` would be
//!   silently downgraded. Closure path mirrors the OnSuspend slice landed for
//!   BT22-098: extend the lowering arm to map `OnAttack`/`OnAllyAttack`/
//!   `OnOpponentAttack` to `DelayTrigger::OnEvent(EffectTiming::*)`.
//!
//! See `code/digimon-engine/cards/bt23/BT23-096.yaml` header for the full gap
//! analysis; the [Your Turn] Delay clause is omitted entirely until the gap
//! stack closes. The behavioral test for that clause is `#[ignore]`'d.
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
//! 2. **[Your Turn] CS-attack <Delay>** — BLOCKED (see header).
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
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT23-096";
const YAML: &str = include_str!("../../../cards/bt23/BT23-096.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card — no traits, default Digimon shape.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon-kind card with the CS trait — for color-bypass gate (Clause 0)
/// and as an attacker filter target (Clause 2 — BLOCKED test).
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

/// Three clauses total (Clause 2 is BLOCKED and OMITTED):
///   [0] flood_gate (declarative, IgnoreColorRequirement)
///   [1] main_from_hand (triggered)
///   [2] inherited on_security (triggered, scope: Inherited)
#[test]
fn bt23_096_has_three_clauses_in_expected_order() {
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
        3,
        "expected 3 clauses (flood_gate, main_from_hand, inherited on_security); \
         [Your Turn] Delay clause is BLOCKED and omitted; got {}",
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
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier, ..
        }) => modifier.eq_ignore_ascii_case("IgnoreColorRequirement"),
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
        other => panic!("clause 1 must be Triggered(main_from_hand); got {:?}", other),
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

/// Clause 2 (inherited on_security): inherited scope, OnSecurity timing,
/// process contains the same de_digivolve + place_self body. Mandatory
/// (DCGO has no canNoSelect on PlaceDelayOptionCards security path).
#[test]
fn bt23_096_clause_2_inherited_security_dedigivolve_and_place_self() {
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
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 2 must have Inherited scope"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 2 must fire at OnSecurity; got {:?}",
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
                "clause 2 process must contain a DeDigivolve(amount=4) step; got {:?}",
                t.process
            );
            let has_place_step = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption));
            assert!(
                has_place_step,
                "clause 2 process must contain PlaceSelfAsDelayOption; got {:?}",
                t.process
            );
        }
        other => panic!(
            "clause 2 must be Triggered(inherited on_security); got {:?}",
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

    let sec_proc_dbg = match &card.effects[2] {
        CompiledClause::Triggered(t) => format!("{:?}", t.process),
        _ => panic!("clause 2 must be Triggered"),
    };

    // Both bodies must mention the same key step shapes (select-opp + de-digi-4
    // + place-self). We don't require byte-for-byte equality (prompts may
    // differ), only structural parity in the three load-bearing steps.
    for marker in [
        "DeDigivolve",
        "amount: Some(4)",
        "PlaceSelfAsDelayOption",
    ] {
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
// Section 4 — BLOCKED: Clause 2 [Your Turn] CS-attack <Delay>
// ═══════════════════════════════════════════════════════════════════════════════

/// BLOCKED: this clause requires three gap closures (G-DSL-ON-ALLY-ATTACK-
/// TIMING + G-ATK-TRAIT-FILTER + G-DSL-DELAY-ON-ATTACK-EVENT). See the YAML
/// header for the full analysis. When the gaps close, this test should:
///   1. Place a CS-trait Digimon on player 0's field.
///   2. Activate BT23-096 from hand (Main clause) — picks any opp digimon to
///      de-digivolve, then BT23-096 lands as a Delay-Option permanent.
///   3. Advance one turn (Delay activation gate: "after the placing turn").
///   4. Have the CS Digimon attack — observe that the Delay activates,
///      prompting to trash BT23-096 + de_digivolve another opp Digimon.
///   5. Verify (a) BT23-096 is trashed, (b) the new target's stack lost up
///      to 4 cards (capped at level 3).
///   6. Negative: have a NON-CS ally attack — verify the Delay does NOT
///      activate (gap-half: G-ATK-TRAIT-FILTER must reject non-CS attackers).
#[test]
#[ignore = "pending: G-DSL-ON-ALLY-ATTACK-TIMING + G-ATK-TRAIT-FILTER + G-DSL-DELAY-ON-ATTACK-EVENT — \
            DSL Timing has no OnAllyAttack variant, no attacker_trait_has predicate, and \
            kind: delay lowering doesn't map on_attack timings to DelayTrigger::OnEvent. \
            See qa/dsl-vocab-gaps.md and BT23-096.yaml header."]
fn bt23_096_your_turn_cs_attack_delay_dedigi4_BLOCKED() {
    unimplemented!(
        "Three gap closures required before this clause is implementable. See yaml header."
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
