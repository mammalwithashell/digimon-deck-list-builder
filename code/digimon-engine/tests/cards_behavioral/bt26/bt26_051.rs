//! BT26-051 Gomimon — Digimon Lv.3 Black, Stnd./Appmon, Trashbin/Seven Code,
//! Tool. <Detach ([Seven Code])>; [Your Turn][OPT] when this Digimon gets
//! linked, 1 of your [Social]/[Tool]/[Open]/[Seven Code] Digimon gains
//! <Collision> and +3000 DP for the turn; Link [Appmon] cost 3, DP+3000;
//! [When Linking] <De-Digivolve 2> 1 opponent Digimon.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Yellow/BT26_051.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-051";
const MAIL: &str = "BT26-019";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-051 compiles")
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
            "SC-LINK",
            "SevenLink",
            CardColor::Red,
            3,
            4,
            &["Seven Code"],
        ))
        .add_card(digimon("OPP-A", "OppA", CardColor::Red, 3, 4, &[]))
        .add_card(digimon("OPP-B", "OppB", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP-C", "OppC", CardColor::Red, 5, 6, &[]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_051_when_linked_grants_collision_and_3000() {
    let mut r = base().hand(0, &[MAIL]).memory(5).start();
    r.game.enter_main_phase();
    let g = r.place_on_field(0, ID, Some(0));
    let dp0 = r.effective_dp(g).unwrap();
    link_from_hand(&mut r, 0, MAIL);
    drain_first(&mut r);
    r.game.tick_declarative_effects();
    assert_eq!(linked_ids(&r, g), vec![MAIL.to_string()]);
    assert!(
        r.game.has_keyword(g, Keyword::Collision),
        "gains <Collision>"
    );
    assert_eq!(
        r.effective_dp(g),
        Some(dp0 + 3000 + 3000),
        "+3000 (effect) +3000 (Mailmon's Link DP)"
    );
}

#[test]
fn bt26_051_when_linked_target_must_have_trait() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    let g = r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "OPP-A", Some(0)); // no listed trait
    push_link_and_fire(&mut r, g, "SC-LINK");
    // Only Gomimon itself qualifies → a single candidate.
    if r.pending_selection().is_some() {
        assert_eq!(non_pass(&r).len(), 1);
        pick_first(&mut r, 0);
    }
    assert!(r.game.has_keyword(g, Keyword::Collision));
}

#[test]
fn bt26_051_when_linked_once_per_turn() {
    let mut r = base().memory(5).start();
    r.game.enter_main_phase();
    let g = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, g, "SC-LINK");
    drain_first(&mut r);
    let dp1 = r.effective_dp(g).unwrap();
    push_link_and_fire(&mut r, g, "SC-LINK");
    drain_first(&mut r);
    assert_eq!(r.effective_dp(g), Some(dp1), "OPT: no second +3000");
}

#[test]
fn bt26_051_when_linked_not_on_opponents_turn() {
    let mut r = base().memory(5).start();
    r.end_turn();
    let g = r.place_on_field(0, ID, Some(0));
    push_link_and_fire(&mut r, g, "SC-LINK");
    drain_first(&mut r);
    assert!(
        !r.game.has_keyword(g, Keyword::Collision),
        "[Your Turn] only"
    );
}

#[test]
fn bt26_051_when_linking_de_digivolves_2() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "APP-HOST", Some(0));
    let opp = r.place_stack(1, &["OPP-A", "OPP-B", "OPP-C"]);
    link_from_hand(&mut r, 0, ID);
    drain_first(&mut r);
    assert_eq!(top_id(&r, opp), "OPP-A", "two cards stripped");
    assert_eq!(r.memory(), 2, "link cost 3");
}

#[test]
fn bt26_051_detach() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let g = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(g, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(g, ReplacementCause::Battle);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()]);
}
