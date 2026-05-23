//! P-103 Offense Training — Option, Cost 2, Red.
//!
//! # Card text (cards.json)
//!
//! **[Main]** Reveal the top 2 cards of your deck. Add 1 red card among them
//! to your hand. Place the rest at the bottom of your deck in any order. Then,
//! place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn, activate
//! the effect below.)
//! ・1 of your Digimon may digivolve into a red Digimon card in your hand for
//! its digivolution cost. When it would digivolve by this effect, reduce the
//! cost by 2.
//!
//! **Inherited:** Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_103.cs
//!
//! # Clause map
//!
//! - Clause 0 ([Main], `main_from_hand`, mandatory): reveal top 2 →
//!   `select_reveal` (optional, red filter) → `add_to_hand_from_reveal` →
//!   `place_remainder_on_deck(bottom)` → `place_self_as_delay_option`.
//! - Clause 1 ([Main] ＜Delay＞, `kind: delay`, `trigger: delayed` →
//!   DelayTrigger::MainPhaseActivated — player-initiated): `select_own_permanent`
//!   (optional Digimon) → `select_hand` (red Digimon) →
//!   `effect_initiated_digivolve` with `cost: { reduce: 2 }`.
//! - Clause 2 ([Security], inherited, `on_security`):
//!   `place_self_as_delay_option`.
//!
//! # Gap history
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT — RESOLVED 2026-05-02.** The DSL step
//! `place_self_as_delay_option: {}` lowers to
//! `EffectContext::place_self_as_delay_option_permanent`, which seats the
//! resolving Option card in the battle area as an `OptionState::Delayed`
//! permanent and dispatches `OnOptionPlaced`. It is used by BOTH the [Main]
//! clause's "Then, place this card in the battle area" tail and the inherited
//! [Security] "Place this card in the battle area." No P-103 clause is blocked.
//!
//! # Patterns this test covers
//! - A2  Two-pass reveal (reveal 2, select-1 red, place rest at bottom)
//! - E2  optional select in reveal pipeline (`select_reveal optional: true`),
//!       including the no-red-card path where `select_reveal` installs nothing
//! - E1  Delay body: `select_own_permanent` → `select_hand` →
//!       `effect_initiated_digivolve` (cost reduce 2)
//! - Option-as-permanent placement via `place_self_as_delay_option` (Main +
//!   inherited Security)

#![allow(dead_code, unused_imports)]

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, TriggerSource};

const YAML: &str = include_str!("../../../cards/p/P-103.yaml");

// ─── helpers ──────────────────────────────────────────────────────────────────

/// A red Digimon Lv.4 card to serve as the digivolve target in Delay body tests.
fn make_red_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c
}

/// A red Lv.3 Digimon carrier — the on-field Digimon the Delay body digivolves.
fn make_red_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Red];
    c
}

/// A red Lv.4 Digimon with a printed `Lv.3 Red, cost 2` evo cost. With the
/// Delay clause's `cost: { reduce: 2 }`, the effective digivolve cost is 0.
/// Used as the in-hand digivolve target so the digivolve actually succeeds
/// through the engine's standard `printed_digivolve_memory_cost` route
/// (P-103's Delay uses `ignore_requirements: false`).
fn make_red_evo_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 2,
    }];
    c
}

/// A red Option card — should appear as a valid reveal-select result.
fn make_red_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Red];
    c
}

/// A non-red card — should NOT be selectable in the reveal step.
fn make_blue_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Blue];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Drive every pending selection to completion, always taking the first legal
/// action. Returns the number of selections resolved.
fn drain_selections(runner: &mut DebugRunner) -> usize {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    steps
}

/// A `CardEffect` that increments a shared counter every time the
/// `OnOptionPlaced` timing fires — used to prove `place_self_as_delay_option`
/// dispatches the `OnOptionPlaced` effect timing.
struct OptionPlacedWitness(Arc<Mutex<u32>>);

