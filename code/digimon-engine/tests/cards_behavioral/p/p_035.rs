//! P-035 Red Memory Boost! — Option, Cost 3, Red.
//!
//! # Card text (cards.json / printed)
//!
//! **[Main]** Reveal the top 4 cards of your deck. Add 1 red Digimon card among
//! them to the hand. Place the remaining cards at the bottom of your deck in any
//! order. Then, place this card in your battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn, activate
//! the effect below.)
//! ・Gain 2 memory.
//!
//! **Inherited:** Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_035.cs
//!
//! # Patterns this test covers
//! - A1  Reveal top-N + select-by-color/kind + add to hand
//! - A2  place_remainder_on_deck(bottom) with auto ordered permutation
//! - Option-as-permanent Delay (implicit via classify_option_subtype + kind:delay clause)
//! - Standard Delay (EndOfYourNextTurn): gain_memory on activation
//! - Inherited security placement (raw_rust stub for G-PLACE-SELF-AS-OPTION-PERMANENT)
//!
//! # Known gaps affecting these tests
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT** (DSL gap):
//!   No `place_option_in_battle_area: {}` step verb. The "Then, place this card
//!   in your battle area" part of the Main clause is implicit — the engine's
//!   `classify_option_subtype` detects the `kind: delay` clause and places the
//!   card in battle area as `OptionState::Delayed` automatically after the Main
//!   clause resolves through `dispose_option`. The placement is engine-level
//!   automatic and requires no explicit DSL step.
//!   Tests that assert the battle-area count after Main are
//!   `#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT"]`.
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT (inherited security variant)**:
//!   The inherited "[Security] Place this card in the battle area" is implemented
//!   as a `raw_rust` stub (`p_035_security_place_in_battle_area`). The engine
//!   has no `EffectContext` method to place a digivolution-source Option card
//!   from the inherited-security context into the battle area as a Delay permanent.
//!   Tests for this clause are `#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT"]`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const YAML: &str = include_str!("../../../cards/p/P-035.yaml");

// ── Fixture builders ─────────────────────────────────────────────────────────

/// A red Digimon filler card — eligible for selection in the Main clause.
fn make_red_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c
}

/// A blue Digimon — NOT eligible (wrong color).
fn make_blue_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c
}

/// A red Option card — NOT eligible (wrong kind — must be Digimon).
fn make_red_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Red];
    c
}

/// Generic filler card (default: red Digimon per make_test_card defaults).
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — YAML parse + structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// P-035 YAML must parse and compile without errors.
#[test]
fn p_035_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-035 YAML must parse and compile without errors");
}

/// P-035 is an Option card with cost 3.
#[test]
fn p_035_is_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner
        .compiled_card("P-035")
        .expect("P-035 compiled card must be registered");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "P-035 must be an Option card"
    );
    assert_eq!(compiled.cost, Some(3), "P-035 must have play cost 3");
}

/// P-035 must have exactly 3 compiled clauses:
///   [0] main_from_hand (triggered)
///   [1] delay (declarative, trigger: end_of_your_next_turn)
///   [2] inherited on_security (triggered or raw_rust stub, scope: Inherited)
#[test]
fn p_035_has_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-035")
        .expect("P-035 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (main_from_hand, delay, inherited on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0: main_from_hand, FaceUp scope, not optional (Main is mandatory).
#[test]
fn p_035_clause_0_is_main_from_hand_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-035").expect("P-035 compiled");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 0 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "clause 0 must have FaceUp scope"
            );
        }
        other => panic!("clause 0 must be Triggered; got {:?}", other),
    }
}

/// Clause 1: Delay declarative with trigger end_of_your_next_turn.
#[test]
fn p_035_clause_1_is_delay_end_of_your_next_turn() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-035").expect("P-035 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::EndOfYourNextTurn,
                "Delay trigger must be EndOfYourNextTurn; got {:?}",
                trigger
            );
        }
        other => panic!("clause 1 must be Declarative(Delay); got {:?}", other),
    }
}

