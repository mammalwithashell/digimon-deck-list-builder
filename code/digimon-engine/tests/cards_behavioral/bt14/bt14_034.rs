//! BT14-034 Sukamon — Lv.4 yellow/black Champion, DP 1000, cost 3
//! (digivolve: yellow Lv.3 / 2, black Lv.3 / 2).
//!
//! Printed text (official Bandai DB bundle + DCGO BT14_034.cs):
//! - [Security] At the end of the battle, play this card without paying the cost.
//! - Inherited: [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the
//!   turn.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn runner() -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("BT14-034")
        .expect("BT14-034 loads")
        .add_card(digimon("HOST", "Host Rookie", 3, 2000, 3, vec![CardColor::Yellow], &[]))
        .add_card(digimon("FOE", "Opposing Digimon", 4, 6000, 5, vec![CardColor::Red], &[]))
        .add_card(digimon("FOE2", "Second Opposing Digimon", 4, 7000, 5, vec![CardColor::Red], &[]))
        .add_card(digimon("ATK", "Attacker", 4, 5000, 5, vec![CardColor::Red], &[]))
        .deck(0, &["HOST"; 6])
        .deck(1, &["HOST"; 6])
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

fn delete(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
}

#[test]
fn bt14_034_inherited_on_deletion_gives_an_opponent_digimon_minus_3000_dp_for_the_turn() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT14-034", "HOST"]);
    let foe = r.place_on_field(1, "FOE", None);
    let before = r.effective_dp(foe).unwrap();
    delete(&mut r, host);
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(foe).unwrap(), before - 3000, "-3000 DP");
    // The modifier lasts only for the turn.
    r.end_turn();
    r.end_turn();
    assert_eq!(
        r.effective_dp(foe).unwrap(),
        before,
        "the -3000 DP expired at the end of the turn"
    );
}

#[test]
fn bt14_034_inherited_on_deletion_is_mandatory_and_chooses_among_opponent_digimon() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT14-034", "HOST"]);
    let a = r.place_on_field(1, "FOE", None);
    let b = r.place_on_field(1, "FOE2", None);
    delete(&mut r, host);
    let view = r.pending_selection_view().expect("target prompt");
    assert_eq!(view.valid_action_ids.len(), 2);
    assert!(!r.pending_is_optional(), "the pick is mandatory");
    r.auto_resolve().unwrap();
    let (da, db) = (r.effective_dp(a).unwrap(), r.effective_dp(b).unwrap());
    assert!(
        (da == 3000 && db == 7000) || (da == 6000 && db == 4000),
        "exactly one Digimon got -3000 DP: {da} / {db}"
    );
}

#[test]
fn bt14_034_on_deletion_is_inherited_only_not_printed_on_the_top_card() {
    let mut r = runner();
    let foe = r.place_on_field(1, "FOE", None);
    let top = r.place_on_field(0, "BT14-034", None);
    let before = r.effective_dp(foe).unwrap();
    delete(&mut r, top);
    let _ = r.auto_resolve();
    assert_eq!(
        r.effective_dp(foe).unwrap(),
        before,
        "as the top card its [On Deletion] is not active (inherited text only)"
    );
}

#[test]
fn bt14_034_security_plays_the_card_after_the_battle_ends() {
    let mut r = runner();
    let attacker = r.place_on_field(0, "ATK", Some(0));
    {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT14-034")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[1].security.push(CardSource::new(idx, 1, next));
    }
    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 0, "the security card was checked");
    assert!(
        field_ids(&r, 1).contains(&"BT14-034".to_string()),
        "[Security] played this Digimon free after the battle"
    );
}
