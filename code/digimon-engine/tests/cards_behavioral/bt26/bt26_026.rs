//! BT26-026 Cougarmon — Digimon Lv.4, Yellow, DP 4000, Cost 4.
//! Traits: Mammal, Glowing Dawn, BEATBREAK.
//! Digivolve: Yellow Lv.3 / cost 2; [Digivolve] Lv.3 w/[Glowing Dawn]: cost 2.
//!
//! <Barrier> [When Attacking] [Once Per Turn] By trashing the bottom face-down
//! card from under any of your Tamers or your top security card, you may use 1
//! Option card with the [Glowing Dawn] trait from your hand with the cost
//! reduced by 2.
//! Inherited: <Barrier>.
//!
//! DCGO: BT26/Yellow/BT26_026.cs — a pay-cost menu offering only the PAYABLE
//! costs plus "Don't pay the cost".

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-026";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-026")
        .add_card(filler("FILLER"))
        .add_card(tamer("SECT", "SecTamer", CardColor::Red, &[]))
        .add_card(tamer(
            "GDT",
            "GD Tamer",
            CardColor::Yellow,
            &["Glowing Dawn"],
        ))
        .add_card(gd_option("GD-OPT", 5))
        .add_card({
            let mut o = gd_option("PLAIN-OPT", 5);
            o.traits.clear();
            o
        })
        .add_card(digimon("HOST", "Host", CardColor::Yellow, 5, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SECT", "SECT", "SECT"])
        .security(1, &["SECT", "SECT", "SECT"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn effect_labels(r: &DebugRunner) -> Vec<String> {
    let sel = r.pending_selection().expect("pay-cost menu");
    sel.effect_choices
        .as_ref()
        .expect("an effect-choice menu")
        .iter()
        .map(|c| c.label.clone())
        .collect()
}

fn board(r: &mut DebugRunner, fd: usize) -> (PermanentHandle, PermanentHandle) {
    let t = tamer_with_face_down(r, 0, "GDT", fd);
    let c = r.place_on_field(0, CARD_ID, Some(0));
    (t, c)
}

#[test]
fn bt26_026_has_barrier_on_face_and_inherited() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(c, Keyword::Barrier));
    let h = r.place_stack(0, &[CARD_ID, "HOST"]);
    assert!(
        r.game.has_keyword(h, Keyword::Barrier),
        "inherited <Barrier>"
    );
}

#[test]
fn bt26_026_menu_offers_both_payable_costs_and_decline() {
    let mut r = setup();
    let (_t, c) = board(&mut r, 1);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert_eq!(effect_labels(&r).len(), 3, "{:?}", effect_labels(&r));
}

#[test]
fn bt26_026_trash_face_down_then_use_option_cost_reduced_by_two() {
    let mut r = setup();
    let (t, c) = board(&mut r, 1);
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(0).expect("trash a Tamer's face-down card");
    pick_first(&mut r, 0); // the Tamer
    assert_eq!(sources(&r, t), 0, "face-down card trashed");
    let v = r.pending_selection_view().expect("option pick");
    assert!(v.is_optional, "'you may use'");
    pick_hand(&mut r, 0, "GD-OPT");
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"GD-OPT".to_string()), "used");
    assert_eq!(r.memory(), 3 - 3, "use cost 5 - 2 = 3");
    assert_eq!(r.security_count(0), 3);
}

#[test]
fn bt26_026_trash_top_security_then_use_option() {
    let mut r = setup();
    let (t, c) = board(&mut r, 1);
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(1).expect("trash top security");
    pick_hand(&mut r, 0, "GD-OPT");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2);
    assert_eq!(sources(&r, t), 1, "Tamer stash untouched");
    assert_eq!(r.memory(), 0);
}

#[test]
fn bt26_026_only_security_payable_offers_two_choices() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert_eq!(effect_labels(&r).len(), 2, "{:?}", effect_labels(&r));
    r.execute_branch(0).expect("trash top security");
    pick_hand(&mut r, 0, "GD-OPT");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2);
    assert_eq!(r.memory(), 0);
}

#[test]
fn bt26_026_only_face_down_payable_offers_two_choices() {
    let mut r = setup();
    let (t, c) = board(&mut r, 1);
    r.game.players[0].security.clear();
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert_eq!(effect_labels(&r).len(), 2, "{:?}", effect_labels(&r));
    r.execute_branch(0).expect("trash a Tamer's face-down card");
    pick_first(&mut r, 0);
    pick_hand(&mut r, 0, "GD-OPT");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0);
    assert_eq!(r.memory(), 0);
}

#[test]
fn bt26_026_dont_pay_does_nothing() {
    let mut r = setup();
    let (t, c) = board(&mut r, 1);
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(2).expect("don't pay");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    assert_eq!(r.security_count(0), 3);
    assert_eq!(hand_ids(&r, 0), vec!["GD-OPT".to_string()]);
}

#[test]
fn bt26_026_cost_paid_but_option_use_declined() {
    let mut r = setup();
    let (_t, c) = board(&mut r, 1);
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(1).expect("trash top security");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2, "cost stays paid");
    assert_eq!(hand_ids(&r, 0), vec!["GD-OPT".to_string()]);
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt26_026_only_glowing_dawn_options_are_eligible() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(0).expect("trash top security");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["PLAIN-OPT".to_string()]);
    assert_eq!(r.memory(), 3);
}

#[test]
fn bt26_026_no_payable_cost_no_trigger() {
    let mut r = setup();
    r.game.players[0].security.clear();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "GD-OPT");
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt26_026_is_once_per_turn() {
    let mut r = setup();
    let (_t, c) = board(&mut r, 1);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    r.execute_branch(2).expect("don't pay");
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(r.game.pending_selection.is_none(), "OPT spent");
}

#[test]
fn bt26_026_when_attacking_is_not_inherited() {
    let mut r = setup();
    let _t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let h = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.game.pending_selection.is_none());
}
