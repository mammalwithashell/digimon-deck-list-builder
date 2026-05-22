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
//! # Clause decomposition
//! - Clause 0 ([Main] main_from_hand): reveal top 4 → select 1 red Digimon →
//!   add to hand → place remainder at deck bottom → place self in battle area.
//! - Clause 1 ([Main]<Delay> end_of_your_next_turn): gain 2 memory.
//! - Clause 2 ([Security], inherited): place self in battle area.
//!
//! # Patterns this test covers
//! - A1  Reveal top-N + select-by-color/kind + add to hand (filter ENFORCED)
//! - A2  place_remainder_on_deck(bottom) with auto ordered permutation
//! - `place_self_as_delay_option` — option-as-Delay-permanent placement, both
//!   from the [Main] main-from-hand context and the inherited [Security] context
//! - Standard Delay (EndOfYourNextTurn): gain_memory(2) on activation
//!
//! # Gap status
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT** — RESOLVED 2026-05-02. The DSL verb
//!   `place_self_as_delay_option: {}` places the currently-resolving Option card
//!   into the owner's battle area as an `OptionState::Delayed` permanent. It is
//!   used for BOTH the [Main] "Then, place this card in your battle area." tail
//!   and the inherited [Security] "Place this card in the battle area." clause.
//!
//! **select_reveal filter** — `install_select_reveal` enforces the compiled
//!   `filter: { all_of: [kind:digimon, color_is:red] }` when building
//!   `valid_action_ids`: only red Digimon among the 4 revealed are selectable,
//!   and with zero eligible the optional select short-circuits (no card added).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, TriggerSource};

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

/// A red Tamer card — NOT eligible (wrong kind — must be Digimon).
fn make_red_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Red];
    c
}

/// Generic filler card. `make_test_card` defaults to a red Digimon, which would
/// pass the Main-clause filter — so for "no eligible card" scenarios use the
/// explicit non-eligible builders above. This filler is for decks/security
/// padding where eligibility is irrelevant.
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
///   [2] inherited on_security (triggered, scope: Inherited)
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

/// Clause 0's process must end with PlaceSelfAsDelayOption — the printed text's
/// "Then, place this card in your battle area." tail.
#[test]
fn p_035_clause_0_process_places_self_as_delay_option() {
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
                t.process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
                "clause 0 process must include PlaceSelfAsDelayOption; got {:?}",
                t.process
            );
        }
        other => panic!("clause 0 must be Triggered; got {:?}", other),
    }
}

/// Clause 1: Delay declarative with the standard printed `<Delay>` trigger
/// (`delayed` → CompiledTiming::Delayed → DelayTrigger::MainPhaseActivated).
/// This is the player-initiated [Main]-phase activation per RULES_CONTEXT 16-16.
#[test]
fn p_035_clause_1_is_delay_main_phase_activated() {
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
                CompiledTiming::Delayed,
                "Delay trigger must be Delayed (standard player-initiated <Delay>); got {:?}",
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

/// Clause 2: inherited scope, SecuritySkill timing, process places self.
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
    match &compiled.effects[2] {
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
                t.process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
                "clause 2 process must include PlaceSelfAsDelayOption; got {:?}",
                t.process
            );
        }
        other => panic!("clause 2 must be Triggered; got {:?}", other),
    }
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
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        // Builder: last element in slice = top of deck.
        .hand(0, &["P-035"])
        .deck(0, &["BLUE-D1", "BLUE-D2", "BLUE-D3", "RED-D1"])
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    // Activate the [Main] effect from hand (hand index 0).
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for P-035");

    // A SelectReveal prompt must be pending after revealing 4 cards that
    // include exactly one eligible red Digimon.
    assert!(
        runner.game.pending_selection.is_some(),
        "A selection prompt must be pending after revealing 4 cards that include a red Digimon"
    );
}

/// When the player selects a red Digimon from the reveal, it goes to hand and
/// P-035 is placed in the battle area as a Delay permanent.
///
/// Net hand accounting: P-035 starts in hand (1) → one red Digimon added (+1)
/// → P-035 placed in battle area via place_self_as_delay_option (-1) → net 1.
#[test]
fn p_035_main_selected_red_digimon_added_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        // Deck: last in slice = top; RED-D1 is the only eligible card.
        .hand(0, &["P-035"])
        .deck(0, &["BLUE-D1", "BLUE-D2", "BLUE-D3", "RED-D1"])
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    // Auto-resolve all pending selections (reveal pick + remainder ordering).
    let _ = runner.auto_resolve();

    // RED-D1 must be in hand; P-035 must NOT (it was placed in battle area).
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"RED-D1".to_string()),
        "the selected red Digimon must be added to hand; hand={hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"P-035".to_string()),
        "P-035 must leave the hand (placed in battle area); hand={hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "hand must hold exactly the 1 added red Digimon; hand={hand_ids:?}"
    );
}

