//! EX7-044 Gigadramon — Digimon, Lv.5, Black, DP 7000, Cost 7.
//! Traits: Cyborg. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX7-044.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-044.webp)
//!
//! [On Play] [When Digivolving] Reveal the top 4 cards of your deck. Place 1
//! Option card with the [Three Musketeers] trait among them as this
//! Digimon's bottom digivolution card. Return the rest to the top or bottom
//! of the deck. If this effect placed, delete 1 of your opponent's Digimon or
//! Tamers with a play cost of 3 or less.
//!
//! Inherited Effect: ＜Collision＞ (During this Digimon's attack, give all of
//! your opponent's Digimon ＜Blocker＞, and the opponent blocks if able.)
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Black Lv.4 / cost 3
//!   - xros_req: "[Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_044.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: `IsLevel4 && HasText("Three Musketeers")`, cost 3.
//! - OP and WD: two ActivateClass entries (isOptional FALSE).
//!   SimplifiedRevealDeckTopCardsAndSelect(revealCount 4, ONE Mode.Custom
//!   slot filtering `IsOption && ContainsTraits("Three Musketeers")`,
//!   maxCount 1) → SelectCardCoroutine: AddDigivolutionCardsBottom + set
//!   `cardWasPlaced`; remainder RemainingCardsPlace.DeckTopOrBottom (single
//!   player-elected choice, fires whether or not a card was placed). Then
//!   `if (cardWasPlaced && HasMatchConditionPermanent(opp Digimon OR Tamer
//!   with HasPlayCost && GetCostItself <= 3))` mandatory
//!   SelectPermanentEffect(Destroy).
//! - ESS: CollisionSelfStaticEffect(isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A2/A6: reveal 4, mandatory single pick placed as own bottom source
//! - §3.1 StackPosition::Choice remainder (top OR bottom)
//! - "If this effect placed" gate → play-cost-capped Digimon/Tamer deletion
//! - H: inherited <Collision>

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-044";
/// EX7-070 Der Blitz — a REAL [Three Musketeers]-trait Option.
const DER_BLITZ: &str = "EX7-070";

fn digimon(id: &str, level: u8, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = cost;
    c
}

fn tamer(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Blue];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c
}

fn option(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Black];
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn sources_of(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-044 YAML loads from the embedded pack")
        .dsl_card(DER_BLITZ)
        .expect("EX7-070 YAML loads from the embedded pack")
        .add_card(option("PLAIN-OPT", &[]))
        .add_card(digimon("TM-DIGI", 4, CardColor::Black, 4))
        .add_card(digimon("BLK-LV4", 4, CardColor::Black, 4))
        .add_card(digimon("OPP-C3", 3, CardColor::Blue, 3))
        .add_card(digimon("OPP-C5", 5, CardColor::Blue, 5))
        .add_card(tamer("TAMER-C3", 3))
        .add_card(tamer("TAMER-C4", 4))
        .add_card(filler("F1"))
        .add_card(filler("F2"))
        .add_card(filler("F3"))
        .add_card(filler("F4"))
        .add_card(filler("F5"))
        .add_card(digimon("CARRIER", 6, CardColor::Black, 12))
        .memory(10)
}

/// `place_remainder_on_deck { position: choice }` follows the top/bottom
/// pick with an `OrderedPermutation` prompt for the returned cards (Working
/// Rule §17: the order is player-visible). Drain it by taking the first legal
/// pick each time — the tests here assert WHICH end the rest went to, not the
/// order among them.
fn drain_ordering(runner: &mut DebugRunner) {
    while matches!(runner.pending_kind(), Some(SelectionKind::OrderedPermutation { .. })) {
        let v = runner.pending_selection_view().unwrap();
        runner
            .execute_action(v.selecting_player, v.valid_action_ids[0])
            .expect("order pick");
    }
}