impl CardEffect for OptionPlacedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_play(card)
            .name("OnOptionPlaced witness")
            .timing(EffectTiming::OnOptionPlaced)
            .process(move |_ctx| {
                *witness.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// True if `process` contains a `place_self_as_delay_option` step at any depth.
fn process_contains_place_self_as_delay_option(process: &[CompiledStep]) -> bool {
    process.iter().any(|step| match step {
        CompiledStep::PlaceSelfAsDelayOption => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_place_self_as_delay_option(then)
                || process_contains_place_self_as_delay_option(else_branch)
        }
        CompiledStep::ForEach { body, .. }
        | CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. }
        | CompiledStep::Optional(body)
        | CompiledStep::AsSelectingPlayer { body, .. } => {
            process_contains_place_self_as_delay_option(body)
        }
        CompiledStep::SelectOwnSources { then, .. }
        | CompiledStep::SelectOwnBreedingPermanent { then, .. }
        | CompiledStep::SelectOpponentDpBudget { then, .. } => {
            process_contains_place_self_as_delay_option(then)
        }
        _ => false,
    })
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

/// The YAML must parse and compile without errors.
#[test]
fn p_103_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-103 YAML must parse and compile without errors");
}

/// P-103 is an Option card with cost 2.
#[test]
fn p_103_is_option_cost_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option
    );
    assert_eq!(compiled.cost, Some(2));
}

/// P-103 must have exactly 3 clauses:
///   Clause 0: main_from_hand (triggered, mandatory)
///   Clause 1: delay (declarative)
///   Clause 2: on_security (triggered, inherited scope)
#[test]
fn p_103_has_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (main_from_hand, delay, on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0 fires at main_from_hand, is NOT optional (no "you may" on reveal),
/// and has FaceUp scope.
#[test]
fn p_103_clause0_is_main_from_hand_not_optional_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let clause0 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    assert!(
        !clause0.optional,
        "Clause 0 must NOT be optional — printed text has no 'you may' on the reveal step"
    );
    assert_eq!(
        clause0.scope,
        CompiledScope::FaceUp,
        "Clause 0 must have FaceUp scope"
    );
}

/// Clause 0's process ends with `place_self_as_delay_option` — the now-resolved
/// "Then, place this card in the battle area" tail.
#[test]
fn p_103_clause0_process_places_self_as_delay_option() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let clause0 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    assert!(
        process_contains_place_self_as_delay_option(&clause0.process),
        "Clause 0 must include place_self_as_delay_option for the printed \
         'Then, place this card in the battle area' tail"
    );
}

/// Clause 1 is a Delay declarative clause.
#[test]
fn p_103_clause1_is_delay_declarative() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let has_delay = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
        )
    });

    assert!(has_delay, "Clause 1 must be a declarative Delay clause");
}

/// Clause 2 fires at on_security with Inherited scope.
#[test]
fn p_103_clause2_is_on_security_inherited_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert_eq!(
        security_clause.scope,
        CompiledScope::Inherited,
        "Security clause must have Inherited scope (it is in inherited_effect_description_eng)"
    );
}

/// Clause 2's process is the single `place_self_as_delay_option` step.
#[test]
fn p_103_clause2_process_places_self_as_delay_option() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert!(
        process_contains_place_self_as_delay_option(&security_clause.process),
        "Clause 2 (inherited Security) must include place_self_as_delay_option \
         for 'Place this card in the battle area'"
    );
}

/// P-103 has exactly 2 triggered clauses (main_from_hand + on_security).
#[test]
fn p_103_has_exactly_two_triggered_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "expected 2 triggered clauses (main_from_hand + on_security); got {}",
        triggered.len()
    );
}

// ─── §2 Main clause — reveal pipeline (condition gating + behavioral) ─────────

/// When P-103's [Main] effect fires from hand, the reveal pipeline runs.
///
/// Note: `activate_hand_main` fires `MainFromHand` timing without playing the
/// card — this is the correct dispatch for Option [Main] effects.
#[test]
fn p_103_main_from_hand_fires_reveal_then_select() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-103"])
        .hand(1, &["FILL"])
        // The deck builder pushes in array order; reveal_top_deck pops from the
        // END of the Vec — so the array's last entry is the deck top. RED1 last
        // → it shows up in the reveal.
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "RED1"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for P-103 at hand index 0"
    );
    // After reveal, a selection may be pending (select_reveal prompt) if any
    // revealed cards match the red filter, or the placement permutation. Either
    // way the effect must not panic.
}

