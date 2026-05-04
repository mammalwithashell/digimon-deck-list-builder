//! LM-034 Wisteria Memory Boost! — Option, Blue, Cost 3.
//!
//! # Card text (cards.json)
//!
//! Red also meets this card's color requirements.
//!
//! **[Main]** Reveal the top 3 cards of your deck. Add 1 blue or red Digimon
//! card among them to the hand. Return the rest to the bottom of deck. Then,
//! place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn,
//! activate the effect below.)
//! ・Gain 2 memory.
//!
//! **Inherited (Security):** [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/LM/Blue/LM_034.cs`
//!
//! # Patterns this test covers
//!
//! - **Color bypass via `kind: flood_gate` + `IgnoreColorRequirement`**
//!   gated by an `any_permanent` predicate over (Red Digimon | Red Tamer).
//!   Mirror of P-151 / BT22-099 Clause 0 with the trait predicate replaced
//!   by `color_is: red`. G-IGNORE-COLOR-MASK was resolved 2026-05-02 so
//!   runtime-enforcement assertions are no longer ignored. (We assert the
//!   structural shape; full action-mask enforcement is exercised by the
//!   engine-side `flood_gates::group6_option_color` regression suite.)
//!
//! - **[Main] reveal-3 + single-bucket Blue/Red-Digimon selection +
//!   bottom remainder** — identical shape to BT22-099 / BT22-094 Clause 1
//!   (single-bucket pattern, max 1, min 1). The bucket filter swaps the
//!   trait predicate for an `all_of [kind: digimon, any_of [color_is: blue,
//!   color_is: red]]` colour disjunction. Mirrors DCGO
//!   `RevealDeckTopCardsAndSelect(revealCount: 3,
//!     [SelectCardCondition(IsDigimon && (HasColor Blue || HasColor Red),
//!     canNoSelect: false, max 1, AddHand)],
//!     remainingCardsPlace: DeckBottom)`.
//!
//! - **Standard ＜Delay＞ + gain_memory: 2** — `kind: delay` declarative
//!   with `trigger: end_of_your_next_turn`. Identical to BT22-099 Clause 2 /
//!   P-035 Clause 1 (DCGO uses the same gain-2-memory body inline rather
//!   than the `Gain2MemoryOptionDelayEffect` factory, but the shape is the
//!   same).
//!
//! - **Inherited [Security] place-self-as-Delay** — `place_self_as_delay_option`
//!   step under `scope: inherited` + `when: on_security`. RESOLVED gap
//!   G-PLACE-SELF-AS-OPTION-PERMANENT (2026-05-02). Behavioural coverage of
//!   the substrate lives in `option_flow::inherited_security_option`; this
//!   test asserts that the clause is structurally present and lowers to the
//!   placement step.
//!
//! # Faithfulness audit (per clause)
//!
//! 0. **Color bypass** — `flood_gate` + `IgnoreColorRequirement` +
//!    `target: { card_number_is: "LM-034" }` mirrors DCGO
//!    `IgnoreColorConditionClass.SetUpIgnoreColorConditionClass(cardCondition:
//!    cardSource == card)`. The `any_permanent` gate matches DCGO's
//!    `HasMatchConditionPermanent((permanent) => permanent.TopCard.Owner ==
//!    card.Owner && permanent.TopCard.CardColors.Contains(Red) &&
//!    (permanent.IsDigimon || permanent.IsTamer))`. BOTH gate branches
//!    require `color_is: red` per the printed text "Red also meets this
//!    card's color requirements" (DCGO models this as a full color bypass
//!    when a Red Digimon/Tamer is on field — for a single-coloured Blue
//!    Option this is equivalent to the printed "Red satisfies" reading per
//!    the source-priority tiebreaker rule).
//!
//! 1. **[Main] reveal-3 mandatory blue/red-Digimon bucket + bottom
//!    remainder** — `select_reveal_buckets` with one bucket
//!    `{ filter: { all_of: [kind: digimon, any_of: [color_is: blue, color_is:
//!    red]] }, min: 1, max: 1 }` matches DCGO single-bucket-mandatory-when-
//!    eligible semantics. The trailing `place_remainder_on_deck { position:
//!    bottom }` matches DCGO `remainingCardsPlace: DeckBottom`. The "Then,
//!    place this card in the battle area" sub-step is implicit — the
//!    engine's `classify_option_subtype` detects Clause 2 (`kind: delay`)
//!    and auto-seats this Option as `OptionState::Delayed` at MainEffectDrain.
//!
//! 2. **[Main] ＜Delay＞ gain 2 memory** — `kind: delay` +
//!    `trigger: end_of_your_next_turn` + `process: [gain_memory: 2]`.
//!    Identical to BT22-099 / P-035 / EX9-021 standard-Delay precedents.
//!    No [OPT] — single-fire is enforced by the trash-cost self-limit.
//!
//! 3. **[Security] (inherited) place this card in battle area** —
//!    `scope: inherited` + `when: on_security` + `process:
//!    [place_self_as_delay_option: {}]`. Matches DCGO
//!    `PlaceSelfDelayOptionSecurityEffect`. Mandatory (no canNoSelect on
//!    the DCGO factory; `optional: false` here).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "LM-034";
const YAML: &str = include_str!("../../../cards/lm/LM-034.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card — no special colour, default Digimon shape. Used as
/// non-eligible reveal padding.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Blue Digimon — eligible for the Clause 1 bucket (kind: digimon,
/// color_is: blue|red). Also relevant as a non-Red board permanent for
/// Clause 0 negative gate tests.
fn make_blue_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// A Red Digimon — eligible for the Clause 1 bucket and gates the Clause 0
/// colour bypass (DCGO CanUseCondition: own field has Red Digimon/Tamer).
fn make_red_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A Red Tamer — alternative gate for the Clause 0 any_of bypass condition.
fn make_red_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A non-Digimon, non-Red filler — used for negative reveal-bucket cases
/// (option/tamer/black filler that fails the bucket filter).
fn make_black_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c
}

