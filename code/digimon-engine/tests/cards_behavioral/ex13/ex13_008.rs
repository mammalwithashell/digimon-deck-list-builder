//! EX13-008 Dracomon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Dragon. Form: Rookie. Attribute: Data.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-008.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-008.md` agrees)
//!
//! ```text
//! [Digivolve] [Bebydomon]: Cost 0        (Red Lv.2 / cost 0 circle)
//!
//! Effect:
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Add 1 card
//! with [Dracomon] or [Examon] in its text among them to the hand. Return the
//! rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [End of Your Turn] This Digimon and any of your other Digimon may DNA
//! digivolve into a Digimon card in the hand.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_008.cs
//! - `AddSelfDigivolutionRequirementStaticEffect(EqualsCardName("Bebydomon"),
//!   cost 0, ignoreDigivolutionRequirement: false)`.
//! - Shared [When Moving]/[On Play] (`optional: false`):
//!   `SimplifiedRevealDeckTopCardsAndSelect(3, HasText("Dracomon") ||
//!   HasText("Examon"), AddHand, maxCount 1, RemainingCardsPlace.DeckBottom)`.
//! - Inherited `OnEndTurn` (skippable, IsOwnerTurn):
//!   `DNADigivolvePermanentsIntoHandOrTrashCard(CanJogressFromTargetPermanent,
//!   payCost: true, isHand: true, permanentConditions: [this])`.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A1 reveal-N single-pick to hand ("[X] in its text", broad HasText scan),
//!   remainder to deck bottom (player-ordered).
//! - Shared [When Moving] + [On Play] trigger (on_move self-gated).
//! - G2 inherited [End of Your Turn] may-DNA-digivolve (pays printed DNA cost,
//!   requirements enforced) — BT17-007 idiom.
//! - Named alt digivolve ([Bebydomon] / cost 0).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-008";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn card(id: &str, name: &str, level: u8, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.effect_text = text.to_string();
    c
}

fn red_req(level: u8) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![CardColor::Red],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

fn dna_result(id: &str, memory_cost: u8) -> CardData {
    let mut c = card(id, id, 5, "");
    c.card_kind = CardKind::Digimon;
    c.dp = Some(7000);
    c.play_cost = 9;
    c.dna_costs = vec![DnaCost {
        requirement1: red_req(4),
        requirement2: red_req(4),
        memory_cost: memory_cost.into(),
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut blue_partner = card("BLUE-L4", "Blue Four", 4, "");
    blue_partner.colors = vec![CardColor::Blue];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-008 YAML parses, compiles and is in the embedded pack")
        // Reveal pool candidates.
        .add_card(card("EXAMON-TEXT", "Text Holder", 4, "Digivolve into [Examon] cheaply."))
        .add_card(card("DRACO-NAME", "Dracomon Variant", 3, ""))
        .add_card(card("PLAIN-A", "Plain A", 3, ""))
        .add_card(card("PLAIN-B", "Plain B", 3, ""))
        .add_card(card("PLAIN-C", "Plain C", 3, ""))
        .add_card(card("FILL", "Fill", 3, ""))
        // DNA fixtures.
        .add_card(card("CARRIER-L4", "Carrier Four", 4, ""))
        .add_card(card("PARTNER-L4", "Partner Four", 4, ""))
        .add_card(blue_partner)
        .add_card(dna_result("DNA-RESULT", 2))
        .add_card(card("BEBYDOMON", "Bebydomon", 2, ""))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn deck_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_index(runner: &DebugRunner, player: u8, id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player {player}'s hand"))
}

/// Push `ids` onto the top of player 0's deck (the LAST id ends on top).
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

fn revealed_ids(runner: &DebugRunner) -> Vec<String> {
    runner
        .game
        .revealed_cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Pick `id` out of the reveal pool, then place the remainder (any order).
fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("reveal pick prompt");
    let pos = revealed_ids(runner)
        .iter()
        .position(|r| r == id)
        .unwrap_or_else(|| panic!("{id} must be revealed"));
    let action = SEL_REVEAL_START + pos as u16;
    assert!(
        view.valid_action_ids.contains(&action),
        "{id} must be a legal pick; view={view:?}"
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("pick revealed card");
    finish_remainder(runner);
}

fn finish_remainder(runner: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        assert!(
            matches!(view.kind, SelectionKind::OrderedPermutation { .. }),
            "only the remainder ordering may follow the pick; got {view:?}"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("order remainder");
    }
}

fn setup_play(deck_top: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    stack_deck_top(&mut runner, deck_top);
    runner
}

fn inherited_eot_clause(runner: &DebugRunner) -> CompiledTriggeredClause {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t.clone()),
            _ => None,
        })
        .expect("inherited EoT clause")
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_008_has_bebydomon_cost0_alt_digivolve() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();
    assert_eq!(paths.len(), 2, "Red Lv.2 circle + [Bebydomon] alt digivolve");
}

#[test]
fn ex13_008_shared_clause_is_mandatory_when_moving_and_on_play() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let face: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(face.len(), 1, "one face-up triggered clause");
    let clause = face[0];
    assert!(clause.when.contains(&CompiledTiming::OnPlay));
    assert!(clause.when.contains(&CompiledTiming::OnMove));
    assert!(!clause.optional, "reveal is mandatory (DCGO optional: false)");
    assert!(!clause.once_per_turn);
}