/// When the deck has a red card in the top 2, the reveal pipeline selects it
/// and the hand grows by 1. The remaining card is placed at the deck bottom.
///
/// Positive condition test: red card present → select_reveal has a candidate.
#[test]
fn p_103_main_reveals_red_card_adds_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED1"))
        .add_card(make_blue_card("BLUE1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-103"])
        .hand(1, &["FILL"])
        // reveal_top_deck pops from the END of the deck Vec, so the array's
        // last two entries are the top 2: RED1 (top), BLUE1 (second).
        .deck(0, &["FILL", "FILL", "FILL", "BLUE1", "RED1"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();

    runner.game.activate_hand_main(0, 0);
    drain_selections(&mut runner);

    let hand_after = runner.game.players[0].hand.len();
    let deck_after = runner.game.players[0].deck.len();

    // Hand should have grown by 1 (added the selected red card).
    // P-103 itself leaves the hand via place_self_as_delay_option, so the net
    // change is: +1 (RED1) -1 (P-103) = 0. RED1 must be present in hand.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "RED1"),
        "RED1 (the selected red card) must be in hand after the reveal-select"
    );
    // Deck must have shrunk net by at least 1 (RED1 moved to hand; BLUE1
    // returned to bottom; P-103 went to battle area).
    assert!(
        deck_after < deck_before,
        "Deck must shrink (the selected red card left the deck); \
         deck_before={deck_before}, deck_after={deck_after}"
    );
    let _ = hand_before;
    let _ = hand_after;
}

/// When the deck has NO red cards in the top 2, the select_reveal installs no
/// selection (no eligible candidate) and the pipeline advances: no card is
/// added to hand, and the 2 revealed cards are still placed at the deck bottom.
///
/// Negative condition test: no red card in top 2. `select_reveal` returns false
/// when `valid_action_ids` is empty → the runner advances to the sibling steps,
/// `add_to_hand_from_reveal` no-ops on the absent `picked` binding, and
/// `place_remainder_on_deck` still returns the 2 revealed blues to the bottom.
#[test]
fn p_103_main_no_red_card_in_top2_no_add_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_card("BLUE1"))
        .add_card(make_blue_card("BLUE2"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-103"])
        .hand(1, &["FILL"])
        // reveal_top_deck pops from the END of the deck Vec — the array's last
        // two entries are the top 2: BLUE1 (top), BLUE2 (second). No red card.
        .deck(0, &["FILL", "FILL", "FILL", "BLUE2", "BLUE1"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let deck_before = runner.game.players[0].deck.len();

    runner.game.activate_hand_main(0, 0);
    drain_selections(&mut runner);

    // Neither blue card may be added to hand — only red cards qualify.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| {
                let id = cs.card_id(&runner.game.card_data);
                id == "BLUE1" || id == "BLUE2"
            }),
        "No blue card may be added to hand when no red card appears in the top 2"
    );
    // Both revealed blues must be returned to the deck — the deck loses only
    // P-103-unrelated cards. The 2 revealed cards re-enter the deck at the
    // bottom, so the deck size is unchanged by the reveal/return cycle.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "Deck size must be unchanged: 2 revealed blues are returned to the bottom"
    );
    // The reveal pool must be empty after place_remainder_on_deck.
    assert!(
        runner.game.revealed_cards.is_empty(),
        "All revealed cards must be returned to the deck (reveal pool empty)"
    );
}

/// The Main clause fires without panic even when the deck is empty.
///
/// Edge case: reveal_top_deck with count:2 on an empty deck must be a no-op,
/// not a panic.
#[test]
fn p_103_main_empty_deck_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["P-103"])
        .hand(1, &["FILL"])
        .deck(0, &[]) // empty deck
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    drain_selections(&mut runner);
    // No panic is the primary assertion.
}