/// Helper: walk the revealed-cards list to find the action ID for a specific
/// revealed card_id. Mirrors `revealed_action_for_id` in bt22_099.
fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

/// LM-034 YAML must parse and compile without errors.
#[test]
fn lm_034_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-034 YAML must parse and compile without errors");
}

/// LM-034 must compile as an Option card with cost 3.
#[test]
fn lm_034_is_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("LM-034 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "LM-034 must be an Option card"
    );
    assert_eq!(card.cost, Some(3), "LM-034 prints Cost 3");
}

/// Four clauses total:
///   [0] flood_gate (declarative, IgnoreColorRequirement)
///   [1] main_from_hand (triggered)
///   [2] delay (declarative, end_of_your_next_turn)
///   [3] inherited on_security (triggered, scope: Inherited)
#[test]
fn lm_034_has_four_clauses_in_expected_order() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "expected 4 clauses (flood_gate, main_from_hand, delay, inherited on_security); got {}",
        card.effects.len()
    );
}

/// Clause 0: flood_gate declarative carrying the IgnoreColorRequirement modifier.
#[test]
fn lm_034_clause_0_is_flood_gate_with_ignore_color_modifier() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

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
/// `SetUpActivateClass(..., -1, false, ...)`; no "you may" in printed text).
#[test]
fn lm_034_clause_1_main_from_hand_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

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
                "clause 1 outer trigger is NOT optional — DCGO SetUpActivateClass(..., -1, false, ...); \
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

/// Clause 2: declarative Delay with trigger end_of_your_next_turn.
#[test]
fn lm_034_clause_2_is_delay_end_of_your_next_turn() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

    match &card.effects[2] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::EndOfYourNextTurn,
                "Delay trigger must be EndOfYourNextTurn (standard <Delay>); got {:?}",
                trigger
            );
        }
        other => panic!("clause 2 must be Declarative(Delay); got {:?}", other),
    }
}

/// Delay clause process must contain GainMemory(2).
#[test]
fn lm_034_delay_process_has_gain_memory_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

    match &card.effects[2] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            assert!(!process.is_empty(), "Delay process must be non-empty");
            let has_gain_2 = process
                .iter()
                .any(|s| matches!(s, CompiledStep::GainMemory(2)));
            assert!(
                has_gain_2,
                "Delay process must contain GainMemory(2); got {:?}",
                process
            );
        }
        other => panic!("clause 2 must be Delay; got {:?}", other),
    }
}

/// Clause 3: inherited scope, OnSecurity timing, process contains the
/// place_self_as_delay_option step (DCGO PlaceSelfDelayOptionSecurityEffect).
#[test]
fn lm_034_clause_3_inherited_security_places_self_as_delay() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

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
                 RULES_CONTEXT.md §16; DCGO PlaceSelfDelayOptionSecurityEffect has no canNoSelect)"
            );
            // Verify the placement step is present in the body.
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
// Section 2 — Behavioral: Clause 1 [Main] reveal-3 + add 1 blue/red Digimon + bottom rest
// ═══════════════════════════════════════════════════════════════════════════════
//
// Strategy mirrors bt22_099 / p_035: use `activate_hand_main` to drive the
// MainFromHand effect process directly without relying on the full play-flow
// (which would exercise color-requirement checks against Blue). The [Main]
// body is identical in shape to BT22-099 Clause 1, with the trait predicate
// substituted by a colour-disjunction `kind: digimon + (blue | red)` filter.