#[test]
fn ex13_008_inherited_eot_clause_is_optional_dna_with_requirements() {
    let runner = builder().start();
    let clause = inherited_eot_clause(&runner);
    assert_eq!(clause.when, vec![CompiledTiming::EndOfYourTurn]);
    assert!(clause.optional, "printed \"may\"");
    assert!(!clause.once_per_turn);
    let dna = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements,
                ..
            } => Some(*ignore_requirements),
            _ => None,
        })
        .expect("MayDnaDigivolveNow step");
    assert!(!dna, "normal DNA: printed requirements + cost apply (DCGO payCost: true)");
}

// ─── Section 3 — [On Play] reveal ────────────────────────────────────────────

#[test]
fn ex13_008_on_play_adds_examon_text_card_and_bottoms_the_rest() {
    let mut runner = setup_play(&["PLAIN-A", "EXAMON-TEXT", "PLAIN-B"]);
    runner.skip_mulligan();
    let slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, slot).expect("Dracomon plays");

    let mut revealed = revealed_ids(&runner);
    revealed.sort();
    assert_eq!(revealed, vec!["EXAMON-TEXT", "PLAIN-A", "PLAIN-B"], "top 3 revealed");
    pick_revealed(&mut runner, "EXAMON-TEXT");

    assert!(hand_ids(&runner, 0).contains(&"EXAMON-TEXT".to_string()));
    let deck = deck_ids(&runner, 0);
    assert_eq!(deck.len(), 5, "3 fillers + 2 returned");
    let mut bottom: Vec<String> = deck[..2].to_vec();
    bottom.sort();
    assert_eq!(bottom, vec!["PLAIN-A", "PLAIN-B"], "rest go to the BOTTOM of the deck");
    assert!(runner.game.revealed_cards.is_empty());
}

#[test]
fn ex13_008_on_play_accepts_dracomon_in_name_as_text() {
    let mut runner = setup_play(&["PLAIN-A", "PLAIN-B", "DRACO-NAME"]);
    runner.skip_mulligan();
    let slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, slot).expect("Dracomon plays");
    pick_revealed(&mut runner, "DRACO-NAME");
    assert!(hand_ids(&runner, 0).contains(&"DRACO-NAME".to_string()));
}

#[test]
fn ex13_008_on_play_plain_cards_are_not_pickable() {
    let mut runner = setup_play(&["PLAIN-A", "EXAMON-TEXT", "PLAIN-B"]);
    runner.skip_mulligan();
    let slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, slot).expect("Dracomon plays");
    let view = runner.pending_selection_view().expect("reveal pick prompt");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    let pos = revealed_ids(&runner)
        .iter()
        .position(|r| r == "EXAMON-TEXT")
        .unwrap();
    assert_eq!(picks, vec![SEL_REVEAL_START + pos as u16], "only the text card is a legal pick");
}