/// "Then, place this card in the battle area" — the [Main] clause seats P-103
/// in the controller's battle area as an `OptionState::Delayed` permanent.
///
/// Driven through the real Option [Main] path (`activate_hand_main`), then
/// `place_self_as_delay_option` resolves the placement (G-PLACE-SELF-AS-OPTION-
/// PERMANENT resolved 2026-05-02).
#[test]
fn p_103_main_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-103"])
        .hand(1, &["FILL"])
        .deck(0, &["RED1", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(
        runner.game.activate_hand_main(0, 0),
        "activate_hand_main must fire P-103 [Main]"
    );
    drain_selections(&mut runner);

    // P-103 must no longer be in hand — it was placed in the battle area.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "P-103"),
        "P-103 must leave the hand when placed in the battle area"
    );

    let placed = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-103")
        .expect("P-103 must become a battle-area permanent after the [Main] clause");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 0, .. }),
        "P-103 must be seated as an OptionState::Delayed permanent; got {:?}",
        placed.option_state
    );
}

/// `place_self_as_delay_option` from the [Main] clause dispatches the
/// `OnOptionPlaced` effect timing — proven via a witness effect on an
/// opponent permanent whose `OnOptionPlaced` observer increments a counter.
#[test]
fn p_103_main_placement_dispatches_on_option_placed() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut observer = make_filler("P103-OBSERVER");
    observer.card_kind = CardKind::Digimon;
    observer.level = Some(4);
    observer.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED1"))
        .add_card(observer.clone())
        .add_card(make_filler("FILL"))
        .hand(0, &["P-103"])
        .hand(1, &["FILL"])
        .deck(0, &["RED1", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.register_effect("P103-OBSERVER", Arc::new(OptionPlacedWitness(witness.clone())));
    // Seat the observer so its OnOptionPlaced effect is live on the field.
    runner.place_on_field(1, "P103-OBSERVER", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    drain_selections(&mut runner);

    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "place_self_as_delay_option must dispatch the OnOptionPlaced timing exactly once"
    );
}

// ─── §3 Delay clause — structural ────────────────────────────────────────────

/// The Delay clause is present as a declarative Delay with the standard printed
/// `<Delay>` trigger (`delayed` → CompiledTiming::Delayed → DelayTrigger::
/// MainPhaseActivated). DCGO: OnDeclaration is CanDeclareOptionDelayEffect — a
/// player-initiated [Main]-phase activation per RULES_CONTEXT 16-16, not an
/// engine-scheduled auto-fire.
#[test]
fn p_103_delay_clause_is_declarative_delay() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let delay_clause = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
        )
    });

    match delay_clause {
        Some(CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, ..
        })) => {
            assert_eq!(
                *trigger,
                CompiledTiming::Delayed,
                "Delay trigger must be Delayed (standard player-initiated <Delay>); got {:?}",
                trigger
            );
        }
        other => panic!("Delay clause must be a declarative Delay; got {:?}", other),
    }
}

// ─── §4 Delay body — behavioral (via placed permanent + game phases) ──────────

