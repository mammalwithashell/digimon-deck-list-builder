//! EX5-054 MetalEtemon — Lv.6 black/yellow Mega, DP 12000, cost 12
//! (digivolve: black Lv.5 / 4, yellow Lv.5 / 4, Lv.5 w/[Etemon]/[Sukamon] in
//! name / 3).
//!
//! Printed text (official Bandai DB bundle + DCGO EX5_054.cs):
//! - [On Play][When Digivolving] Delete 1 of your opponent's play cost 3 or
//!   lower Digimon or Tamers. For each card with [Etemon] or [Sukamon] in its
//!   name in your trash, add 1 to the maximum play cost this effect can choose.
//! - [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by
//!   placing 1 card with [Etemon] or [Sukamon] in its name from your hand on top
//!   of your security stack, you may switch the target of attack to this
//!   Digimon or the player.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::AttackTarget;

fn runner() -> DebugRunner {
    let mut cost3 = digimon("C3", "Cost Three", 3, 3000, 3, vec![CardColor::Red], &[]);
    let cost4 = digimon("C4", "Cost Four", 4, 4000, 4, vec![CardColor::Red], &[]);
    let cost5 = digimon("C5", "Cost Five", 4, 5000, 5, vec![CardColor::Red], &[]);
    let mut tamer3 = card("T3", "Cost Three Tamer", CardKind::Tamer, vec![CardColor::Red]);
    tamer3.play_cost = 3;
    let mut suka = digimon("SUKA", "Sukamon", 4, 4000, 4, vec![CardColor::Black], &[]);
    let ete = digimon("ETE", "Etemon", 5, 6000, 6, vec![CardColor::Black], &[]);
    let r = DebugRunner::builder()
        .dsl_card("EX5-054")
        .expect("EX5-054 loads")
        .add_card(cost3)
        .add_card(cost4)
        .add_card(cost5)
        .add_card(tamer3)
        .add_card(suka)
        .add_card(ete)
        .add_card(digimon("OTHER", "Other Digimon", 3, 2000, 3, vec![CardColor::Black], &[]))
        .add_card(digimon("ATK", "Attacker", 4, 5000, 5, vec![CardColor::Red], &[]))
        .add_card(digimon("SEC", "Security Card", 3, 1000, 3, vec![CardColor::Red], &[]))
        .deck(0, &["OTHER"; 6])
        .deck(1, &["OTHER"; 6])
        .security(0, &["SEC"; 3])
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

fn candidate_count(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.len())
        .unwrap_or(0)
}

#[test]
fn ex5_054_on_play_deletes_a_play_cost_3_or_lower_digimon_or_tamer() {
    let mut r = runner();
    let m = r.place_on_field(0, "EX5-054", Some(0));
    let c3 = r.place_on_field(1, "C3", None);
    let c4 = r.place_on_field(1, "C4", None);
    let t3 = r.place_on_field(1, "T3", None);
    r.fire_on_play(0, m.index as usize);
    // Cost-3 Digimon and the cost-3 Tamer qualify; the cost-4 Digimon does not.
    assert_eq!(candidate_count(&r), 2, "play cost <= 3: the Digimon and the Tamer");
    assert!(!r.pending_is_optional(), "the delete is mandatory");
    r.auto_resolve().unwrap();
    let field = field_ids(&r, 1);
    assert!(field.contains(&"C4".to_string()), "the cost-4 Digimon is untouched");
    assert_eq!(field.len(), 2, "exactly one of the two eligible permanents was deleted");
}

#[test]
fn ex5_054_cap_rises_by_one_per_etemon_or_sukamon_name_card_in_your_trash() {
    let mut r = runner();
    let m = r.place_on_field(0, "EX5-054", Some(0));
    r.place_on_field(1, "C3", None);
    r.place_on_field(1, "C4", None);
    r.place_on_field(1, "C5", None);
    r.inject_trash(0, "SUKA");
    r.inject_trash(0, "ETE");
    r.inject_trash(0, "OTHER");
    r.fire_on_play(0, m.index as usize);
    // Cap = 3 + 2 (Sukamon + Etemon in trash; OTHER does not count) = 5.
    assert_eq!(candidate_count(&r), 3, "cost 3, 4 and 5 are all in range at cap 5");
}

#[test]
fn ex5_054_cap_is_three_with_no_etemon_or_sukamon_in_trash() {
    let mut r = runner();
    let m = r.place_on_field(0, "EX5-054", Some(0));
    r.place_on_field(1, "C4", None);
    r.place_on_field(1, "C5", None);
    r.inject_trash(0, "OTHER");
    r.fire_on_play(0, m.index as usize);
    assert!(r.pending_selection().is_none(), "nothing at play cost 3 or lower: no target");
    assert_eq!(field_ids(&r, 1).len(), 2);
}

#[test]
fn ex5_054_cap_counts_only_your_own_trash() {
    let mut r = runner();
    let m = r.place_on_field(0, "EX5-054", Some(0));
    r.place_on_field(1, "C4", None);
    r.inject_trash(1, "SUKA");
    r.fire_on_play(0, m.index as usize);
    assert!(r.pending_selection().is_none(), "the opponent's trash does not raise the cap");
}