/// After Main resolves, P-035 lands in the battle area as an OptionState::Delayed
/// permanent (the "Then, place this card in your battle area." tail).
#[test]
fn p_035_main_places_self_in_battle_area_as_delay_permanent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        .hand(0, &["P-035"])
        .deck(0, &["BLUE-D1", "BLUE-D2", "BLUE-D3", "RED-D1"])
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    let battle_before = runner.game.players[0].battle_area.len();
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "P-035 must be placed into the battle area as a new permanent"
    );
    let placed = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-035")
        .expect("P-035 must be a battle-area permanent after Main resolves");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 0, .. }),
        "placed P-035 must be an OptionState::Delayed permanent owned by player 0; got {:?}",
        placed.option_state
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2b — Behavioral: Main clause — negative condition (no eligible card)
// ─────────────────────────────────────────────────────────────────────────────

/// Negative: when no red Digimon is among the top 4, the select_reveal filter
/// finds zero eligible cards. The optional select short-circuits, no card is
/// added to hand, the 4 revealed cards return to deck bottom, and P-035 is
/// still placed in the battle area.
#[test]
fn p_035_main_no_red_digimon_does_not_add_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        .add_card(make_red_option("RED-OPT"))
        // Top 4: 3 blue Digimon + 1 red Option — no red Digimon.
        .hand(0, &["P-035"])
        .deck(0, &["BLUE-D1", "BLUE-D2", "BLUE-D3", "RED-OPT"])
        .deck(1, &["BLUE-D1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // Hand must be empty: P-035 left the hand (placed in battle area) and no
    // card was added from the reveal (no eligible red Digimon).
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.is_empty(),
        "no red Digimon in top 4: no card added, P-035 placed → hand empty; got {hand_ids:?}"
    );
}

/// Negative: a blue Digimon must not be selectable (wrong color). With all four
/// top-of-deck cards blue Digimon, no card passes the `color_is: red` filter,
/// so the reveal pick installs no selectable action and nothing is added.
#[test]
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

    // The reveal-pick selection (if installed) must not offer any blue Digimon.
    // `select_reveal` short-circuits when no candidate matches the filter, so
    // the only pending selection that can appear is the remainder ordering.
    if let Some(sel) = runner.game.pending_selection.as_ref() {
        assert!(
            !matches!(sel.kind, digimon_engine::selection::SelectionKind::Reveal),
            "no Reveal-pick selection may be installed when no red Digimon is eligible"
        );
    }

    let _ = runner.auto_resolve();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.is_empty(),
        "blue Digimon must not be eligible; no card added → hand empty; got {hand_ids:?}"
    );
}

