//! P-236 Glowing Dawn — Option, Green, Cost 3.
//! Traits: Glowing Dawn, BEATBREAK.
//!
//! # Printed text (data/cards.json — verbatim)
//! <Use Req. ([Glowing Dawn] trait)> (Specified cards let you ignore color
//!   requirements.)
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with the [Glowing
//!   Dawn] trait among them to the hand. Return the rest to the bottom of the
//!   deck. Then, place this card in the battle area.
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//!   effect below.)
//!   ・Gain 2 memory.
//! Inherited: [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Green/P_236.cs
//!   - EffectTiming.None: UseRequirements(card, cardSource.EqualsTraits("Glowing
//!     Dawn")). P-236 IS a [Glowing Dawn] card, so this is UNCONDITIONALLY true
//!     → ignore color requirements always (authored as an unconditional
//!     IgnoreColorRequirement flood gate, no active_when).
//!   - OptionSkill: SimplifiedRevealDeckTopCardsAndSelect(3, bucket
//!     EqualsTraits("Glowing Dawn"), maxCount 1, AddHand, remaining DeckBottom);
//!     then PlaceDelayOptionCards (place self as a standard <Delay> Option).
//!   - OnDeclaration: Gain2MemoryOptionDelayEffect → standard <Delay> (manual,
//!     self-trash after the placing turn) gaining 2 memory.
//!   - SecuritySkill: PlaceSelfDelayOptionSecurityEffect.
//!   This matches BT24-100 In-Between Theater 1:1 except trait/color.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Group E: reveal-N single pick (add 1 [Glowing Dawn]; rest to deck BOTTOM)
//! - Option pipeline: [Main] places self as a standard <Delay> battle-area
//!   Option ("Then, place this card in the battle area")
//! - Standard <Delay> (DelayTrigger::MainPhaseActivated): self-trash cost →
//!   gain 2 memory, after the placing turn
//! - Inherited [Security] places self in battle area
//! - Unconditional IgnoreColorRequirement (self-trait <Use Req.>)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "P-236";

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-236 YAML parses and compiles")
        .memory(10)
        .start()
}

/// Stack the given card IDs onto player 0's deck top. Last id in the slice ends
/// up on top of the deck (pushed last).
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

fn gd_card(id: &str) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn plain_card(id: &str) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == card_id)
    })
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn p_236_is_green_glowing_dawn_option_cost_3() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Glowing Dawn");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
}

#[test]
fn p_236_has_ignore_color_requirement_flood_gate() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier == "IgnoreColorRequirement"
        )),
        "self-trait <Use Req.> ⇒ unconditional IgnoreColorRequirement flood gate present"
    );
}

#[test]
fn p_236_main_reveals_three_adds_glowing_dawn_bottoms_rest_places_self() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::MainFromHand)
                    && t.scope != CompiledScope::Inherited =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main] reveal clause");

    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })));
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectReveal { .. })));
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. })));
    // "Return the rest to the BOTTOM of the deck" (NOT trash).
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceRemainderOnDeck { .. })),
        "the rest return to the deck bottom (place_remainder_on_deck)"
    );
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn p_236_has_standard_delay_clause_gaining_two_memory() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let delay = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("standard <Delay> clause present");

    assert_eq!(
        *delay.0,
        CompiledTiming::Delayed,
        "the <Delay> is a standard manual delay (trash after the placing turn)"
    );
    assert!(
        delay
            .1
            .iter()
            .any(|s| matches!(s, CompiledStep::GainMemory { .. })),
        "the <Delay> body gains memory"
    );
}

#[test]
fn p_236_security_places_self_in_battle_area() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let security = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("[Security] clause");

    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(security
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
}

// ── Section 2: Behavioral — [Main] reveal/add/bottom/place ──────────────────