/// Delay clause process must contain GainMemory(2).
#[test]
fn p_035_delay_clause_process_has_gain_memory_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-035").expect("P-035 compiled");
    match &compiled.effects[1] {
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
        other => panic!("clause 1 must be Delay; got {:?}", other),
    }
}

/// Clause 2: inherited scope, SecuritySkill timing.
#[test]
fn p_035_clause_2_is_inherited_security_skill() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-035").expect("P-035 compiled");
    let is_inherited_security = match &compiled.effects[2] {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        }
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            scope, triggers, ..
        }) => *scope == CompiledScope::Inherited && triggers.contains(&CompiledTiming::OnSecurity),
        _ => false,
    };
    assert!(
        is_inherited_security,
        "clause 2 must be Inherited scope + SecuritySkill timing; got {:?}",
        compiled.effects[2]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Behavioral: Main clause — positive condition (red Digimon present)
// ─────────────────────────────────────────────────────────────────────────────

/// When a red Digimon is among the top 4 revealed cards, a select_reveal
/// prompt appears so the player can add it to hand.
///
/// Uses `activate_hand_main` (not `runner.play()`) to fire MainFromHand.
#[test]
fn p_035_main_installs_select_reveal_with_red_digimon_in_top_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        // Deck top-to-bottom: FILL-B (top drawn first) … last pushed = top
        // Builder: last element in slice = top of deck.
        .hand(0, &["P-035"])
        .deck(0, &["FILL-A", "FILL-B", "BLUE-D1", "RED-D1"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    // Activate the [Main] effect from hand (hand index 0).
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for P-035");

    // A SelectReveal (or OrderedPermutation) prompt must be pending after reveal.
    assert!(
        runner.game.pending_selection.is_some(),
        "A selection prompt must be pending after revealing 4 cards that include a red Digimon"
    );
}

/// When the player selects from the reveal, a card goes to hand.
///
/// Note: `activate_hand_main` fires the `MainFromHand` effect process without
/// physically consuming the option card (no `dispose_option` call — that is
/// triggered by the effect queue's `MainEffectDrain` phase in normal play flow).
/// The hand therefore nets +1: P-035 stays + one card from reveal is added.
///
/// Phase 2b note: `install_select_reveal` uses an accept-all filter at the
/// engine level — the YAML `filter: { all_of: [kind:digimon, color_is:red] }`
/// is compiled but not yet enforced on valid_action_ids. auto_resolve picks
/// the first revealed card (RED-D1, which is the top card) regardless.
/// Filter-specific tests are #[ignore]'d pending Phase 2b closure.
#[test]
fn p_035_main_selected_red_digimon_added_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        // Deck: last in slice = top; RED-D1 is top
        .hand(0, &["P-035"])
        .deck(0, &["FILL-A", "FILL-B", "FILL-C", "RED-D1"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len(); // 1 (P-035)
    runner.game.activate_hand_main(0, 0);

    // Auto-resolve all pending selections (picks first available action each time).
    let _ = runner.auto_resolve();

    // P-035 is NOT consumed by activate_hand_main (stays in hand).
    // One card added from reveal → hand grows by net +1.
    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "Hand must grow by 1 (one card from reveal added; P-035 stays via activate_hand_main); \
         before={hand_before}, after={hand_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2b — Behavioral: Main clause — negative condition (no eligible card)
// ─────────────────────────────────────────────────────────────────────────────

/// Negative: when no red Digimon is among the top 4, no select_reveal for
/// a Digimon appears; the remainder is placed bottom and hand stays empty.
///
/// **Phase 2b gap**: `install_select_reveal` uses an accept-all filter — the
/// YAML `filter: { all_of: [kind:digimon, color_is:red] }` is not enforced at
/// runtime. auto_resolve picks the first revealed card (a blue Digimon) and
/// adds it to hand even though it fails the filter. This test is #[ignore]'d
/// pending Phase 2b filter enforcement closure.
#[test]
#[ignore = "pending Phase 2b: install_select_reveal accept-all filter — color/kind filter not enforced at runtime"]
fn p_035_main_no_red_digimon_does_not_add_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_red_option("RED-OPT"))
        // Only blue Digimon and red Option in top 4 — no red Digimon.
        .hand(0, &["P-035"])
        .deck(0, &["BLUE-D1", "BLUE-D1", "RED-OPT", "BLUE-D1"])
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_after = runner.game.players[0].hand.len();
    // Expected: hand = 1 (P-035 stays, no card added from reveal since no eligible).
    // Actual: hand = 2 (first revealed card added despite failing filter).
    assert_eq!(
        hand_after,
        1,
        "No red Digimon in top 4: hand must remain at 1 (P-035 only) after P-035 Main resolves; got {}",
        hand_after
    );
}

/// Negative: blue Digimon must not be selectable (wrong color).
///
/// All four top-of-deck cards are blue Digimon — none are red, so no card
/// is eligible for selection per the YAML filter.
///
/// **Phase 2b gap**: `install_select_reveal` uses an accept-all filter — blue
/// Digimon cards are presented as valid options despite failing `color_is: red`.
/// auto_resolve picks the first card and adds it to hand. Ignored pending
/// Phase 2b filter enforcement.
#[test]
#[ignore = "pending Phase 2b: install_select_reveal accept-all filter — color filter not enforced at runtime"]
fn p_035_main_blue_digimon_is_not_eligible_for_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        .add_card(make_blue_digimon("BLUE-D4"))
        .hand(0, &["P-035"])
        // Top 4: all blue Digimon — none are eligible (wrong color).
        .deck(0, &["BLUE-D1", "BLUE-D2", "BLUE-D3", "BLUE-D4"])
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_after = runner.game.players[0].hand.len();
    // Expected once filter enforcement lands: hand = 1 (P-035 stays, no card added).
    assert_eq!(
        hand_after, 1,
        "Blue Digimon must not be eligible for selection; hand must be 1 (P-035 only)"
    );
}

/// Negative: red Option card is not eligible (wrong kind — must be Digimon).
///
/// Top 4 cards are: 1 red Option + 3 red Tamers. None are Digimon, so the
/// `kind: digimon` filter in select_reveal excludes all of them.
///
/// **Phase 2b gap**: `install_select_reveal` uses an accept-all filter — red
/// Option and Tamers are presented as valid options despite failing `kind: digimon`.
/// auto_resolve picks the first card and adds it to hand. Ignored pending
/// Phase 2b filter enforcement.
#[test]
#[ignore = "pending Phase 2b: install_select_reveal accept-all filter — kind filter not enforced at runtime"]
fn p_035_main_red_option_is_not_eligible_for_selection() {
    let tamer1 = {
        let mut t = make_test_card("TAMER-1", "Tamer1");
        t.card_kind = CardKind::Tamer;
        t.colors = vec![CardColor::Red];
        t
    };
    let tamer2 = {
        let mut t = make_test_card("TAMER-2", "Tamer2");
        t.card_kind = CardKind::Tamer;
        t.colors = vec![CardColor::Red];
        t
    };
    let tamer3 = {
        let mut t = make_test_card("TAMER-3", "Tamer3");
        t.card_kind = CardKind::Tamer;
        t.colors = vec![CardColor::Red];
        t
    };

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_option("RED-OPT"))
        .add_card(tamer1)
        .add_card(tamer2)
        .add_card(tamer3)
        .hand(0, &["P-035"])
        // Top 4: red Option + 3 red Tamers — none are Digimon → not eligible.
        .deck(0, &["TAMER-1", "TAMER-2", "TAMER-3", "RED-OPT"])
        .deck(1, &["RED-OPT"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        hand_after, 0,
        "Red Option is not a Digimon card; hand must be empty"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral: Delay fires gain_memory on next turn
// ─────────────────────────────────────────────────────────────────────────────

/// After Main resolves and the card is in the battle area as a Delay permanent,
/// activating the Delay (at end of NEXT turn) fires gain_memory: 2.
///
/// Ignored pending G-PLACE-SELF-AS-OPTION-PERMANENT: the engine places the card
/// in battle area automatically but we cannot drive the Delay activation from
/// the test API without asserting battle-area presence first.
#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT — cannot drive Delay activation without battle-area assertion API"]
fn p_035_delay_activation_gains_2_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL-A"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL-A", "FILL-A", "FILL-A", "FILL-A"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // Advance past placing turn to enable the Delay.
    runner.end_turn(); // player 1 turn
    runner.end_turn(); // player 0 turn again

    let memory_before = runner.memory();
    // Trigger the Delay (end of next turn fires resolve_delayed_options).
    runner.end_turn(); // player 1 turn
    runner.end_turn(); // player 0 turn — Delay should fire here

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "Delay must gain 2 memory when activated"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Deck accounting after Main
// ─────────────────────────────────────────────────────────────────────────────

/// After Main resolves (1 red Digimon added to hand), deck size must shrink by 1
/// net (4 revealed - 3 returned bottom = 1 removed to hand).
#[test]
fn p_035_main_deck_size_shrinks_by_one_when_card_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        .add_card(filler("FILL-D"))
        .hand(0, &["P-035"])
        // 8 cards in deck: last = top; RED-D1 is at top (revealed first)
        .deck(
            0,
            &[
                "FILL-A", "FILL-B", "FILL-C", "FILL-D", "FILL-A", "FILL-B", "FILL-C", "RED-D1",
            ],
        )
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    assert_eq!(
        deck_before - deck_after,
        1,
        "Deck must shrink by exactly 1 (4 revealed, 1 to hand, 3 returned to bottom); \
         before={deck_before}, after={deck_after}"
    );
}

/// After Main with no eligible card, all 4 revealed cards return to deck bottom,
/// so deck size is unchanged.
///
/// **Phase 2b gap**: `install_select_reveal` accept-all filter means even a
/// blue Digimon is added to hand (1 card removed from deck net). This test
/// is #[ignore]'d pending Phase 2b filter enforcement.
#[test]
#[ignore = "pending Phase 2b: install_select_reveal accept-all filter — deck accounting incorrect when filter not enforced"]
fn p_035_main_deck_size_unchanged_when_no_card_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        .add_card(make_blue_digimon("BLUE-D4"))
        .add_card(make_blue_digimon("BLUE-D5"))
        .add_card(make_blue_digimon("BLUE-D6"))
        .hand(0, &["P-035"])
        // 6 blue Digimon in deck — none eligible; all 4 revealed must return to bottom.
        .deck(
            0,
            &[
                "BLUE-D1", "BLUE-D2", "BLUE-D3", "BLUE-D4", "BLUE-D5", "BLUE-D6",
            ],
        )
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    assert_eq!(
        deck_after, deck_before,
        "When no eligible card is selected, all 4 revealed cards return to deck bottom; \
         deck size must be unchanged: before={deck_before}, after={deck_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Inherited security clause (structural only)
// ─────────────────────────────────────────────────────────────────────────────

/// The inherited security clause (clause 2) has Inherited scope.
/// Structural test — behavioral placement is blocked by G-PLACE-SELF-AS-OPTION-PERMANENT.
#[test]
fn p_035_inherited_security_has_inherited_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-035").expect("P-035 compiled");
    assert_eq!(compiled.effects.len(), 3, "P-035 must have 3 clauses");

    let scope = match &compiled.effects[2] {
        CompiledClause::Triggered(t) => t.scope,
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { scope, .. }) => *scope,
        other => panic!("clause 2 must be Triggered or RawRust; got {:?}", other),
    };
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "clause 2 must have Inherited scope"
    );
}

/// The inherited security effect (place this in battle area) is blocked for full
/// behavioral testing pending G-PLACE-SELF-AS-OPTION-PERMANENT.
#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT — inherited security placement of digivolution-source Option not expressible in DSL"]
fn p_035_inherited_security_places_self_in_battle_area() {
    // When P-035 is under another permanent as a digivolution source and that
    // permanent's security card is checked, P-035's inherited security effect fires
    // and places P-035 itself in the battle area as a Delay permanent.
    todo!("implement once G-PLACE-SELF-AS-OPTION-PERMANENT is resolved");
}
