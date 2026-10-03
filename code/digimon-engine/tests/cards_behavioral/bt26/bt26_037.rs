//! BT26-037 Weatherdramon — Digimon Lv.4 Green, Sup./Appmon, Weather/Seven
//! Code, Navi. <Blocker> <Detach ([Seven Code])>; [On Play][When
//! Digivolving] may link 1 Lv.3 [Navi]/[System]/[Seven Code] Digimon card
//! from its digivolution cards to it free; Assembly -2; App Fusion
//! [Weathermon] & [Rocketmon] & [Newsmon]; Link [Appmon] cost 3, DP+3000;
//! [When Linking] this Digimon may battle 1 of your opponent's Digimon.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Green/BT26_037.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-037";
const MAIL: &str = "BT26-019";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-037 compiles")
        .dsl_card(MAIL)
        .expect("BT26-019 compiles")
        .add_card(digimon(
            "APP-HOST",
            "AppHost",
            CardColor::White,
            4,
            5,
            &["Appmon"],
        ))
        .add_card(digimon(
            "NAVI-L3",
            "NaviL3",
            CardColor::Red,
            3,
            4,
            &["Navi"],
        ))
        .add_card(digimon(
            "NAVI-L4",
            "NaviL4",
            CardColor::Red,
            4,
            4,
            &["Navi"],
        ))
        .add_card(digimon(
            "SC-LINK",
            "SevenLink",
            CardColor::Red,
            3,
            4,
            &["Seven Code"],
        ))
        .add_card(digimon("OPP-WEAK", "OppWeak", CardColor::Red, 3, 4, &[]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_037_structure_and_blocker() {
    let mut r = base().start();
    let c = r.compiled_card(ID).unwrap();
    assert!(c
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::AppFusion));
    assert!(c.alt_paths.iter().any(
        |p| p.kind == CompiledAltPathKind::Assembly && p.cost == Some(CompiledCost::Literal(2))
    ));
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

#[test]
fn bt26_037_on_play_links_navi_lv3_source() {
    let mut r = base().memory(3).start();
    r.game.enter_main_phase();
    let h = r.place_stack(0, &["NAVI-L4", "NAVI-L3", ID]);
    fire(&mut r, EffectTiming::OnPlay, h);
    assert_eq!(non_pass(&r).len(), 1, "only the Lv.3 [Navi] source");
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(linked_ids(&r, h), vec!["NAVI-L3".to_string()]);
}

#[test]
fn bt26_037_when_linking_host_battles_opponent_digimon() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    link_from_hand(&mut r, 0, ID);
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert!(
        field_ids(&r, 1).is_empty(),
        "host (7000) beat the 3000 Digimon"
    );
    assert_eq!(field_ids(&r, 0), vec!["APP-HOST".to_string()]);
    assert!(!r.game.players[0].battle_area[host.index as usize].is_suspended);
}

#[test]
fn bt26_037_when_linking_battle_is_optional() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    link_from_hand(&mut r, 0, ID);
    pass(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP-WEAK".to_string()]);
}

#[test]
fn bt26_037_detach() {
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