#[test]
fn p_236_main_adds_one_glowing_dawn_bottoms_the_other_two_then_parks_as_delay() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-236 YAML loads")
        .add_card(gd_card("GD-A"))
        .add_card(plain_card("PLAIN-B"))
        .add_card(plain_card("PLAIN-C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PLAIN-B"; 3])
        .deck(1, &["PLAIN-B"; 3])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    // Top-3 (last pushed = top): PLAIN-C, PLAIN-B, GD-A.
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "GD-A"]);
    let deck_before = runner.game.players[0].deck.len();

    // The [Main] reveal/select parks the Option pipeline at the reveal pick.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the [Main] reveal selection parks the Option pipeline"
    );

    let view = runner
        .pending_selection_view()
        .expect("reveal add-to-hand selection pending");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the lone [Glowing Dawn] card (GD-A) is eligible to add"
    );
    let add = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete add action");
    runner
        .execute_action(view.selecting_player, add)
        .expect("add the lone Glowing Dawn to hand");
    runner
        .auto_resolve()
        .expect("resolve return-rest-to-bottom + battle-area placement");

    // GD-A went to hand; the two non-matching cards returned to the deck bottom
    // (NOT trashed).
    assert!(
        hand_ids(&runner, 0).contains(&"GD-A".to_string()),
        "the [Glowing Dawn] card is added to hand: {:?}",
        hand_ids(&runner, 0)
    );
    assert!(
        trash_ids(&runner, 0).is_empty(),
        "the rest must go to the deck BOTTOM, not the trash: {:?}",
        trash_ids(&runner, 0)
    );
    // 3 revealed, 1 added to hand, 2 returned to deck bottom: net deck change
    // is -1 vs the post-stack count.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 1,
        "one card added to hand, two returned to deck bottom"
    );

    // "Then, place this card in the battle area" — parked as a standard <Delay>.
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("P-236 placed in the battle area after the [Main] body");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

/// Negative for the reveal pick: when NO [Glowing Dawn] is among the top three,
/// the add-pick has no eligible candidate, so nothing is added and all three
/// return to the deck bottom; self still places.
#[test]
fn p_236_main_with_no_glowing_dawn_revealed_bottoms_all_three_and_still_places() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-236 YAML loads")
        .add_card(plain_card("PLAIN-A"))
        .add_card(plain_card("PLAIN-B"))
        .add_card(plain_card("PLAIN-C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PLAIN-A"; 3])
        .deck(1, &["PLAIN-A"; 3])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "PLAIN-A"]);
    let deck_before = runner.game.players[0].deck.len();

    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(result, OptionPlayResult::Pending) {
        let _ = runner.auto_resolve();
    }

    assert!(
        !hand_ids(&runner, 0)
            .iter()
            .any(|id| id.starts_with("PLAIN")),
        "no non-Glowing-Dawn card is ever added to hand: {:?}",
        hand_ids(&runner, 0)
    );
    assert!(
        trash_ids(&runner, 0).is_empty(),
        "the rest go to the deck bottom, not trash: {:?}",
        trash_ids(&runner, 0)
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "all three returned to the deck bottom (none added to hand)"
    );
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "self is placed in the battle area even when nothing is added"
    );
}

// ── Section 3: Behavioral — <Delay> gain 2 memory ───────────────────────────

/// Seat P-236 as a standard <Delay> Option placed on the current turn, then
/// advance past the placing turn so the <Delay> [Main] activation is legal.
fn seat_and_advance_to_activatable_delay(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    handle
}

/// The standard <Delay> can only be activated after the placing turn. Once
/// active, activating it trashes P-236 (the self-trash cost) and gains 2 memory.
#[test]
fn p_236_delay_after_placing_turn_gains_two_memory_and_self_trashes() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-236 YAML loads")
        .add_card(plain_card("FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(5);
    let memory_before = runner.memory();

    assert!(
        runner.game.activate_field_main(0, handle.index as usize),
        "the standard <Delay> [Main] is activatable after the placing turn"
    );
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "the <Delay> gains 2 memory"
    );
    assert!(
        trash_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "the <Delay> cost trashes P-236: {:?}",
        trash_ids(&runner, 0)
    );
    assert!(
        !field_contains(&runner, 0, CARD_ID),
        "P-236 leaves the battle area when its <Delay> activates"
    );
}

// ── Section 4: Security behavioral ───────────────────────────────────────────

/// When P-236 is revealed as a security card, its [Security] effect places it
/// in the controller's battle area as a <Delay> Option.
#[test]
fn p_236_security_places_self_in_battle_area_when_revealed() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Green];
    attacker.level = Some(5);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-236 YAML loads")
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(10)
        .start();

    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        placed.is_some(),
        "P-236 must be placed in P1's battle area after the [Security] effect fires"
    );
    if let Some(perm) = placed {
        assert!(
            matches!(perm.option_state, OptionState::Delayed { .. }),
            "P-236 placed via [Security] must be a Delayed Option"
        );
    }
}