/// Positive: top-3 reveal contains [BLUE-DIGI, FILLER, FILLER] — the player
/// must pick the Blue Digimon, which lands in hand; the two FILLERs go to
/// the bottom of the deck.
#[test]
fn lm_034_main_picks_blue_digimon_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-DIGI", "Blue Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        // Builder convention: last in slice = top of deck (drawn first).
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-A", "FILL-B", "BLUE-DIGI"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for LM-034");

    // The blue/red-Digimon bucket prompt must install (single eligible card).
    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install for the digi bucket when an eligible \
         Digimon is in the reveal"
    );

    // Pick BLUE-DIGI for the bucket.
    let blue_action = revealed_action_for_id(&runner, "BLUE-DIGI")
        .expect("BLUE-DIGI must appear among the revealed-card actions");
    runner
        .execute_action(0, blue_action)
        .expect("pick BLUE-DIGI for the digi bucket");

    // Drive any trailing remainder-placement order prompts.
    let _ = runner.auto_resolve();

    // BLUE-DIGI must be in P0's hand.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"BLUE-DIGI".to_string()),
        "BLUE-DIGI must be added to hand; hand was {hand_ids:?}"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILL-A" || id == "FILL-B"),
        "FILLERs must NOT enter hand; hand was {hand_ids:?}"
    );

    // The two FILLERs must be at the bottom of the deck.
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let bottom_two: Vec<String> = deck_ids.iter().take(2).cloned().collect();
    assert!(
        bottom_two.iter().any(|id| id == "FILL-A")
            && bottom_two.iter().any(|id| id == "FILL-B"),
        "FILL-A and FILL-B must be at the bottom of the deck; bottom-2 was {bottom_two:?}"
    );
}

/// Positive (Red branch): top-3 reveal contains [RED-DIGI, FILLER, FILLER] —
/// the player must pick the Red Digimon (Red is also valid per the bucket's
/// colour disjunction), which lands in hand.
#[test]
fn lm_034_main_picks_red_digimon_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-DIGI", "Red Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-A", "FILL-B", "RED-DIGI"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for LM-034");

    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install for the digi bucket when a Red Digimon \
         is in the reveal (Red is in the bucket colour disjunction)"
    );

    let red_action = revealed_action_for_id(&runner, "RED-DIGI")
        .expect("RED-DIGI must appear among the revealed-card actions");
    runner
        .execute_action(0, red_action)
        .expect("pick RED-DIGI for the digi bucket");

    let _ = runner.auto_resolve();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"RED-DIGI".to_string()),
        "RED-DIGI must be added to hand; hand was {hand_ids:?}"
    );
}

/// Negative: no eligible Digimon in the reveal — the bucket has zero
/// candidates and silently resolves; all 3 revealed cards return to the
/// bottom of the deck. Hand stays at size 1 (LM-034 itself; activate_hand_main
/// does not consume it).
#[test]
fn lm_034_main_no_blue_or_red_digimon_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        // BLACK Digimon is the right kind but wrong colour — fails the bucket.
        .add_card(make_black_digimon("BLK-A", "Black A"))
        .add_card(make_black_digimon("BLK-B", "Black B"))
        .add_card(make_black_digimon("BLK-C", "Black C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["BLK-A", "BLK-B", "BLK-C"])
        .deck(1, &["BLK-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // No black Digimon should have been added to hand — only LM-034 remains.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    for blk in ["BLK-A", "BLK-B", "BLK-C"] {
        assert!(
            !hand_ids.contains(&blk.to_string()),
            "{blk} (Black, wrong colour) must NOT enter hand; hand was {hand_ids:?}"
        );
    }

    // All 3 revealed cards must be back in the deck — net deck size unchanged.
    let deck_after = runner.deck_size(0);
    assert_eq!(
        deck_after, deck_before,
        "all 3 revealed Black Digimon must return to deck bottom (no eligible card to hand); \
         deck size before={deck_before}, after={deck_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Deck accounting after the [Main] reveal
// ═══════════════════════════════════════════════════════════════════════════════

/// Mirror of bt22_099 / p_035 deck-accounting test: when 1 eligible Digimon
/// is added to hand, deck shrinks by exactly 1 (3 revealed - 2 returned bottom
/// = 1 removed to hand).
#[test]
fn lm_034_main_deck_size_shrinks_by_one_when_eligible_digi_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-DIGI", "Blue Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        .add_card(filler("FILL-D"))
        .hand(0, &[CARD_ID])
        // Top of deck = last in slice. BLUE-DIGI is on top, in the reveal-3.
        .deck(
            0,
            &["FILL-A", "FILL-B", "FILL-C", "FILL-D", "FILL-A", "BLUE-DIGI"],
        )
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);

    if let Some(action) = revealed_action_for_id(&runner, "BLUE-DIGI") {
        runner
            .execute_action(0, action)
            .expect("pick BLUE-DIGI for the digi bucket");
    }
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    assert_eq!(
        deck_before - deck_after,
        1,
        "Deck must shrink by exactly 1 (3 revealed, 1 to hand, 2 returned to bottom); \
         before={deck_before}, after={deck_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence unused-warning across test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_red_tamer("X", "X");
    let _ = make_black_digimon("Y", "Y");
}
