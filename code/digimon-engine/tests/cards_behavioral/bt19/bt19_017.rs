//! BT19-017 Sangomon — Digimon, Lv.3, Blue, DP 1000, Cost 3 (Mollusk/LIBERATOR,
//! (Rule) Aquatic).
//!
//! Official text (data/card_bundles/BT19-017.md):
//!   [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Aqua] or
//!   [Sea Animal] in any of its traits and 1 card with the [LIBERATOR] trait
//!   among them to the hand. Return the rest to the bottom of the deck.
//!   (Rule) Trait: Has [Aquatic] Type.
//!   Inherited: [End of Attack] [Once Per Turn] Gain 1 memory.
//!   Official Q&A: you must add as many cards to your hand as possible.
//!
//! DCGO: BT19/Blue/BT19_017.cs.
//!
//! Regression origin: a real DCGO PvP game replayed through the engine. Sangomon
//! had no YAML, so its [On Play] reveal never prompted (it is also the free-play
//! target of BT22-096 Unique Emblem: Poseidia Lagoon). Synthetic boards only.
#![allow(dead_code, unused_imports)]

use std::path::Path;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "BT19-017";

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt19/BT19-017.yaml"))
        .expect("BT19-017 YAML must exist at cards/bt19/BT19-017.yaml")
}

fn card(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT19-017 YAML parses and compiles")
        .add_card(card("SEA", "Sea Card", &["Sea Animal"]))
        .add_card(card("AQUATIC", "Aquatic Card", &["Aquatic"]))
        .add_card(card("LIB", "Liberator Card", &["LIBERATOR"]))
        .add_card(card("BOTH", "Both Card", &["Aqua", "LIBERATOR"]))
        .add_card(card("PLAIN", "Plain Card", &["Beast"]))
        .add_card(card("PLAIN-B", "Plain Card B", &[]))
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn deck_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn revealed_action(r: &DebugRunner, id: &str) -> Option<u16> {
    r.game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, c)| (c.card_id(&r.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16))
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

/// Builder convention: the LAST id is the top of the deck.
fn play_with_top3(top3_bottom_to_top: [&str; 3]) -> DebugRunner {
    let mut deck = vec!["PLAIN-B"; 6];
    deck.extend(top3_bottom_to_top);
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.play(0, 0).expect("play Sangomon");
    r
}

#[test]
fn bt19_017_printed_metadata() {
    let r = builder().build();
    let data = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .expect("registered");
    assert_eq!(data.play_cost, 3);
    assert_eq!(data.dp, Some(1000));
    assert!(
        data.traits.iter().any(|t| t == "Aquatic"),
        "(Rule) Trait: Has [Aquatic] Type"
    );
    assert!(data.traits.iter().any(|t| t == "LIBERATOR"));
    let compiled = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.alt_paths.len(), 1, "Blue Lv.2 / cost 0");
}

#[test]
fn bt19_017_on_play_adds_one_aqua_and_one_liberator_card_rest_to_bottom() {
    // Top 3 (top first): SEA, LIB, PLAIN.
    let mut r = play_with_top3(["PLAIN", "LIB", "SEA"]);
    // The 3 revealed cards are out of the deck while the pick is pending.
    let deck_before = r.deck_size(0) + r.game.revealed_cards.len();
    // First bucket (Aqua / Sea Animal): only SEA is eligible.
    let sea = revealed_action(&r, "SEA").expect("SEA revealed");
    assert_eq!(non_pass(&r), vec![sea], "only the [Sea Animal] card");
    r.execute_action(0, sea).expect("pick SEA");
    // Second bucket (LIBERATOR): only LIB is eligible.
    let lib = revealed_action(&r, "LIB").expect("LIB revealed");
    assert_eq!(non_pass(&r), vec![lib], "only the [LIBERATOR] card");
    r.execute_action(0, lib).expect("pick LIB");
    let _ = r.auto_resolve();
    let hand = hand_ids(&r);
    assert!(hand.contains(&"SEA".to_string()) && hand.contains(&"LIB".to_string()), "{hand:?}");
    assert!(!hand.contains(&"PLAIN".to_string()));
    assert_eq!(r.deck_size(0), deck_before - 2, "2 added, 1 returned to the bottom");
    assert_eq!(
        deck_ids(&r).first().map(String::as_str),
        Some("PLAIN"),
        "the unchosen card goes to the bottom of the deck"
    );
}

#[test]
fn bt19_017_aqua_in_any_of_its_traits_matches_aquatic() {
    // "[Aqua] ... in any of its traits" is a substring scan (DCGO HasAquaTraits
    // = ContainsTraits("Aqua")): an [Aquatic]-trait card qualifies.
    let mut r = play_with_top3(["PLAIN", "LIB", "AQUATIC"]);
    let aquatic = revealed_action(&r, "AQUATIC").expect("AQUATIC revealed");
    assert_eq!(non_pass(&r), vec![aquatic]);
    r.execute_action(0, aquatic).unwrap();
    let lib = revealed_action(&r, "LIB").unwrap();
    r.execute_action(0, lib).unwrap();
    let _ = r.auto_resolve();
    assert!(hand_ids(&r).contains(&"AQUATIC".to_string()));
}

#[test]
fn bt19_017_one_card_cannot_fill_both_slots_when_two_cards_can_be_added() {
    // BOTH is [Aqua]+[LIBERATOR]. Q&A: add as many cards as possible, so BOTH
    // fills one slot and LIB the other — never BOTH twice.
    let mut r = play_with_top3(["PLAIN", "LIB", "BOTH"]);
    let both = revealed_action(&r, "BOTH").expect("BOTH revealed");
    let first = non_pass(&r);
    assert!(first.contains(&both), "BOTH is a legal [Aqua] pick");
    r.execute_action(0, both).unwrap();
    let lib = revealed_action(&r, "LIB").unwrap();
    assert_eq!(
        non_pass(&r),
        vec![lib],
        "BOTH is already taken, so the [LIBERATOR] slot can only take LIB"
    );
    r.execute_action(0, lib).unwrap();
    let _ = r.auto_resolve();
    let hand = hand_ids(&r);
    assert!(hand.contains(&"BOTH".to_string()) && hand.contains(&"LIB".to_string()));
}

#[test]
fn bt19_017_on_play_with_no_matching_card_returns_all_three() {
    let mut r = play_with_top3(["PLAIN", "PLAIN-B", "PLAIN"]);
    let deck_before = r.deck_size(0) + r.game.revealed_cards.len();
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before, "nothing added");
    assert!(!hand_ids(&r).iter().any(|id| id == "PLAIN" || id == "PLAIN-B"));
}

#[test]
fn bt19_017_inherited_end_of_attack_gain_memory_is_once_per_turn() {
    let r = builder().build();
    let compiled = r.compiled_card(CARD_ID).expect("compiled");
    let inherited = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("an inherited clause");
    assert!(inherited.when.contains(&CompiledTiming::EndOfAttack));
    assert!(inherited.once_per_turn, "[Once Per Turn]");
}