fn reveal_action(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not revealed"));
    SEL_REVEAL_START + idx as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_044_structure_matches_printed_text() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]);
    assert!(!triggered[0].optional, "the reveal is mandatory (no printed 'may')");

    let collision = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. })
                if *scope == CompiledScope::Inherited && keyword == "Collision"
        )
    });
    assert!(collision, "inherited <Collision>");
    let paths = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(paths, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — [On Play] reveal → place → remainder → conditional delete
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_044_on_play_places_tm_option_returns_rest_to_top_and_deletes_cheap_target() {
    // Deck (last = top): F5 | F1, F2, DER_BLITZ, F3 ← top 4.
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F5", "F1", "F2", DER_BLITZ, "F3"])
        .start();
    runner.place_on_field(1, "OPP-C3", Some(0));
    runner.place_on_field(1, "OPP-C5", Some(0));
    runner.place_on_field(1, "TAMER-C4", Some(0));
    let deck_before = runner.deck_size(0);
    let giga_idx = runner.play(0, 0).expect("play Gigadramon");
    let giga = PermanentHandle { player: 0, index: giga_idx as u8 };

    let view = runner.pending_selection_view().expect("reveal pick prompt");
    assert_eq!(view.kind, SelectionKind::Reveal);
    assert!(!view.valid_action_ids.contains(&PASS), "placing the Option is mandatory when one is revealed");
    assert_eq!(view.valid_action_ids.len(), 1, "only Der Blitz qualifies");
    runner
        .execute_action(0, reveal_action(&runner, DER_BLITZ))
        .expect("pick Der Blitz");

    let view = runner.pending_selection_view().expect("top/bottom choice for the remainder");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("return the rest to the TOP");
    drain_ordering(&mut runner);

    let view = runner.pending_selection_view().expect("delete target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.valid_action_ids.contains(&PASS));
    assert_eq!(view.valid_action_ids.len(), 1, "only OPP-C3 (cost 3) qualifies; cost-5 Digimon and cost-4 Tamer excluded");
    runner.execute_action(0, view.valid_action_ids[0]).expect("delete OPP-C3");
    runner.auto_resolve().expect("finish");

    assert_eq!(sources_of(&runner, giga), 2, "Der Blitz placed as Gigadramon's bottom source");
    let bottom = runner.game.players[0].battle_area[giga.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert_eq!(bottom, DER_BLITZ);
    assert_eq!(runner.deck_size(0), deck_before - 1, "3 of the 4 revealed returned");
    let deck = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck[0], "F5", "the pre-existing bottom card is still at the bottom (rest went to the TOP)");
    assert!(runner.game.revealed_cards.is_empty());
    assert_eq!(runner.battle_area_size(1), 2, "OPP-C3 deleted");
    let survivors = zone_ids(
        &runner.game.players[1].battle_area.iter().map(|p| p.top_card().clone()).collect::<Vec<_>>(),
        &runner.game.card_data,
    );
    assert!(!survivors.contains(&"OPP-C3".to_string()));
}

#[test]
fn ex7_044_on_play_can_delete_a_cheap_tamer() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F5", "F1", "F2", DER_BLITZ, "F3"])
        .start();
    runner.place_on_field(1, "TAMER-C3", Some(0));
    runner.play(0, 0).expect("play Gigadramon");
    runner
        .execute_action(0, reveal_action(&runner, DER_BLITZ))
        .expect("pick Der Blitz");
    runner.execute_branch(1).expect("return the rest to the BOTTOM");
    drain_ordering(&mut runner);
    let view = runner.pending_selection_view().expect("delete target prompt");
    assert_eq!(view.valid_action_ids.len(), 1, "a cost-3 Tamer is a legal target");
    runner.execute_action(0, view.valid_action_ids[0]).expect("delete TAMER-C3");
    runner.auto_resolve().expect("finish");
    assert_eq!(runner.battle_area_size(1), 0);
    let deck = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck.last().map(String::as_str), Some("F5"), "rest went to the BOTTOM: F5 is now the top");
}

