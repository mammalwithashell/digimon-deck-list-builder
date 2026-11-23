//! Source: `data/card_bundles/BT27-007.md` (printed text).
//!
//! ### Effect
//! [Start of Your Main Phase] By trashing 1 card with the [Flame] or [Machine] trait from your hand, ＜Draw 1＞ and gain 1 memory. (Rule) Trait: Has [Flame] Type.
//!
//! ### Inherited Effect
//! [On Deletion] Delete 1 of your opponent's Digimon with 6000 DP or less.

use super::common::*;
use digimon_engine::events::GameEvent;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-007";

#[test]
fn main_phase_trash_payment_draws_and_gains_memory() {
    let mut material = digimon("MACHINE", 3, 2000, false);
    material.traits.push("Machine".into());
    let mut r = builder(CARD)
        .add_card(material)
        .hand(0, &["MACHINE", "TEXT-4"])
        .memory(5)
        .start();
    r.place_on_field(0, CARD, Some(0));
    let memory = r.memory();
    r.game.enter_main_phase();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["MACHINE"]);
    r.choose_prompt_card("MACHINE", None).unwrap();
    assert_eq!(r.memory(), memory + 1);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["MACHINE"]);
    assert!(r.game.events.iter().any(
        |event| matches!(event, GameEvent::Trash { player: 0, card_id, .. } if card_id == "MACHINE")
    ));
    assert_eq!(r.hand_size(0), 2);
}

#[test]
fn declining_flame_payment_leaves_hand_and_memory_unchanged() {
    let mut flame = digimon("FLAME", 3, 2000, false);
    flame.traits.push("Flame".into());
    let mut r = builder(CARD)
        .add_card(flame)
        .hand(0, &["FLAME"])
        .memory(5)
        .start();
    r.place_on_field(0, CARD, Some(0));
    r.game.enter_main_phase();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["FLAME"]);
    let hand = ids(&r, &r.game.players[0].hand);
    r.decline_prompt().unwrap();
    assert_eq!(ids(&r, &r.game.players[0].hand), hand);
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.memory(), 5);
}

#[test]
fn inherited_deletion_only_targets_digimon_at_or_below_6000_dp() {
    let mut r = start(builder(CARD));
    let source = r.place_stack(0, &[CARD, "TEXT-4"]);
    let big = r.place_on_field(1, "TEXT-6", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.game.suspend(big);
    r.attack_digimon(source, big, false);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(r.dp_of(big), Some(12000));
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3"]);
}