#[test]
fn ex5_054_digivolves_for_3_from_a_lv5_with_etemon_or_sukamon_in_its_name() {
    use digimon_engine::enums::{GamePhase, PlaySource};
    // The printed alt route "Lv.5 w/[Etemon]/[Sukamon] in name: Cost 3" has no
    // colour gate: a red Lv.5 Etemon still qualifies.
    let r = runner();
    let compiled = r.compiled_card("EX5-054").expect("compiled");
    let paths: Vec<_> = compiled.alt_paths.iter().collect();
    assert_eq!(paths.len(), 3, "two printed circles + the name-gated route");
}

/// Opponent (seat 1) attacks seat 0's player while MetalEtemon is in play.
fn opponent_attack_opens(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    r.game.turn_count = 1; // seat 1's turn
    r.set_first_player(1);
    let attacker = r.place_on_field(1, "ATK", Some(0));
    r.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    r.game.drain_effect_queue();
    attacker
}

/// Open the hand pick, assert it is the optional, matching-name-only pick, and
/// choose the (single) legal card.
fn pick_the_name_card(r: &mut DebugRunner) {
    let view = r
        .pending_selection_view()
        .expect("[Opponent's Turn] ... the hand pick opens");
    assert!(view.is_optional, "'by placing ..., you may' (the pick is declinable)");
    assert_eq!(
        view.valid_action_ids.iter().filter(|a| **a != PASS).count(),
        1,
        "only the [Etemon]/[Sukamon]-name card is a legal pick"
    );
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .unwrap();
    r.execute_action(view.selecting_player, pick).unwrap();
}

// NOTE: once the effect resolves the engine carries the attack on to its
// result, so the switch is judged by what the attack DID: an attack on this
// Digimon (DP 12000) deletes the 5000 DP attacker and never checks security;
// an attack on the player checks the top security card (the freshly placed one).

#[test]
fn ex5_054_opponent_attack_places_hand_card_on_security_top_and_switches_to_this_digimon() {
    let mut r = runner();
    let m = r.place_on_field(0, "EX5-054", Some(0));
    r.add_to_hand(0, "SUKA");
    r.add_to_hand(0, "OTHER");
    let sec_before = r.security_count(0);
    opponent_attack_opens(&mut r);
    pick_the_name_card(&mut r);
    // The 3-way choice: this Digimon / the player / do not switch.
    let view = r.pending_selection_view().expect("switch choice");
    assert_eq!(view.valid_action_ids.len(), 3);
    r.execute_branch(0).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        security_top_first(&r, 0)[0],
        "SUKA",
        "the card sits on TOP of the security stack"
    );
    assert_eq!(
        r.security_count(0),
        sec_before + 1,
        "placed on security and the attack on MetalEtemon never checked security"
    );
    assert!(!hand_ids(&r, 0).contains(&"SUKA".to_string()));
    assert!(
        trash_ids(&r, 1).contains(&"ATK".to_string()),
        "the attacker ran into MetalEtemon (12000 DP) and was deleted"
    );
}

#[test]
fn ex5_054_not_switching_still_pays_the_security_placement() {
    let mut r = runner();
    r.place_on_field(0, "EX5-054", Some(0));
    r.add_to_hand(0, "SUKA");
    let sec_before = r.security_count(0);
    opponent_attack_opens(&mut r);
    pick_the_name_card(&mut r);
    // Third branch: "Do not switch".
    r.execute_branch(2).unwrap();
    let _ = r.auto_resolve();
    assert!(!hand_ids(&r, 0).contains(&"SUKA".to_string()), "the card left the hand (cost paid)");
    // +1 placed, -1 checked by the attack that still targets the player.
    assert_eq!(r.security_count(0), sec_before);
    assert!(
        !trash_ids(&r, 1).contains(&"ATK".to_string()),
        "the attack was not switched onto MetalEtemon"
    );
}

#[test]
fn ex5_054_declining_the_hand_pick_pays_nothing_and_switches_nothing() {
    let mut r = runner();
    r.place_on_field(0, "EX5-054", Some(0));
    r.add_to_hand(0, "ETE");
    let sec_before = r.security_count(0);
    opponent_attack_opens(&mut r);
    let view = r.pending_selection_view().expect("hand pick");
    r.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"ETE".to_string()), "nothing was placed");
    assert_eq!(r.security_count(0), sec_before - 1, "the unswitched attack checked security");
}

#[test]
fn ex5_054_switch_to_the_player_keeps_the_player_as_the_target() {
    let mut r = runner();
    r.place_on_field(0, "EX5-054", Some(0));
    r.add_to_hand(0, "SUKA");
    let sec_before = r.security_count(0);
    opponent_attack_opens(&mut r);
    pick_the_name_card(&mut r);
    r.execute_branch(1).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec_before, "the attack hit the player (+1 placed, -1 checked)");
    assert!(!trash_ids(&r, 1).contains(&"ATK".to_string()));
}

#[test]
fn ex5_054_attack_effect_is_not_offered_without_a_hand_card() {
    let mut r = runner();
    r.place_on_field(0, "EX5-054", Some(0));
    opponent_attack_opens(&mut r);
    assert!(r.pending_selection().is_none(), "no hand card: the effect cannot activate");
}
