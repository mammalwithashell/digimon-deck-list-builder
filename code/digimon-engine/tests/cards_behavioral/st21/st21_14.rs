//! ST21-14 Believe in Our Friendship — Option, Purple, Cost 3,
//! traits: [ADVENTURE].
//!
//! # Card text (cards.json / official Bandai DB bundle)
//!
//! While you have an [ADVENTURE] trait Digimon or Tamer on the field, you can
//! ignore this card's color requirements.
//!
//! **[Main]** Reveal the top 3 cards of your deck. Add 1 card with the
//! [ADVENTURE] trait among them to the hand. Return the rest to the bottom of
//! the deck. Then, place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn,
//! activate the effect below.)
//! ・Gain 2 memory.
//!
//! **Inherited (Security):** [Security] Place this card in the battle area.
//!
//! Official Q&A: "on the field" covers the battle area and the breeding area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_14.cs`
//!
//! # Patterns this test covers
//!
//! - **D3 colour bypass via card-level `use_requirement`** (+ the DCGO-mirroring
//!   `flood_gate`): positive for an ADVENTURE Digimon / Tamer / breeding
//!   Digimon; negatives for no ADVENTURE card, non-ADVENTURE Digimon, and an
//!   OPPONENT's ADVENTURE card (printed "you have"; DCGO's helper is
//!   owner-blind — deliberate divergence, see the YAML header).
//! - **A-group reveal-3 / single bucket / bottom remainder** (BT22-099 and
//!   BT24-100 siblings): picks an ADVENTURE card, picks at most one, picks a
//!   non-Digimon ADVENTURE card, and resolves silently when none is revealed.
//! - **Place self as a Delay Option** after the main effect.
//! - **Standard ＜Delay＞** (`trigger: delayed`): activation barred on the
//!   placing turn, available afterwards, gains 2 memory and trashes the Option.
//! - **Inherited [Security] place-self-as-Delay**, end-to-end through a
//!   security check.

#![allow(dead_code, unused_imports, unused_variables)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, SEL_REVEAL_START};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-14";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// An ADVENTURE-trait Digimon (red, so it never satisfies the purple colour
/// requirement by itself).
fn adventure_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn adventure_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec![];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec![];
    c
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

/// Colour-bypass playability of the hand Option at index 0 for player 0.
fn playable(runner: &DebugRunner) -> f32 {
    build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize]
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st21_14_compiles_as_purple_option_cost_3_with_adventure_trait() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Believe in Our Friendship");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    assert_eq!(card.cost, Some(3), "ST21-14 prints Cost 3");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST21-14 must carry the ADVENTURE trait"
    );
    assert!(
        card.use_requirement.is_some(),
        "colour bypass must be carried by the card-level use_requirement"
    );
}

#[test]
fn st21_14_has_four_clauses_in_expected_order() {
    let card = compiled(CARD_ID);
    assert_eq!(
        card.effects.len(),
        4,
        "flood_gate + main_from_hand + delay + inherited on_security"
    );

    assert!(matches!(
        &card.effects[0],
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    ));

    let CompiledClause::Triggered(main) = &card.effects[1] else {
        panic!("clause 1 must be the triggered [Main]");
    };
    assert_eq!(main.when, vec![CompiledTiming::MainFromHand]);
    assert!(!main.optional, "[Main] has no 'may': mandatory");
    assert!(
        matches!(
            main.process.as_slice(),
            [
                CompiledStep::RevealTopDeck { count: 3, .. },
                CompiledStep::SelectRevealBuckets { .. },
                CompiledStep::AddToHandFromReveal { .. },
                CompiledStep::PlaceRemainderOnDeck {
                    position: CompiledStackPosition::Bottom,
                    ..
                },
                CompiledStep::PlaceSelfAsDelayOption,
            ]
        ),
        "[Main] body shape; got {:?}",
        main.process
    );

    assert!(matches!(
        &card.effects[2],
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger: CompiledTiming::Delayed,
            process,
            ..
        }) if process == &vec![CompiledStep::GainMemory(2)]
    ));

    assert!(matches!(
        &card.effects[3],
        CompiledClause::Triggered(security)
            if security.scope == CompiledScope::Inherited
                && security.when == vec![CompiledTiming::OnSecurity]
                && !security.optional
                && security.process == vec![CompiledStep::PlaceSelfAsDelayOption]
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Colour-requirement bypass
// ═══════════════════════════════════════════════════════════════════════════════

/// Baseline: with nothing on the field, the purple Option is not playable.
#[test]
fn st21_14_not_playable_without_adventure_card_or_purple_source() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    assert_eq!(
        playable(&runner),
        0.0,
        "purple Option must be unplayable with no colour source and no ADVENTURE card"
    );
}

#[test]
fn st21_14_playable_with_adventure_digimon_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(adventure_digimon("ADV-DIGI"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "ADV-DIGI", Some(0));

    assert_eq!(
        playable(&runner),
        1.0,
        "an ADVENTURE Digimon on the field must bypass the purple requirement"
    );
}

#[test]
fn st21_14_playable_with_adventure_tamer_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(adventure_tamer("ADV-TAMER"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "ADV-TAMER", Some(0));

    assert_eq!(
        playable(&runner),
        1.0,
        "an ADVENTURE Tamer on the field must bypass the purple requirement"
    );
}

/// Official Q&A: "on the field" includes the breeding area.
#[test]
fn st21_14_playable_with_adventure_digimon_in_breeding_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(adventure_digimon("ADV-DIGI"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "ADV-DIGI");

    assert_eq!(
        playable(&runner),
        1.0,
        "an ADVENTURE Digimon in the breeding area counts as 'on the field' (official Q&A)"
    );
}

#[test]
fn st21_14_not_playable_with_non_adventure_digimon_or_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(plain_digimon("PLAIN-DIGI"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));

    assert_eq!(
        playable(&runner),
        0.0,
        "non-ADVENTURE permanents must not bypass the colour requirement"
    );
}