/// When P-103 is placed in the battle area (as a Delay permanent) and the Delay
/// body fires, the player can select a Digimon to digivolve into a red Digimon
/// in hand with cost -2.
///
/// Positive condition: at least 1 Digimon on field + red Digimon in hand →
/// selection installs.
#[test]
fn p_103_delay_body_installs_digimon_selection_when_conditions_met() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_evo_lv4("RED_EVO"))
        .add_card(make_red_carrier("CARRIER"))
        .add_card(make_filler("FILL"))
        .hand(0, &["RED_EVO", "FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Place P-103 as a Delay permanent in the battle area (simulating it was
    // placed there by the Main clause). place_on_field bypasses play costs.
    let delay_perm = runner.place_on_field(0, "P-103", Some(0));

    // Place a Digimon for the player to digivolve.
    let _carrier = runner.place_on_field(0, "CARRIER", Some(0));

    // Trigger the Delay body via the DelayEffect timing.
    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    // The Delay body should have installed a pending selection
    // (select_own_permanent for the target Digimon).
    assert!(
        runner.game.pending_selection.is_some(),
        "Delay body must install a selection when an eligible Digimon is on field"
    );
    drain_selections(&mut runner);
    // No panic = Delay body executed without error.
}

/// Negative condition test for Delay body: if no Digimon is on field, the
/// select_own_permanent (optional) is skipped and no digivolve occurs.
#[test]
fn p_103_delay_body_optional_target_when_no_digimon_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_evo_lv4("RED_EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["RED_EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Place P-103 in battle area but NO other Digimon on field.
    let delay_perm = runner.place_on_field(0, "P-103", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    drain_selections(&mut runner);

    // RED_EVO must still be in hand — there was no Digimon to digivolve.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "RED_EVO"),
        "RED_EVO must remain in hand when there is no Digimon to digivolve"
    );
}

/// Delay body: the player selects a Lv.3 red carrier Digimon and a Lv.4 red
/// Digimon in hand; `effect_initiated_digivolve` runs with cost reduction 2.
///
/// The carrier is `make_red_carrier` (Lv.3 Red); the hand card is
/// `make_red_evo_lv4` (Lv.4 Red, printed `Lv.3 Red, cost 2`). With
/// `cost: { reduce: 2 }` the effective cost is 0, so the digivolve succeeds:
/// RED_EVO becomes the top card of the carrier's stack.
///
/// DCGO: payCost: true, reduceCostTuple: (2, null) → cost: { reduce: 2 }.
#[test]
fn p_103_delay_body_digivolves_with_cost_reduction_2() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_evo_lv4("RED_EVO"))
        .add_card(make_red_carrier("CARRIER"))
        .add_card(make_filler("FILL"))
        .hand(0, &["RED_EVO", "FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-103", Some(0));
    let carrier_perm = runner.place_on_field(0, "CARRIER", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    drain_selections(&mut runner);

    // After the chained selections + digivolve, RED_EVO must be the top card of
    // the (former) CARRIER stack, and must have left the hand.
    let carrier = &runner.game.players[0].battle_area[carrier_perm.index as usize];
    assert_eq!(
        carrier.top_card().card_id(&runner.game.card_data),
        "RED_EVO",
        "RED_EVO must be the top card of the carrier stack after the Delay digivolve"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "RED_EVO"),
        "RED_EVO must leave the hand once it digivolves onto the carrier"
    );
}

