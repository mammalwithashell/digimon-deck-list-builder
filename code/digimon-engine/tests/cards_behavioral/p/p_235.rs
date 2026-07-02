//! P-235 Digital Accident Tactics Squad — Option, Cost 3, Yellow.
//!
//! # Card text (card image — authoritative; cards.json accurate)
//!
//! ＜Use Req. ([DATA SQUAD] trait)＞ (Specified cards let you ignore color
//! requirements.)
//!
//! **[Main]** Reveal the top 3 cards of your deck. Add 1 card with the
//! [DATA SQUAD] trait among them to the hand. Return the rest to the bottom of
//! the deck. Then, place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn, activate
//! the effect below.)
//! ・Gain 2 memory.
//!
//! **Inherited:** [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Yellow/P_235.cs
//!   - EffectTiming.None: UseRequirements(card, EqualsTraits("DATA SQUAD")).
//!   - EffectTiming.OptionSkill: SimplifiedRevealDeckTopCardsAndSelect(3,
//!     DATA SQUAD trait, maxCount 1, AddHand, RemainingCardsPlace.DeckBottom),
//!     then PlaceDelayOptionCards.
//!   - EffectTiming.OnDeclaration: Gain2MemoryOptionDelayEffect.
//!   - EffectTiming.SecuritySkill: PlaceSelfDelayOptionSecurityEffect.
//!
//! # Clause map
//!
//! - Clause 0 (use_requirement): DATA SQUAD board predicate → ignore color reqs.
//! - Clause 1 ([Main], `main_from_hand`, mandatory): reveal top 3 →
//!   `select_reveal` (optional, DATA SQUAD trait filter) →
//!   `add_to_hand_from_reveal` → `place_remainder_on_deck(bottom)` →
//!   `place_self_as_delay_option`.
//! - Clause 2 ([Main] ＜Delay＞, `kind: delay`, `trigger: delayed`): gain 2 memory.
//! - Clause 3 ([Security], inherited, `on_security`): `place_self_as_delay_option`.
//!
//! # Patterns this test covers
//! - A1/A2  reveal top 3, add 1 by trait, return rest to deck bottom
//! - Option use_requirement (DATA SQUAD board predicate, color-ignore)
//! - Delay body: gain 2 memory (player-initiated <Delay>)
//! - Option-as-permanent placement via `place_self_as_delay_option` (Main +
//!   inherited Security)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/p/P-235.yaml");

// ─── helpers ──────────────────────────────────────────────────────────────────

/// A DATA SQUAD Digimon — a valid reveal-select target.
fn make_data_squad(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A non-DATA-SQUAD card — must NOT be selectable in the reveal step.
fn make_non_ds(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Cyborg".to_string()];
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
        _ => false,
    })
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

#[test]
fn p_235_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-235 YAML must parse and compile without errors");
}

#[test]
fn p_235_is_option_cost_3_yellow() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-235")
        .expect("P-235 compiled card present");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option
    );
    assert_eq!(compiled.cost, Some(3));
}

/// P-235 declares a use_requirement (DATA SQUAD color-ignore).
#[test]
fn p_235_has_use_requirement() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-235")
        .expect("P-235 compiled card present");

    assert!(
        compiled.use_requirement.is_some(),
        "P-235 must carry a use_requirement for the <Use Req. ([DATA SQUAD] trait)> color-ignore"
    );
}

/// Clause 0 (Main) fires at main_from_hand, is NOT optional (no "you may" on the
/// reveal), FaceUp scope, and ends with place_self_as_delay_option.
#[test]
fn p_235_main_clause_shape() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-235")
        .expect("P-235 compiled card present");

    let main = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    assert!(!main.optional, "Main reveal clause must NOT be optional");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(
        process_contains_place_self_as_delay_option(&main.process),
        "Main clause must place self in battle area"
    );
}

/// Clause 2 is a declarative Delay clause with the standard <Delay> trigger.
#[test]
fn p_235_has_delay_declarative() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-235")
        .expect("P-235 compiled card present");

    let delay = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
        )
    });

    match delay {
        Some(CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. })) => {
            assert_eq!(
                *trigger,
                CompiledTiming::Delayed,
                "Delay trigger must be the standard player-initiated <Delay>"
            );
        }
        other => panic!("expected a declarative Delay clause; got {:?}", other),
    }
}

/// Clause 3 fires at on_security with Inherited scope and places self.
#[test]
fn p_235_security_clause_inherited_places_self() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-235")
        .expect("P-235 compiled card present");

    let sec = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert_eq!(sec.scope, CompiledScope::Inherited);
    assert!(
        process_contains_place_self_as_delay_option(&sec.process),
        "Security clause must place self in battle area"
    );
}

// ─── §2 Main clause — reveal pipeline (behavioral) ────────────────────────────