#[test]
fn ex7_044_on_play_no_tm_option_revealed_still_places_remainder_but_no_delete() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F5", "F1", "PLAIN-OPT", "TM-DIGI", "F3"])
        .start();
    runner.place_on_field(1, "OPP-C3", Some(0));
    let deck_before = runner.deck_size(0);
    let giga_idx = runner.play(0, 0).expect("play Gigadramon");
    let giga = PermanentHandle { player: 0, index: giga_idx as u8 };

    let view = runner.pending_selection_view().expect("top/bottom choice for the remainder");
    assert_eq!(view.kind, SelectionKind::EffectChoice, "no qualifying Option → straight to the remainder choice");
    runner.execute_branch(0).expect("top");
    drain_ordering(&mut runner);
    assert!(runner.pending_selection().is_none(), "'If this effect placed' is false → no delete prompt");
    assert_eq!(sources_of(&runner, giga), 1);
    assert_eq!(runner.deck_size(0), deck_before, "all 4 returned");
    assert_eq!(runner.battle_area_size(1), 1, "OPP-C3 survives");
}

#[test]
fn ex7_044_on_play_placed_but_no_cheap_target_skips_delete() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F5", "F1", "F2", DER_BLITZ, "F3"])
        .start();
    runner.place_on_field(1, "OPP-C5", Some(0));
    runner.place_on_field(1, "TAMER-C4", Some(0));
    let giga_idx = runner.play(0, 0).expect("play Gigadramon");
    let giga = PermanentHandle { player: 0, index: giga_idx as u8 };
    runner
        .execute_action(0, reveal_action(&runner, DER_BLITZ))
        .expect("pick Der Blitz");
    runner.execute_branch(0).expect("top");
    drain_ordering(&mut runner);
    assert!(runner.pending_selection().is_none(), "no opponent permanent with play cost <= 3");
    assert_eq!(sources_of(&runner, giga), 2, "the Option was still placed");
    assert_eq!(runner.battle_area_size(1), 2);
}

#[test]
fn ex7_044_when_digivolving_runs_the_same_effect() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F5", "F1", "F2", DER_BLITZ, "F3"])
        .start();
    let base_perm = runner.place_on_field(0, "BLK-LV4", Some(0));
    runner.place_on_field(1, "OPP-C3", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 3, "Black Lv.4 / cost 3");
    runner
        .execute_action(0, reveal_action(&runner, DER_BLITZ))
        .expect("pick Der Blitz");
    runner.execute_branch(0).expect("top");
    drain_ordering(&mut runner);
    let view = runner.pending_selection_view().expect("delete target prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("delete");
    runner.auto_resolve().expect("finish");
    assert_eq!(sources_of(&runner, base_perm), 3, "BLK-LV4 + Der Blitz + Gigadramon");
    assert_eq!(runner.battle_area_size(1), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited <Collision> + digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_044_inherited_collision_grants_keyword_to_carrier() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(stack, Keyword::Collision));
    let alone = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner.game.has_keyword(alone, Keyword::Collision), "inherited only");
}

#[test]
fn ex7_044_digivolves_from_red_lv4_with_tm_in_text_for_three() {
    let mut red = digimon("RED-TM-LV4", 4, CardColor::Red, 4);
    red.effect_text = "... [Three Musketeers] ...".to_string();
    let mut runner = base()
        .add_card(red)
        .hand(0, &[CARD_ID])
        .deck(0, &["F1", "F2", "F3", "F4"])
        .start();
    let base_perm = runner.place_on_field(0, "RED-TM-LV4", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 3);
    runner.auto_resolve().expect("resolve the WD reveal");
}

#[test]
fn ex7_044_cannot_digivolve_from_plain_red_lv4() {
    let mut runner = base()
        .add_card(digimon("RED-LV4", 4, CardColor::Red, 4))
        .hand(0, &[CARD_ID])
        .start();
    let base_perm = runner.place_on_field(0, "RED-LV4", Some(0));
    assert!(!runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
}
