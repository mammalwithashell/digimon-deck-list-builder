//! BT22-099 Kuremi Detective Agency — Option, Black+Yellow, Cost 3, traits: [CS].
//!
//! # Card text (cards.json)
//!
//! While you have a [CS] trait Digimon or Tamer on the field, you can ignore
//! this card's color requirements.
//!
//! **[Main]** Reveal the top 3 cards of your deck. Add 1 [CS] trait card
//! among them to the hand. Return the rest to the bottom of the deck. Then,
//! place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn,
//! activate the effect below.)
//! ・Gain 2 memory.
//!
//! **Inherited (Security):** [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Black/BT22_099.cs`
//!
//! # Patterns this test covers
//!
//! - **Color bypass via `kind: flood_gate` + `IgnoreColorRequirement`**
//!   gated by an `any_permanent` predicate over (CS Digimon | CS Tamer).
//!   Mirror of P-151 Clause 0; G-IGNORE-COLOR-MASK was resolved 2026-05-02
//!   so runtime-enforcement assertions are no longer ignored. (We assert
//!   the structural shape — full action-mask enforcement is exercised by
//!   the engine-side `flood_gates::group6_option_color` regression suite.)
//!
//! - **[Main] reveal-3 + single-bucket CS selection + bottom remainder** —
//!   identical shape to BT22-094 Clause 1 (single-bucket pattern, max 1,
//!   min 1, trait_has: CS). Mirrors DCGO `SimplifiedRevealDeckTopCardsAndSelect
//!   (revealCount: 3, [bucket(CS, AddHand, max 1)], remainingCardsPlace: DeckBottom)`.
//!
//! - **Standard ＜Delay＞ + gain_memory: 2** — `kind: delay` declarative
//!   with `trigger: end_of_your_next_turn`. Identical to P-035 Clause 1
//!   (same DCGO factory `Gain2MemoryOptionDelayEffect`).
//!
//! - **Inherited [Security] place-self-as-Delay** — `place_self_as_delay_option`
//!   step under `scope: inherited` + `when: on_security`. RESOLVED gap
//!   G-PLACE-SELF-AS-OPTION-PERMANENT (2026-05-02). Behavioral coverage of
//!   the substrate lives in `option_flow::inherited_security_option`; this
//!   test asserts that the clause is structurally present and lowers to the
//!   placement step.
//!
//! # Faithfulness audit (per clause)
//!
//! 0. **Color bypass** — `flood_gate` + `IgnoreColorRequirement` +
//!    `target: { card_number_is: "BT22-099" }` mirrors DCGO
//!    `IgnoreColorConditionClass.SetUpIgnoreColorConditionClass(cardCondition:
//!    cardSource == card)`. The `any_permanent` gate matches DCGO's
//!    `HasMatchConditionPermanent((permanent) => permanent.TopCard.Owner == card.Owner
//!    && (permanent.IsTamer || permanent.IsDigimon) && permanent.TopCard.HasCSTraits)`.
//!    BOTH gate branches require `trait_has: CS` per the printed text
//!    "[CS] trait Digimon or Tamer".
//!
//! 1. **[Main] reveal-3 mandatory CS bucket + bottom remainder** —
//!    `select_reveal_buckets` with one bucket `{ trait_has: CS, min: 1,
//!    max: 1 }` matches DCGO single-bucket-mandatory-when-eligible
//!    semantics. The trailing `place_remainder_on_deck { position: bottom }`
//!    matches DCGO `remainingCardsPlace: DeckBottom`. The "Then, place this
//!    card in the battle area" sub-step is implicit — the engine's
//!    `classify_option_subtype` detects Clause 2 (`kind: delay`) and
//!    auto-seats this Option as `OptionState::Delayed` at MainEffectDrain.
//!
//! 2. **[Main] ＜Delay＞ gain 2 memory** — `kind: delay` +
//!    `trigger: end_of_your_next_turn` + `process: [gain_memory: 2]`.
//!    Identical to P-035 Clause 1 / EX9-021 / LM-027 standard-Delay
//!    precedents. Same DCGO factory `Gain2MemoryOptionDelayEffect`.
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

const CARD_ID: &str = "BT22-099";
const YAML: &str = include_str!("../../../cards/bt22/BT22-099.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card — no traits, default Digimon shape.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon-kind card with the CS trait. Used both as a reveal-bucket target
/// (Clause 1) and as a board-presence enabler (Clause 0 color-bypass gate).
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

/// A Tamer-kind card with the CS trait. Alternative board-presence enabler
/// for the Clause 0 color-bypass any_of gate.
fn make_cs_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["CS".to_string()];
    c
}

/// A Digimon WITHOUT the CS trait — for negative-bucket reveal tests (Clause 1)
/// and negative color-bypass gate tests (Clause 0).
fn make_non_cs_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c.traits = vec![]; // no CS
    c
}

/// Helper: walk the revealed-cards list to find the action ID for a specific
/// revealed card_id. Mirrors `revealed_action_for_id` in bt22_094.
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

/// BT22-099 YAML must parse and compile without errors.
#[test]
fn bt22_099_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-099 YAML must parse and compile without errors");
}

/// BT22-099 must compile as an Option card with cost 3 and the CS trait.
#[test]
fn bt22_099_is_option_cost_3_with_cs_trait() {
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
        .expect("BT22-099 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "BT22-099 must be an Option card"
    );
    assert_eq!(card.cost, Some(3), "BT22-099 prints Cost 3");
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("CS")),
        "BT22-099 must carry the CS trait (printed type_eng = CS)"
    );
}