#[test]
fn ex13_008_on_play_with_no_match_returns_all_three_to_bottom() {
    let mut runner = setup_play(&["PLAIN-A", "PLAIN-B", "PLAIN-C"]);
    runner.skip_mulligan();
    let hand_before = hand_ids(&runner, 0).len();
    let slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, slot).expect("Dracomon plays");
    finish_remainder(&mut runner);

    assert_eq!(hand_ids(&runner, 0).len(), hand_before - 1, "only Dracomon left the hand");
    let deck = deck_ids(&runner, 0);
    let mut bottom: Vec<String> = deck[..3].to_vec();
    bottom.sort();
    assert_eq!(bottom, vec!["PLAIN-A", "PLAIN-B", "PLAIN-C"]);
    assert_eq!(&deck[3..], &["FILL", "FILL", "FILL"]);
}

// ─── Section 3 — [When Moving] ───────────────────────────────────────────────

#[test]
fn ex13_008_when_moving_out_of_breeding_fires_the_reveal() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(5)
        .build();
    stack_deck_top(&mut runner, &["PLAIN-A", "PLAIN-B", "EXAMON-TEXT"]);
    runner.place_in_breeding(0, CARD_ID);
    assert!(runner.move_from_breeding(0), "Dracomon moves to the battle area");
    pick_revealed(&mut runner, "EXAMON-TEXT");
    assert!(hand_ids(&runner, 0).contains(&"EXAMON-TEXT".to_string()));
}

#[test]
fn ex13_008_another_digimon_moving_does_not_fire_dracomons_reveal() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(5)
        .build();
    stack_deck_top(&mut runner, &["PLAIN-A", "PLAIN-B", "EXAMON-TEXT"]);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_in_breeding(0, "PLAIN-C");
    assert!(runner.move_from_breeding(0));
    assert!(runner.pending_selection().is_none(), "[When Moving] is self-only");
    assert!(runner.game.revealed_cards.is_empty());
}

// ─── Section 3 — Inherited [End of Your Turn] DNA ────────────────────────────

fn dna_setup(partner: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, &["DNA-RESULT"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-L4"]);
    runner.place_on_field(0, partner, Some(0));
    (runner, carrier)
}

fn top_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn ex13_008_inherited_eot_dna_digivolves_carrier_and_partner() {
    let (mut runner, _carrier) = dna_setup("PARTNER-L4");
    runner.end_turn();

    let mut saw_hand_pick = false;
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if view.kind == SelectionKind::Hand {
            saw_hand_pick = true;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or_else(|| panic!("a non-PASS choice must exist: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance DNA prompts");
    }
    assert!(saw_hand_pick, "the DNA result card is chosen from hand");

    let tops = top_ids(&runner, 0);
    assert_eq!(tops, vec!["DNA-RESULT"], "carrier + partner merged under the DNA result");
    let stack: Vec<String> = runner.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(stack.contains(&CARD_ID.to_string()));
    assert!(stack.contains(&"PARTNER-L4".to_string()));
    assert!(!hand_ids(&runner, 0).contains(&"DNA-RESULT".to_string()));
}

#[test]
fn ex13_008_inherited_eot_dna_can_be_declined() {
    let (mut runner, _carrier) = dna_setup("PARTNER-L4");
    runner.end_turn();
    let view = runner.pending_selection_view().expect("optional EoT DNA prompt");
    assert!(view.is_optional, "the DNA digivolve is a \"may\"");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(top_ids(&runner, 0), vec!["CARRIER-L4", "PARTNER-L4"], "no DNA happened");
    assert!(hand_ids(&runner, 0).contains(&"DNA-RESULT".to_string()));
}

#[test]
fn ex13_008_inherited_eot_dna_requires_a_valid_dna_route() {
    // Blue partner does not satisfy the Red Lv.4 + Red Lv.4 DNA requirement.
    let (mut runner, _carrier) = dna_setup("BLUE-L4");
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(top_ids(&runner, 0), vec!["CARRIER-L4", "BLUE-L4"], "no DNA without a legal route");
    assert!(hand_ids(&runner, 0).contains(&"DNA-RESULT".to_string()));
}
