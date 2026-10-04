//! BT26-007 Swipemon — Digi-Egg Lv.2 White. Inherited: [When Attacking]
//! [Once Per Turn] You may link 1 [Seven Code] trait Digimon card from your
//! hand or this Digimon's digivolution cards to this Digimon with the cost
//! reduced by 2.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Green/BT26_007.cs
//! Linked card fixture: BT26-019 Mailmon (real [Seven Code], <Link> [Appmon]
//! trait: Cost 3 → pays 1 after the -2).

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming};

const ID: &str = "BT26-007";
const MAIL: &str = "BT26-019";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-007 compiles")
        .dsl_card(MAIL)
        .expect("BT26-019 compiles")
        .add_card(digimon(
            "APP-TOP",
            "AppTop",
            CardColor::White,
            3,
            4,
            &["Appmon"],
        ))
        .add_card(digimon(
            "PLAIN-TOP",
            "PlainTop",
            CardColor::White,
            3,
            4,
            &[],
        ))
        .add_card(digimon("NOT-SC", "NotSC", CardColor::Red, 3, 4, &["Tool"]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_007_links_mailmon_from_hand_paying_1() {
    let mut r = base().hand(0, &[MAIL]).memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    pick_hand(&mut r, 0, MAIL);
    drain_first(&mut r);
    assert_eq!(linked_ids(&r, s), vec![MAIL.to_string()]);
    assert_eq!(r.memory(), 2, "link cost 3 reduced by 2 → pays 1");
}

#[test]
fn bt26_007_links_from_digivolution_cards() {
    let mut r = base().memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, MAIL, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(linked_ids(&r, s), vec![MAIL.to_string()]);
    assert_eq!(source_ids(&r, s), vec![ID.to_string()]);
    assert_eq!(r.memory(), 2);
}

#[test]
fn bt26_007_both_zones_prompt_zone_choice_first() {
    let mut r = base().hand(0, &[MAIL]).memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, MAIL, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert_eq!(non_pass(&r).len(), 2, "From hand / From digivolution cards");
}

#[test]
fn bt26_007_decline_refunds_once_per_turn() {
    let mut r = base().hand(0, &[MAIL]).memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    pass(&mut r, 0);
    assert!(linked_ids(&r, s).is_empty());
    assert_eq!(r.memory(), 3);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert!(
        r.pending_selection().is_some(),
        "OPT not consumed by a decline"
    );
}

#[test]
fn bt26_007_once_per_turn_after_link() {
    let mut r = base().hand(0, &[MAIL, MAIL]).memory(5).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    pick_hand(&mut r, 0, MAIL);
    drain_first(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert!(r.pending_selection().is_none(), "OPT consumed");
    assert_eq!(linked_ids(&r, s).len(), 1);
}

#[test]
fn bt26_007_non_seven_code_not_offered() {
    let mut r = base().hand(0, &["NOT-SC"]).memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt26_007_host_must_satisfy_link_condition() {
    // Mailmon's <Link> needs an [Appmon] host; a non-Appmon top can't take it.
    let mut r = base().hand(0, &[MAIL]).memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, "PLAIN-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt26_007_unaffordable_link_not_offered() {
    let mut r = base().hand(0, &[MAIL]).memory(-10).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID, "APP-TOP"]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert!(
        r.pending_selection().is_none(),
        "can't pay the reduced cost of 1"
    );
}

#[test]
fn bt26_007_not_active_on_its_own() {
    let mut r = base().hand(0, &[MAIL]).memory(3).start();
    r.game.enter_main_phase();
    let s = r.place_stack(0, &[ID]);
    fire(&mut r, EffectTiming::WhenAttacking, s);
    assert!(r.pending_selection().is_none(), "inherited only");
}
