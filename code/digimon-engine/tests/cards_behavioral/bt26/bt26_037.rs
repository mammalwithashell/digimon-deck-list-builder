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
        // Linked cards carry [Link] (Q4533; §17-1-3-2-6 / -7).
        .link_requirement("NAVI-L3", 0)
        .link_requirement("NAVI-L4", 0)
        .link_requirement("SC-LINK", 0)
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

// Linked-source resume regression controls. The compaction/detach cases are
// captured-prompt adversarial fixtures, not claims of a printed interleaving.
fn linked_resume_pick_opponent(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("When Linking target choice");
    assert_eq!(v.selecting_player, 0);
    assert!(v.is_optional);
    let a = digimon_engine::action::space::encode_attack(0, 0);
    assert!(v.valid_action_ids.contains(&a));
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.execute_action(0, a).expect("legal opponent pick");
    assert!(r.pending_selection().is_none(), "battle completed without auto choices");
}

#[test]
fn bt26_037_linked_resume_original_and_clone_battle_with_host() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    link_from_hand(&mut r, 0, ID);
    let link = r.game.players[0].battle_area[host.index as usize].linked_cards[0].handle();
    let saved = r.game.clone();
    for branch in 0..2 {
        if branch == 1 { r.game = saved.clone(); }
        linked_resume_pick_opponent(&mut r);
        assert!(field_ids(&r, 1).is_empty());
        assert_eq!(field_ids(&r, 0), vec!["APP-HOST".to_string()]);
        assert_eq!(r.game.players[0].battle_area[0].linked_cards[0].handle(), link);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
    }
}

#[test]
fn bt26_037_linked_resume_compaction_uses_physical_carrier_original_and_clone() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "FILLER", Some(0)); // not a legal Appmon link host
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    link_from_hand(&mut r, 0, ID);
    assert_eq!(host.index, 1);
    let carrier = r.game.players[0].battle_area[1].top_card().handle();
    let linked = r.game.players[0].battle_area[1].linked_cards[0].handle();
    // Deliberate fixture surgery: establish real physical compaction without
    // invoking a second effect while the first target prompt is pending.
    let removed = r.game.players[0].battle_area.remove(0);
    assert!(removed.linked_cards.is_empty());
    r.game.players[0].trash.extend(removed.card_sources);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().handle(), carrier);
    assert_eq!(r.game.players[0].battle_area[0].linked_cards[0].handle(), linked);
    r.game.tick_declarative_effects();
    let saved = r.game.clone();
    for branch in 0..2 {
        if branch == 1 { r.game = saved.clone(); }
        linked_resume_pick_opponent(&mut r);
        assert!(field_ids(&r, 1).is_empty(), "compacted carrier still battles");
        assert_eq!(r.game.players[0].battle_area[0].top_card().handle(), carrier);
        assert_eq!(r.game.players[0].battle_area[0].linked_cards[0].handle(), linked);
    }
}

#[test]
fn bt26_037_linked_resume_detached_source_does_not_borrow_live_host() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    link_from_hand(&mut r, 0, ID);
    let link = r.game.players[0].battle_area[0].linked_cards[0].handle();
    // Public physical movement API, with explicit post-movement assertions.
    assert!(r.game.trash_specific_link_card(host, link));
    assert!(r.game.players[0].battle_area[0].linked_cards.is_empty());
    assert!(r.game.players[0].trash.iter().any(|c| c.handle() == link));
    assert!(r.pending_selection().is_some());
    r.game.tick_declarative_effects();
    let saved = r.game.clone();
    for branch in 0..2 {
        if branch == 1 { r.game = saved.clone(); }
        linked_resume_pick_opponent(&mut r);
        assert_eq!(field_ids(&r, 1), vec!["OPP-WEAK".to_string()]);
        assert_eq!(field_ids(&r, 0), vec!["APP-HOST".to_string()]);
        assert!(r.game.players[0].trash.iter().any(|c| c.handle() == link));
    }
}

#[test]
fn bt26_037_linked_resume_option_origin_never_invents_a_carrier() {
    // Synthetic substrate negative, not an authored card behavior: a card-only
    // Option source has no battle carrier despite an unrelated friendly body.
    let yaml = r#"
card: TEST-LINK-OPTION-ORIGIN
name: Option Origin Probe
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: enemy
          optional: true
          filter: { kind: digimon }
          prompt: "Choose a target for source-origin probe"
      - battle: { attacker: source, defender: enemy }
      - draw: { of: you, count: 1 }
"#;
    let mut r = base().from_dsl_yaml(yaml).unwrap()
        .hand(0, &["TEST-LINK-OPTION-ORIGIN"]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "APP-HOST", Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    let a = digimon_engine::action::space::PLAY_HAND_START;
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.game.decode_action(a, 0);
    assert!(r.pending_selection().is_some());
    let hand_before = r.game.players[0].hand.len();
    let saved = r.game.clone();
    for branch in 0..2 {
        if branch == 1 { r.game = saved.clone(); }
        linked_resume_pick_opponent(&mut r);
        assert_eq!(field_ids(&r, 1), vec!["OPP-WEAK".to_string()]);
        assert_eq!(field_ids(&r, 0), vec!["APP-HOST".to_string()]);
        assert_eq!(r.game.players[0].hand.len(), hand_before + 1, "tail actually ran");
    }
}