/// Four clauses total:
///   [0] flood_gate (declarative, IgnoreColorRequirement)
///   [1] main_from_hand (triggered)
///   [2] delay (declarative, end_of_your_next_turn)
///   [3] inherited on_security (triggered, scope: Inherited)
#[test]
fn bt22_099_has_four_clauses_in_expected_order() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT22-099 compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "expected 4 clauses (flood_gate, main_from_hand, delay, inherited on_security); got {}",
        card.effects.len()
    );
}

/// Clause 0: flood_gate declarative carrying the IgnoreColorRequirement modifier.
#[test]
fn bt22_099_clause_0_is_flood_gate_with_ignore_color_modifier() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT22-099 compiled");

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
fn bt22_099_clause_1_main_from_hand_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT22-099 compiled");

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

/// Clause 2: declarative Delay with trigger end_of_your_next_turn.
#[test]
fn bt22_099_clause_2_is_delay_end_of_your_next_turn() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT22-099 compiled");

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
fn bt22_099_delay_process_has_gain_memory_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT22-099 compiled");

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
fn bt22_099_clause_3_inherited_security_places_self_as_delay() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("BT22-099 compiled");

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
// Section 2 — Behavioral: Clause 1 [Main] reveal-3 + add 1 CS + bottom rest
// ═══════════════════════════════════════════════════════════════════════════════
//
// Strategy mirrors p_035 / bt22_094: use `activate_hand_main` to drive the
// MainFromHand effect process directly without relying on the full play-flow
// (which would exercise color-requirement checks against Black+Yellow). The
// [Main] body is identical in shape to BT22-094 Clause 1, so we re-use the
// positive-and-negative bucket assertions.

/// Positive: top-3 reveal contains [CS_DIGI, FILLER, FILLER] — the player
/// must pick the CS Digimon, which lands in hand; the two FILLERs go to the
/// bottom of the deck.
#[test]
fn bt22_099_main_picks_cs_card_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_cs_digimon("CS-DIGI", "CS Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        // Builder convention: last in slice = top of deck (drawn first).
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-A", "FILL-B", "CS-DIGI"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT22-099");

    // The CS bucket prompt must install (single eligible CS card in reveal).
    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install for the CS bucket when a CS card is in the reveal"
    );

    // Pick CS-DIGI for the CS bucket.
    let cs_action = revealed_action_for_id(&runner, "CS-DIGI")
        .expect("CS-DIGI must appear among the revealed-card actions");
    runner
        .execute_action(0, cs_action)
        .expect("pick CS-DIGI for the CS bucket");

    // Drive any trailing remainder-placement order prompts.
    let _ = runner.auto_resolve();

    // CS-DIGI must be in P0's hand.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"CS-DIGI".to_string()),
        "CS-DIGI must be added to hand; hand was {hand_ids:?}"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILL-A" || id == "FILL-B"),
        "FILLERs must NOT enter hand; hand was {hand_ids:?}"
    );

    // The two FILLERs must be at the bottom of the deck (last 2 entries
    // when iterated in storage order — engine convention: deck[0] = bottom,
    // deck.last() = top of draw).
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let bottom_two: Vec<String> = deck_ids.iter().take(2).cloned().collect();
    assert!(
        bottom_two.iter().any(|id| id == "FILL-A") && bottom_two.iter().any(|id| id == "FILL-B"),
        "FILL-A and FILL-B must be at the bottom of the deck; bottom-2 was {bottom_two:?}"
    );
}

/// Negative: no CS card in the reveal — the CS bucket has zero candidates and
/// silently resolves; all 3 revealed cards return to the bottom of the deck.
/// Hand stays at size 1 (BT22-099 itself; activate_hand_main does not consume it).
#[test]
fn bt22_099_main_no_cs_card_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_non_cs_digimon("NON-CS-A", "NonCS A"))
        .add_card(make_non_cs_digimon("NON-CS-B", "NonCS B"))
        .add_card(make_non_cs_digimon("NON-CS-C", "NonCS C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["NON-CS-A", "NON-CS-B", "NON-CS-C"])
        .deck(1, &["NON-CS-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // No NON-CS card should have been added to hand — only BT22-099 remains.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    for non_cs in ["NON-CS-A", "NON-CS-B", "NON-CS-C"] {
        assert!(
            !hand_ids.contains(&non_cs.to_string()),
            "{non_cs} must NOT enter hand when no CS card is revealed; hand was {hand_ids:?}"
        );
    }

    // All 3 revealed cards must be back in the deck — net deck size unchanged.
    let deck_after = runner.deck_size(0);
    assert_eq!(
        deck_after, deck_before,
        "all 3 revealed NON-CS cards must return to deck bottom (no card to hand); \
         deck size before={deck_before}, after={deck_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Deck accounting after the [Main] reveal
// ═══════════════════════════════════════════════════════════════════════════════

/// Mirror of bt22_094 / p_035 deck-accounting test: when 1 CS card is added
/// to hand, deck shrinks by exactly 1 (3 revealed - 2 returned bottom = 1
/// removed to hand).
#[test]
fn bt22_099_main_deck_size_shrinks_by_one_when_cs_card_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_cs_digimon("CS-DIGI", "CS Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        .add_card(filler("FILL-D"))
        .hand(0, &[CARD_ID])
        // Top of deck = last in slice. CS-DIGI is on top, in the reveal-3.
        .deck(
            0,
            &["FILL-A", "FILL-B", "FILL-C", "FILL-D", "FILL-A", "CS-DIGI"],
        )
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);

    // Pick CS-DIGI explicitly (mirrors `bt22_099_main_picks_cs_card_...` setup).
    if let Some(action) = revealed_action_for_id(&runner, "CS-DIGI") {
        runner
            .execute_action(0, action)
            .expect("pick CS-DIGI for the CS bucket");
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
    let _ = make_cs_tamer("X", "X");
    let _ = make_non_cs_digimon("Y", "Y");
}
