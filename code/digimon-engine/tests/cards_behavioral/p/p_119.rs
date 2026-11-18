//! P-119 Hawkmon — Lv.3 Red, cost 3, 1000 DP.
//!
//! Printed text (data/card_bundles/P-119.md, official Bandai DB):
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 red or yellow card with
//!   2 or more colors and 1 Tamer card with [Yolei Inoue] in its name among
//!   them to the hand. Return the rest to the bottom of the deck.
//! Inherited: [End of Your Turn] This Digimon and another of your Digimon may
//!   DNA digivolve into a Digimon card in your hand.
//! ```
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/P/Red/P_119.cs. Synthetic decks.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};

fn digimon(id: &str, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = colors;
    c
}

/// A Tamer in `colors`. NOTE: the first picker reads "1 red or yellow card
/// with 2 or more colors" -- any card KIND -- so a red/yellow 2-colour Tamer
/// would qualify for it too. These Tamers are blue/green so each test card
/// matches exactly one picker.
fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue, CardColor::Green];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("P-119")
        .expect("P-119 compiles into the embedded pack")
        .add_card(digimon("T-RY", vec![CardColor::Red, CardColor::Yellow]))
        .add_card(digimon("T-RED-MONO", vec![CardColor::Red]))
        .add_card(digimon("T-BLUE-GREEN", vec![CardColor::Blue, CardColor::Green]))
        .add_card(tamer("T-YOLEI", "Yolei Inoue & Kari Kamiya"))
        .add_card(tamer("T-OTHER-TAMER", "Davis Motomiya"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(1, &["FILL"; 6])
}

fn in_hand(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

#[test]
fn p_119_has_on_play_and_inherited_end_of_turn_dna() {
    let runner = base().deck(0, &["FILL"; 6]).memory(10).start();
    let compiled = runner.compiled_card("P-119").expect("compiled");
    assert_eq!(compiled.effects.len(), 2);
    assert!(
        compiled
            .effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited)),
        "the End of Your Turn DNA digivolve is inherited"
    );
}

/// The top 3 cards are revealed: 1 red/yellow multicolour card and 1 Tamer with
/// [Yolei Inoue] go to the hand; the rest returns to the bottom of the deck.
#[test]
fn p_119_on_play_adds_the_multicolour_card_and_the_yolei_tamer() {
    // A 3-card deck: the top 3 are the whole deck, whatever the deck's order.
    let mut runner = base()
        .hand(0, &["P-119"])
        .deck(0, &["T-RY", "T-YOLEI", "T-RED-MONO"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.play(0, 0);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("reveal picks");
    assert!(in_hand(&runner, "T-RY"), "the red/yellow 2-colour card was added");
    assert!(in_hand(&runner, "T-YOLEI"), "the [Yolei Inoue] Tamer was added");
    assert!(!in_hand(&runner, "T-RED-MONO"), "a mono-coloured card does not qualify");
    assert_eq!(runner.deck_size(0), 1, "the rest returned to the deck");
}

/// A 2-colour card that is neither red nor yellow, and a Tamer without Yolei,
/// do not qualify.
#[test]
fn p_119_on_play_ignores_non_qualifying_cards() {
    let mut runner = base()
        .hand(0, &["P-119"])
        .deck(0, &["T-BLUE-GREEN", "T-OTHER-TAMER", "T-RED-MONO"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.play(0, 0);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("reveal picks");
    assert!(!in_hand(&runner, "T-BLUE-GREEN"));
    assert!(!in_hand(&runner, "T-OTHER-TAMER"));
    assert_eq!(runner.deck_size(0), 3, "nothing qualified: all 3 returned to the deck");
}
