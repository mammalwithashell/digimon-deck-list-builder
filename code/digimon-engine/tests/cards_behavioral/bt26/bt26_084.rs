//! BT26-084 Copipemon — Digimon Lv.3 White, Stnd./Appmon, Copy & Paste/Seven
//! Code, System. <Detach ([Seven Code])>; [Your Turn][OPT] when this Digimon
//! gets linked, reveal 3, you may play or use 1 [Seven Code] card among them
//! with the cost reduced by 3, rest top or bottom; Link [Appmon] cost 3,
//! DP+3000; [When Linking] you may link 1 non-white Lv.4- [System]/[Seven
//! Code] card from your trash to this Digimon for free.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/White/BT26_084.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-084";

fn sc_option(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::White];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.traits = vec!["Seven Code".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-084 compiles")
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
            "SC-DIGI",
            "SevenDigi",
            CardColor::Red,
            4,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SC-L4",
            "SevenL4",
            CardColor::Red,
            4,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "WHITE-SYS",
            "WhiteSys",
            CardColor::White,
            3,
            4,
            &["System"],
        ))
        .add_card(digimon(
            "SYS-L5",
            "SysL5",
            CardColor::Blue,
            5,
            6,
            &["System"],
        ))
        .add_card(sc_option("SC-OPT", 4))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_084_when_linked_plays_seven_code_digimon_minus_3() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    stack_deck_top(&mut r, 0, &["FILLER", "SC-DIGI", "FILLER"]);
    let c = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, c, "SC-LINK");
    pick_reveal(&mut r, 0, "SC-DIGI");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"SC-DIGI".to_string()));
    assert_eq!(r.memory(), 3, "play cost 5 reduced by 3 → pays 2");
    assert!(
        r.game.revealed_cards.is_empty(),
        "remainder returned to the deck"
    );
}

#[test]
fn bt26_084_when_linked_uses_seven_code_option_minus_3() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    stack_deck_top(&mut r, 0, &["FILLER", "SC-OPT", "FILLER"]);
    let c = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, c, "SC-LINK");
    pick_reveal(&mut r, 0, "SC-OPT");
    drain_first(&mut r);
    assert!(
        trash_ids(&r, 0).contains(&"SC-OPT".to_string()),
        "Option used"
    );
    assert_eq!(r.memory(), 4, "use cost 4 reduced by 3 → pays 1");
}

#[test]
fn bt26_084_when_linked_may_decline_and_remainder_still_returns() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    stack_deck_top(&mut r, 0, &["FILLER", "SC-DIGI", "FILLER"]);
    let c = r.place_on_field(0, ID, Some(0));
    let deck0 = r.deck_size(0);
    push_link_and_fire(&mut r, c, "SC-LINK");
    pass(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(r.deck_size(0), deck0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()]);
    assert_eq!(r.memory(), 5);
}

#[test]
fn bt26_084_unaffordable_card_not_offered() {
    let mut r = base().memory(-9).start();
    r.game.enter_main_phase();
    stack_deck_top(&mut r, 0, &["FILLER", "SC-DIGI", "FILLER"]);
    let c = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, c, "SC-LINK");
    // No affordable candidate → the optional pick is skipped and the only
    // prompt is the remainder top/bottom choice (2 options).
    assert_eq!(non_pass(&r).len(), 2, "remainder top/bottom choice only");
    drain_first(&mut r);
    assert!(!field_ids(&r, 0).contains(&"SC-DIGI".to_string()));
}

#[test]
fn bt26_084_when_linked_not_on_opponents_turn() {
    let mut r = base().memory(5).start();
    r.end_turn();
    stack_deck_top(&mut r, 0, &["FILLER", "SC-DIGI", "FILLER"]);
    let c = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, c, "SC-LINK");
    assert!(r.game.revealed_cards.is_empty());
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt26_084_when_linking_links_eligible_trash_card() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    push_trash(&mut r, 0, "WHITE-SYS");
    push_trash(&mut r, 0, "SYS-L5");
    push_trash(&mut r, 0, "SC-L4");
    link_from_hand(&mut r, 0, ID);
    assert_eq!(non_pass(&r).len(), 1, "only the non-white Lv.4- card");
    pick_trash(&mut r, 0, "SC-L4");
    drain_first(&mut r);
    assert_eq!(
        linked_ids(&r, host),
        vec![ID.to_string(), "SC-L4".to_string()]
    );
    assert_eq!(r.memory(), 2, "only Copipemon's own link cost 3 is paid");
}

#[test]
fn bt26_084_when_linking_is_optional() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    push_trash(&mut r, 0, "SC-L4");
    link_from_hand(&mut r, 0, ID);
    pass(&mut r, 0);
    assert_eq!(linked_ids(&r, host), vec![ID.to_string()]);
}

#[test]
fn bt26_084_detach() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let c = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(c, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(c, ReplacementCause::OpponentEffect);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()]);
}
