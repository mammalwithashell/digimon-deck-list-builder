//! BT26-010 Roleplaymon — Digimon Lv.3 Red, Stnd./Appmon, Role-playing/Seven
//! Code, Game. <Detach ([Seven Code])>; [When Attacking] by trashing 1
//! [Game]/[Open]/[Seven Code] card from hand, <Draw 2>; Link [Appmon] cost 3,
//! DP+3000; Link effect <Progress> <Piercing>.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Red/BT26_010.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-010";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-010 compiles")
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
        .add_card(digimon("GAME-C", "GameC", CardColor::Red, 3, 4, &["Game"]))
        .add_card(digimon("OPEN-C", "OpenC", CardColor::Red, 3, 4, &["Open"]))
        .add_card(digimon(
            "PLAIN-C",
            "PlainC",
            CardColor::Red,
            3,
            4,
            &["Beast"],
        ))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_010_when_attacking_trash_game_card_draw_2() {
    let mut r = base().hand(0, &["PLAIN-C", "GAME-C"]).start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert_eq!(non_pass(&r).len(), 1, "only the [Game] card is eligible");
    pick_hand(&mut r, 0, "GAME-C");
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 3, "2 - 1 + 2");
    assert!(trash_ids(&r, 0).contains(&"GAME-C".to_string()));
}

#[test]
fn bt26_010_when_attacking_open_trait_also_eligible() {
    let mut r = base().hand(0, &["OPEN-C"]).start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, h);
    pick_hand(&mut r, 0, "OPEN-C");
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 2);
}

#[test]
fn bt26_010_when_attacking_decline_no_draw() {
    let mut r = base().hand(0, &["GAME-C"]).start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, h);
    pass(&mut r, 0);
    assert_eq!(hand_ids(&r, 0), vec!["GAME-C".to_string()]);
}

#[test]
fn bt26_010_when_attacking_no_eligible_card_no_prompt() {
    let mut r = base().hand(0, &["PLAIN-C"]).start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_010_link_grants_progress_piercing_and_3000() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    let dp0 = r.effective_dp(host).unwrap();
    link_from_hand(&mut r, 0, ID);
    drain_first(&mut r);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(host, Keyword::Progress));
    assert!(r.game.has_keyword(host, Keyword::Piercing));
    assert_eq!(r.effective_dp(host), Some(dp0 + 3000));
}

#[test]
fn bt26_010_unlinked_has_no_progress() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(h, Keyword::Progress));
    assert!(!r.game.has_keyword(h, Keyword::Piercing));
    assert_eq!(
        r.effective_dp(h),
        Some(4000),
        "Link DP only applies to a host"
    );
}

#[test]
fn bt26_010_detach() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(h, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::Battle);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()]);
}