/// When a DATA SQUAD card is among the top 3, the reveal pipeline adds it to the
/// hand, returns the other 2 to the deck bottom, and places P-235 in the battle
/// area.
#[test]
fn p_235_main_adds_data_squad_card_and_returns_rest_to_bottom() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_data_squad("DS1"))
        .add_card(make_non_ds("NDS1"))
        .add_card(make_non_ds("NDS2"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-235"])
        .hand(1, &["FILL"])
        // reveal_top_deck pops from the END of the deck Vec, so the array's last
        // three entries are the top 3: DS1 (top), NDS1, NDS2.
        .deck(0, &["FILL", "FILL", "NDS2", "NDS1", "DS1"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let deck_before = runner.game.players[0].deck.len();

    runner.game.activate_hand_main(0, 0);
    drain_selections(&mut runner);

    // DS1 (the only DATA SQUAD card) must be in hand.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "DS1"),
        "DS1 (the DATA SQUAD reveal pick) must be in hand"
    );
    // The reveal pool must be drained (rest returned to deck).
    assert!(
        runner.game.revealed_cards.is_empty(),
        "all revealed cards must be consumed (1 to hand, 2 to deck bottom)"
    );
    // Deck: 1 card (DS1) left for hand, 2 returned → net −1 (plus P-235 went to
    // battle area, not deck). So the deck shrinks.
    assert!(
        runner.game.players[0].deck.len() < deck_before,
        "deck must shrink by 1 (DS1 moved to hand); the other 2 return to bottom"
    );
}

/// Negative path: no DATA SQUAD card among the top 3 → nothing is added to hand,
/// all 3 are returned to the deck bottom, P-235 is still placed.
#[test]
fn p_235_main_no_data_squad_card_adds_nothing() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_non_ds("NDS1"))
        .add_card(make_non_ds("NDS2"))
        .add_card(make_non_ds("NDS3"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-235"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "NDS3", "NDS2", "NDS1"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let deck_before = runner.game.players[0].deck.len();

    runner.game.activate_hand_main(0, 0);
    drain_selections(&mut runner);

    // No non-DATA-SQUAD card may be added to hand.
    assert!(
        !runner.game.players[0].hand.iter().any(|cs| {
            let id = cs.card_id(&runner.game.card_data);
            id == "NDS1" || id == "NDS2" || id == "NDS3"
        }),
        "no non-DATA-SQUAD card may be added to hand"
    );
    // All 3 revealed cards return to the deck bottom → deck size unchanged.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "all 3 revealed cards return to the deck bottom (size unchanged)"
    );
    assert!(
        runner.game.revealed_cards.is_empty(),
        "reveal pool must be drained"
    );
}

/// "Then, place this card in the battle area" — P-235 leaves the hand and seats
/// as an OptionState::Delayed permanent.
#[test]
fn p_235_main_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_data_squad("DS1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["P-235"])
        .hand(1, &["FILL"])
        .deck(0, &["DS1", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    drain_selections(&mut runner);

    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "P-235"),
        "P-235 must leave the hand when placed in the battle area"
    );

    let placed = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-235")
        .expect("P-235 must become a battle-area permanent after [Main]");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 0, .. }),
        "P-235 must be seated as an OptionState::Delayed permanent; got {:?}",
        placed.option_state
    );
}

/// Edge: empty deck → reveal is a no-op, no panic.
#[test]
fn p_235_main_empty_deck_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["P-235"])
        .hand(1, &["FILL"])
        .deck(0, &[])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    drain_selections(&mut runner);
}

// ─── §3 Delay body — gain 2 memory ────────────────────────────────────────────

/// When P-235's <Delay> body fires, the controller gains 2 memory.
#[test]
fn p_235_delay_body_gains_two_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    // Place P-235 as a Delay permanent (as if placed by the Main clause).
    let delay_perm = runner.place_on_field(0, "P-235", Some(0));

    let mem_before = runner.game.memory;

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    drain_selections(&mut runner);

    // Memory moved +2 toward player 0. Memory is signed toward the active
    // player; assert a net +2 swing relative to the active controller.
    let delta = runner.game.memory - mem_before;
    assert_eq!(
        delta.abs(),
        2,
        "the <Delay> body must move memory by 2 (gain 2 memory); delta was {delta}"
    );
}

// ─── §4 Security clause — behavioral ──────────────────────────────────────────

/// [Security] "Place this card in the battle area." — driven through the real
/// combat / security-check path.
#[test]
fn p_235_security_places_self_in_battle_area() {
    let mut attacker = make_filler("P235-ATK");
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
        .security(1, &["P-235"])
        .start();

    let attacker_handle = runner.place_on_field(0, "P235-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: P-235 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(
        runner.security_count(1),
        0,
        "P-235 must leave the defender's security stack after the security check"
    );

    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "P-235")
        .expect("P-235 must be placed in the defender's battle area after [Security]");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 1, .. }),
        "P-235 must be seated as an OptionState::Delayed permanent owned by the defender; got {:?}",
        placed.option_state
    );

    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "P-235"),
        "P-235 must not fall through to the trash — the [Security] clause placed it"
    );
}
