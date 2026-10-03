//! BT26-063 Tellermon — Digimon Lv.3 Purple, Stnd./Appmon, Fortune
//! Telling/Seven Code, Entertainment. <Detach ([Seven Code])>; [Your Turn]
//! [OPT] when this Digimon gets linked, reveal 3, add 1 [Entertainment]/
//! [Open]/[Seven Code] card to hand, rest top or bottom; Link [Appmon] cost 3,
//! DP+3000; [When Linking] delete 1 opponent Digimon with the lowest level.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Purple/BT26_063.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-063";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-063 compiles")
        .add_card(digimon(
            "APP-HOST",
            "AppHost",
            CardColor::White,
            4,
            5,
            &["Appmon"],
        ))
        .add_card(digimon(
            "SC-LINK",
            "SevenLink",
            CardColor::Red,
            3,
            4,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "ENT-CARD",
            "EntCard",
            CardColor::Red,
            3,
            4,
            &["Entertainment"],
        ))
        .add_card(digimon("OPP-L3", "OppL3", CardColor::Red, 3, 4, &[]))
        .add_card(digimon("OPP-L5", "OppL5", CardColor::Red, 5, 6, &[]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_063_when_linked_reveals_and_adds_trait_card() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    stack_deck_top(&mut r, 0, &["FILLER", "ENT-CARD", "FILLER"]);
    let t = r.place_on_field(0, ID, Some(0));
    let deck0 = r.deck_size(0);
    push_link_and_fire(&mut r, t, "SC-LINK");
    pick_reveal(&mut r, 0, "ENT-CARD");
    drain_first(&mut r);
    assert_eq!(hand_ids(&r, 0), vec!["ENT-CARD".to_string()]);
    assert_eq!(r.deck_size(0), deck0 - 1, "the other 2 go back to the deck");
    assert!(r.game.revealed_cards.is_empty());
}

#[test]
fn bt26_063_when_linked_not_on_opponents_turn() {
    let mut r = base().memory(5).start();
    r.end_turn();
    stack_deck_top(&mut r, 0, &["FILLER", "ENT-CARD", "FILLER"]);
    let t = r.place_on_field(0, ID, Some(0));
    let deck0 = r.deck_size(0);
    push_link_and_fire(&mut r, t, "SC-LINK");
    drain_first(&mut r);
    assert_eq!(r.deck_size(0), deck0, "[Your Turn] only");
}

#[test]
fn bt26_063_when_linked_once_per_turn() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    stack_deck_top(&mut r, 0, &["ENT-CARD", "ENT-CARD", "ENT-CARD"]);
    let t = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, t, "SC-LINK");
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 1);
    push_link_and_fire(&mut r, t, "SC-LINK");
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 1, "OPT");
}

#[test]
fn bt26_063_when_linking_deletes_lowest_level_only() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-L5", Some(0));
    r.place_on_field(1, "OPP-L3", Some(0));
    link_from_hand(&mut r, 0, ID);
    assert_eq!(non_pass(&r).len(), 1, "only the lowest-level Digimon");
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP-L5".to_string()]);
}

#[test]
fn bt26_063_detach() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let t = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(t, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(t, ReplacementCause::OpponentEffect);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()]);
}