/// Playing P-103 from hand parks it as a `MainPhaseActivated` delayed Option.
/// On a LATER main phase the controller may take the `FIELD_EFFECT` activation
/// action, which trashes the Option as the activation cost and runs the
/// `<Delay>` body (1 of your Digimon may digivolve into a red Digimon in hand
/// for cost −2).
///
/// Drives the real option-play path (`play_option_from_hand`) and the real
/// `[Main]`-phase activation (`decode_action` on the `FIELD_EFFECT` bit) — the
/// player-initiated `<Delay>` route, not `enqueue_triggered` force-firing.
/// RULES_CONTEXT 16-16-3: the `<Delay>` cannot be activated on the placing turn.
#[test]
fn p_103_delay_activation_digivolves_via_main_phase_action() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_evo_lv4("RED_EVO"))
        .add_card(make_red_carrier("CARRIER"))
        .add_card(make_filler("FILL"))
        // Reveal top 2: 2 filler (no red card to add — keeps the reveal simple).
        .hand(0, &["P-103", "RED_EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    // The on-field red Lv.3 Digimon the Delay body will digivolve. As the sole
    // eligible target it is also a red card that satisfies P-103's red-Option
    // play-color requirement.
    let carrier_perm = runner.place_on_field(0, "CARRIER", Some(0));
    runner.game.enter_main_phase();

    // Play P-103 as an Option from hand. Reveal + delay placement installs a
    // pending selection → OptionPlayResult::Pending.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "P-103 install + reveal selection must park as Pending"
    );
    runner
        .auto_resolve()
        .expect("resolve P-103 reveal selection and delay placement");

    // P-103 parks as a MainPhaseActivated delayed Option permanent.
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
        .expect("playing P-103 must park it as a MainPhaseActivated delayed option");

    // Lockout check: same turn as placement → activation is NOT legal
    // (RULES_CONTEXT 16-16-3: cannot activate on the placing turn).
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "P-103 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // The Delay activation is now a legal [Main]-phase action; PASS too.
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "P-103 <Delay> activation must be a legal action after the placing turn"
    );
    assert_eq!(mask[PASS as usize], 1.0, "declining the <Delay> stays legal");

    // Take the activation: trash P-103 as cost, then run the Delay body.
    runner.game.decode_action(bit as u16, 0);
    // The Delay body installs select_own_permanent → select_hand chained
    // selections (digivolve a Digimon into a red Digimon in hand).
    runner.auto_resolve().expect("resolve Delay body selections");

    // P-103 must be trashed as the <Delay> activation cost — no delayed Option
    // permanent remains.
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "P-103 must be trashed as the <Delay> activation cost"
    );

    // The Delay body ran: RED_EVO digivolved onto the CARRIER stack (cost −2
    // makes the printed Lv.3 Red cost-2 evo free) and left the hand.
    let carrier = &runner.game.players[0].battle_area[carrier_perm.index as usize];
    assert_eq!(
        carrier.top_card().card_id(&runner.game.card_data),
        "RED_EVO",
        "RED_EVO must be the top card of the carrier stack after the <Delay> digivolve"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "RED_EVO"),
        "RED_EVO must leave the hand once it digivolves onto the carrier"
    );
}

// ─── §5 Security clause — structural + behavioral ────────────────────────────

/// The on_security clause is present with inherited scope (it is in
/// inherited_effect_description_eng per cards.json).
#[test]
fn p_103_security_clause_is_inherited_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-103")
        .expect("P-103 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity));

    assert!(
        security_clause.is_some(),
        "P-103 must have an on_security clause"
    );
    assert_eq!(
        security_clause.unwrap().scope,
        CompiledScope::Inherited,
        "Security clause must be inherited scope"
    );
}

/// The on_security clause fires without panic when triggered on a field permanent.
#[test]
fn p_103_security_clause_fires_without_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .memory(10)
        .start();

    // Place P-103 in the battle area (as if it was placed by the Main clause).
    let field_handle = runner.place_on_field(0, "P-103", Some(0));

    // Fire the SecuritySkill timing for this permanent.
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();
    drain_selections(&mut runner);
    // No panic is the primary assertion.
}

/// [Security] "Place this card in the battle area." — driven through the real
/// combat / security-check path: P-103 sits in the defender's security stack,
/// an attacker checks it, and the inherited [Security] clause places P-103 in
/// the defender's battle area as an `OptionState::Delayed` permanent.
///
/// Driven through the real security path (see BT17-097's
/// `bt17_097_security_plays_davis_from_hand_and_places_self_on_field`) — an
/// `enqueue_triggered(SecuritySkill, ..)` shortcut is a silent no-op for
/// inherited-scope [Security] clauses.
#[test]
fn p_103_security_places_self_in_battle_area() {
    let mut attacker = make_filler("P103-ATK");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(attacker.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .security(1, &["P-103"])
        .start();

    let attacker_handle = runner.place_on_field(0, "P103-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: P-103 in the defender's security stack"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // P-103's inherited [Security] clause must have run: P-103 leaves the
    // security stack and is seated as a Delay-Option permanent on the
    // defender's field.
    assert_eq!(
        runner.security_count(1),
        0,
        "P-103 must leave the defender's security stack after the security check"
    );

    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-103")
        .expect("P-103 must be placed in the defender's battle area after [Security]");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 1, .. }),
        "P-103 must be seated as an OptionState::Delayed permanent owned by the \
         defender; got {:?}",
        placed.option_state
    );

    // P-103 must not have fallen through to the security-dispose trash.
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "P-103"),
        "P-103 must not fall through to the trash — the [Security] clause placed it"
    );
}