/// Negative: red Option / red Tamer cards are not eligible (wrong kind — must be
/// a Digimon). Top 4 are 1 red Option + 3 red Tamers — all red, none Digimon —
/// so the `kind: digimon` filter excludes them all.
#[test]
fn p_035_main_red_non_digimon_is_not_eligible_for_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_option("RED-OPT"))
        .add_card(make_red_tamer("TAMER-1"))
        .add_card(make_red_tamer("TAMER-2"))
        .add_card(make_red_tamer("TAMER-3"))
        .hand(0, &["P-035"])
        // Top 4: red Option + 3 red Tamers — none are Digimon → not eligible.
        .deck(0, &["TAMER-1", "TAMER-2", "TAMER-3", "RED-OPT"])
        .deck(1, &["RED-OPT"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);

    if let Some(sel) = runner.game.pending_selection.as_ref() {
        assert!(
            !matches!(sel.kind, digimon_engine::selection::SelectionKind::Reveal),
            "no Reveal-pick selection may be installed when only red non-Digimon are revealed"
        );
    }

    let _ = runner.auto_resolve();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.is_empty(),
        "red Option / red Tamer are not Digimon cards; no card added → hand empty; got {hand_ids:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral: standard <Delay> — player-initiated Main-phase action
// ─────────────────────────────────────────────────────────────────────────────

/// Playing P-035 from hand parks it as a `MainPhaseActivated` delayed Option.
/// On a LATER main phase the controller may take the `FIELD_EFFECT` activation
/// action, which trashes the Option as the activation cost and runs the
/// `<Delay>` body (gain 2 memory).
///
/// Drives the real option-play path (`play_option_from_hand`) and the real
/// `[Main]`-phase activation (`decode_action` on the `FIELD_EFFECT` bit).
#[test]
fn p_035_delay_activation_gains_2_memory_via_main_phase_action() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(make_red_digimon("RED-FIELD"))
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        .add_card(filler("FILL"))
        // Reveal top 4: 3 blue Digimon + RED-D1 (top, eligible).
        .hand(0, &["P-035"])
        .deck(0, &["FILL", "FILL", "BLUE-D1", "BLUE-D2", "BLUE-D3", "RED-D1"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    // A red card on the field satisfies the red Option's play color requirement.
    runner.place_on_field(0, "RED-FIELD", Some(0));
    runner.game.enter_main_phase();

    // Play P-035 as an Option from hand. Reveal/select + delay placement
    // installs a pending selection → OptionPlayResult::Pending.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "P-035 install + reveal selection must park as Pending"
    );
    runner
        .auto_resolve()
        .expect("resolve P-035 reveal selection and delay placement");

    // P-035 parks as a MainPhaseActivated delayed Option permanent.
    let delay_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| {
            matches!(
                p.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
        })
        .expect("playing P-035 must park it as a MainPhaseActivated delayed option");

    // OPT/lockout-style check: same turn as placement → activation is NOT legal
    // (RULES_CONTEXT 16-16-3: cannot activate on the placing turn).
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "P-035 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    // The Delay activation is now a legal [Main]-phase action; PASS too.
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "P-035 <Delay> activation must be a legal action after the placing turn"
    );
    assert_eq!(mask[PASS as usize], 1.0, "declining the <Delay> stays legal");

    // Take the activation: trash P-035 as cost, run the body (gain 2 memory).
    runner.game.decode_action(bit as u16, 0);

    assert_eq!(
        runner.memory(),
        2,
        "P-035 <Delay> body must gain 2 memory when the player activates it"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "P-035 must be trashed as the <Delay> activation cost"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Deck accounting after Main
// ─────────────────────────────────────────────────────────────────────────────

/// After Main resolves with 1 red Digimon added to hand, deck size shrinks by 1
/// net (4 revealed - 3 returned bottom = 1 removed to hand). P-035's placement
/// does not touch the deck.
#[test]
fn p_035_main_deck_size_shrinks_by_one_when_card_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-D1"))
        .add_card(make_blue_digimon("BLUE-D1"))
        .add_card(make_blue_digimon("BLUE-D2"))
        .add_card(make_blue_digimon("BLUE-D3"))
        .add_card(filler("FILL"))
        .hand(0, &["P-035"])
        // 8 cards in deck: last = top; RED-D1 is the only eligible card.
        .deck(
            0,
            &[
                "FILL", "FILL", "FILL", "FILL", "BLUE-D1", "BLUE-D2", "BLUE-D3", "RED-D1",
            ],
        )
        .deck(1, &["FILL"])
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

/// After Main with no eligible red Digimon, all 4 revealed cards return to deck
/// bottom, so deck size is unchanged.
#[test]
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
        // 6 blue Digimon in deck — none eligible; all 4 revealed return to bottom.
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
// SECTION 5 — Inherited security clause
// ─────────────────────────────────────────────────────────────────────────────

/// The inherited security clause (clause 2) has Inherited scope.
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

/// Inherited [Security]: when P-035 is revealed from a player's security stack
/// during a security check, its inherited security clause fires and places
/// P-035 into that player's battle area as a Delay permanent (instead of
/// disposing it to trash). Driven through the real `attack_player` combat path.
#[test]
fn p_035_inherited_security_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        // Player 1's lone security card is P-035.
        .security(1, &["P-035"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .memory(10)
        .start();

    // Player 0's attacker hits player 1's security.
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let result = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "the single security check must resolve (attacker survives)"
    );
    // P-035 must NOT fall through to security trash — it was placed instead.
    assert_eq!(
        runner.trash_size(1),
        0,
        "revealed P-035 must not be disposed to trash; it is placed in the battle area"
    );
    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-035")
        .expect("P-035 must become a battle-area permanent after the security check");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 1, .. }),
        "placed P-035 must be an OptionState::Delayed permanent owned by player 1; got {:?}",
        placed.option_state
    );
}

/// Inherited [Security] placement preserves P-035's own Delay trigger
/// (the standard player-initiated `MainPhaseActivated` `<Delay>`) on the
/// placed permanent — the placed Option remains a standard parked `<Delay>`.
#[test]
fn p_035_inherited_security_placement_preserves_main_phase_activated_trigger() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .security(1, &["P-035"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-035")
        .expect("P-035 must be placed after the security check");
    assert!(
        matches!(
            placed.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        ),
        "placed P-035 must carry its MainPhaseActivated <Delay> trigger; got {:?}",
        placed.option_state
    );
}
