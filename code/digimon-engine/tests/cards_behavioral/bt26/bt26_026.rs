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
        // A purple [Glowing Dawn] Option: its colour requirement is unmet on
        // a yellow-only board (CR 4.3 4-22).
        .add_card({
            let mut o = gd_option("GD-PURPLE", 5);
            o.colors = vec![CardColor::Purple];
            o
        })
        // Use cost 15 / 16 reduced by 2 at memory 3: 13 → -10 (payable),
        // 14 → -11 (not payable).
        .add_card(gd_option("GD-15", 15))
        .add_card(gd_option("GD-16", 16))
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

// ─── Hand-pick candidate legality (select-hand playability audit) ────────────
// DCGO BT26_026.cs:52-56 offers only `IsOption && EqualsTraits("Glowing Dawn")
// && !CanNotPlayThisOption && Owner.MaxMemoryCost >= GetCostItself - 2`:
// `CanNotPlayThisOption` covers a "can't use Option cards" lock and an unmet
// colour requirement (CR 4.3 4-22; Q1500 / Q2785 — an effect-driven use is a
// use), and the memory check refuses a reduced cost the player can't pay. A
// card the use would refuse is never offered (CR 4.3 1-3-2).

/// Fire [When Attacking], pay `branch` (0 = a Tamer's face-down card, 1 = top
/// security) and return the hand-pick candidates' card ids.
fn offered_after_paying(r: &mut DebugRunner, branch: usize, hand: &[&str]) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    let (_t, c) = board(r, 1);
    for id in hand {
        push_hand(r, 0, id);
    }
    fire(r, EffectTiming::WhenAttacking, c);
    r.execute_branch(branch).expect("pay the cost");
    if branch == 0 {
        pick_first(r, 0); // the Tamer
    }
    let Some(v) = r.pending_selection_view() else {
        return Vec::new();
    };
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .is_none_or(|p| p.card_choice.is_none()),
        "hand pick uses PLAY_HAND_START action ids"
    );
    let ids = hand_ids(r, 0);
    v.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS && a >= PLAY_HAND_START && ((a - PLAY_HAND_START) as usize) < ids.len())
        .map(|a| ids[(a - PLAY_HAND_START) as usize].clone())
        .collect()
}

/// Security branch: a purple [Glowing Dawn] Option with no purple Digimon or
/// Tamer is not offered.
#[test]
fn bt26_026_security_branch_excludes_option_whose_colour_is_unmet() {
    let mut r = setup();
    let offered = offered_after_paying(&mut r, 1, &["GD-PURPLE", "GD-OPT"]);
    assert_eq!(offered, vec!["GD-OPT".to_string()]);
}

/// Face-down branch: the same colour check applies to its own hand pick.
#[test]
fn bt26_026_face_down_branch_excludes_option_whose_colour_is_unmet() {
    let mut r = setup();
    let offered = offered_after_paying(&mut r, 0, &["GD-PURPLE", "GD-OPT"]);
    assert_eq!(offered, vec!["GD-OPT".to_string()]);
}

/// "You can't use Option cards": no [Glowing Dawn] Option is offered.
#[test]
fn bt26_026_excludes_options_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_after_paying(&mut r, 1, &["GD-OPT"]);
    assert!(offered.is_empty(), "{offered:?}");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GD-OPT".to_string()]);
}

/// The use pays the cost reduced by 2: an Option whose reduced cost would take
/// memory past -10 is not offered.
#[test]
fn bt26_026_security_branch_excludes_unaffordable_option() {
    let mut r = setup();
    let offered = offered_after_paying(&mut r, 1, &["GD-15", "GD-16"]);
    assert_eq!(offered, vec!["GD-15".to_string()]);
}

/// Face-down branch: the same affordability check.
#[test]
fn bt26_026_face_down_branch_excludes_unaffordable_option() {
    let mut r = setup();
    let offered = offered_after_paying(&mut r, 0, &["GD-15", "GD-16"]);
    assert_eq!(offered, vec!["GD-15".to_string()]);
}

#[test]
fn bt26_026_when_attacking_is_not_inherited() {
    let mut r = setup();
    let _t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let h = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.game.pending_selection.is_none());
}