/// Printed "While YOU have": the opponent's ADVENTURE permanents don't count.
#[test]
fn st21_14_not_playable_when_only_opponent_has_adventure_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(adventure_digimon("OPP-ADV"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-ADV", Some(0));

    assert_eq!(
        playable(&runner),
        0.0,
        "an opponent's ADVENTURE Digimon must not satisfy 'while you have'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] reveal 3, add 1 ADVENTURE, bottom the rest, place self
// ═══════════════════════════════════════════════════════════════════════════════

/// A runner with ST21-14 in hand and an ADVENTURE Digimon on the field
/// (colour bypass active). `deck0` is bottom-to-top (last = top).
fn main_runner(extra: Vec<CardData>, deck0: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(adventure_digimon("BOARD-ADV"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"));
    for card in extra {
        b = b.add_card(card);
    }
    let mut runner = b
        .hand(0, &[CARD_ID])
        .deck(0, deck0)
        .deck(1, &["FILL-A", "FILL-B", "FILL-C"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BOARD-ADV", Some(0));
    runner
}

/// Positive: top 3 = [ADV-HIT, FILL-A, FILL-B] → the player picks ADV-HIT, the
/// two fillers go to the deck bottom, memory pays the Option's cost 3, and the
/// Option is placed in the battle area as a Delay Option.
#[test]
fn st21_14_main_adds_adventure_card_bottoms_rest_and_places_self() {
    let mut runner = main_runner(
        vec![adventure_digimon("ADV-HIT")],
        &["FILL-C", "FILL-B", "FILL-A", "ADV-HIT"],
    );
    let mem_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let action = revealed_action_for_id(&runner, "ADV-HIT");
    let view = runner
        .pending_selection_view()
        .expect("[ADVENTURE] bucket prompt");
    assert!(view.valid_action_ids.contains(&action));
    assert!(
        !runner.pending_is_optional(),
        "'Add 1 card' is mandatory when an ADVENTURE card is revealed"
    );
    runner
        .execute_action(0, action)
        .expect("pick the ADVENTURE card");
    runner.auto_resolve().expect("finish ST21-14 main");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"ADV-HIT".to_string()), "hand: {hand:?}");
    assert!(
        !hand.iter().any(|id| id == "FILL-A" || id == "FILL-B"),
        "the non-ADVENTURE reveals must not enter the hand; hand: {hand:?}"
    );
    // Deck: 4 cards - 3 revealed + 2 returned to the bottom = 3.
    assert_eq!(runner.deck_size(0), 3, "two reveals return to the deck");
    let deck = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert!(
        deck[..2].contains(&"FILL-A".to_string()) && deck[..2].contains(&"FILL-B".to_string()),
        "the returned cards sit at the deck BOTTOM (front of storage); deck: {deck:?}"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3,
        "using the Option costs its printed 3 memory"
    );

    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("ST21-14 must be placed in the battle area");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

/// At most ONE ADVENTURE card is added even if several are revealed; the
/// unchosen ADVENTURE card is returned to the bottom of the deck.
#[test]
fn st21_14_main_adds_only_one_when_several_adventure_cards_are_revealed() {
    let mut runner = main_runner(
        vec![adventure_digimon("ADV-ONE"), adventure_digimon("ADV-TWO")],
        &["FILL-C", "FILL-A", "ADV-TWO", "ADV-ONE"],
    );

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let action = revealed_action_for_id(&runner, "ADV-ONE");
    runner.execute_action(0, action).expect("pick ADV-ONE");
    runner.auto_resolve().expect("finish ST21-14 main");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"ADV-ONE".to_string()));
    assert!(
        !hand.contains(&"ADV-TWO".to_string()),
        "only 1 card may be added; hand: {hand:?}"
    );
    let deck = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert!(
        deck.contains(&"ADV-TWO".to_string()),
        "the other ADVENTURE card returns to the deck; deck: {deck:?}"
    );
}

/// "1 card with the [ADVENTURE] trait" is not limited to Digimon: an
/// ADVENTURE Tamer in the reveal is a legal pick, a plain Digimon is not.
#[test]
fn st21_14_main_can_add_a_non_digimon_adventure_card_and_rejects_plain_cards() {
    let mut runner = main_runner(
        vec![adventure_tamer("ADV-TAMER"), plain_digimon("PLAIN-DIGI")],
        &["FILL-C", "PLAIN-DIGI", "FILL-A", "ADV-TAMER"],
    );

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let view = runner.pending_selection_view().expect("bucket prompt");
    let tamer_action = revealed_action_for_id(&runner, "ADV-TAMER");
    assert!(view.valid_action_ids.contains(&tamer_action));
    assert!(
        !view
            .valid_action_ids
            .contains(&revealed_action_for_id(&runner, "PLAIN-DIGI")),
        "a card without the ADVENTURE trait is not a legal pick"
    );
    runner
        .execute_action(0, tamer_action)
        .expect("pick the ADVENTURE Tamer");
    runner.auto_resolve().expect("finish ST21-14 main");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"ADV-TAMER".to_string()), "hand: {hand:?}");
}

/// Negative: no ADVENTURE card among the top 3 → nothing is added and all 3
/// return to the bottom; the Option is still placed.
#[test]
fn st21_14_main_with_no_adventure_card_bottoms_all_three_and_still_places_self() {
    let mut runner = main_runner(
        vec![plain_digimon("PLAIN-A")],
        &["FILL-C", "PLAIN-A", "FILL-A", "FILL-B"],
    );
    let deck_before = runner.deck_size(0);

    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand.is_empty(),
        "no card may be added when no ADVENTURE card is revealed; hand: {hand:?}"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all three revealed cards return to the deck"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "the Option is still placed in the battle area"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Main] <Delay> gain 2 memory
// ═══════════════════════════════════════════════════════════════════════════════

fn placed_delay_handle(runner: &DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .map(|index| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: index as u8,
        })
        .expect("ST21-14 waits on the field as a Delay Option")
}

/// The Delay is barred on the placing turn and available (and costs the
/// Option itself) on a later [Main]; activating it gains 2 memory.
#[test]
fn st21_14_delay_is_barred_on_placing_turn_then_gains_2_memory_and_trashes_self() {
    let mut runner = main_runner(
        vec![adventure_digimon("ADV-HIT")],
        &["FILL-C", "FILL-B", "FILL-A", "ADV-HIT"],
    );
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let action = revealed_action_for_id(&runner, "ADV-HIT");
    runner.execute_action(0, action).expect("pick ADV-HIT");
    runner.auto_resolve().expect("place ST21-14 as delay");

    // Placing turn: the Delay cannot be activated yet (§16-17-3).
    let handle = placed_delay_handle(&runner);
    assert!(
        !runner.game.activate_delayed_option_main(handle),
        "the Delay must not be activatable on the turn it was placed"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "a barred activation leaves the Option on the field"
    );

    let start_turn = runner.turn_count();
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.turn_count(), start_turn + 2);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    let handle = placed_delay_handle(&runner);
    let cp = runner.event_checkpoint();
    assert!(
        runner.game.activate_delayed_option_main(handle),
        "the [Main] <Delay> activation must be available after the placing turn"
    );
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 2, "the Delay gains 2 memory");
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "the Option is trashed as the Delay's cost"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "ST21-14 ends up in the trash"
    );
    let gains: Vec<i16> = runner
        .events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange { delta, .. } => Some(*delta),
            _ => None,
        })
        .collect();
    assert!(
        gains.contains(&2),
        "a +2 MemoryChange must be logged for the Delay; got {gains:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [Security] place this card in the battle area
// ═══════════════════════════════════════════════════════════════════════════════

/// End-to-end: P0 attacks and flips ST21-14 as P1's security card → it is
/// placed in P1's battle area as a Delay Option (no memory spent).
#[test]
fn st21_14_security_effect_places_self_in_battle_area_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-14 in embedded DSL pack")
        .add_card(filler("ATTACKER"))
        .security(1, &[CARD_ID])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let mem_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("ST21-14 must be placed in P1's battle area by its [Security] effect");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
    assert_eq!(runner.memory(), mem_before, "the [Security] placement is free");
}

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = adventure_tamer("X");
    let _ = plain_tamer("X");
    let _ = PASS;
}
