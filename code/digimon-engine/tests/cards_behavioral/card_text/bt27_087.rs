//! Source: `data/card_bundles/BT27-087.md` (printed text).
//!
//! ### Effect
//! [Start of Your Main Phase] If you have a Digimon with the [Chronicle] trait, gain 1 memory. [On Play] Activate 1 of the effects below: ・Reveal the top 3 cards of your deck. Add 1 [X Antibody] or [Chronicle] trait card among them to the hand. Trash the rest. ・You may play 1 level 3 [Chronicle] trait Digimon card from your hand to your breeding area without paying the cost.
//!
//! ### Security Effect
//! [Security] Play this card without paying the cost.
//!
//! ## Official Q&A
//! - No. You can only have 1 Digimon in the breeding area at a time.

use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-087";

#[test]
fn main_phase_gains_memory_only_with_a_chronicle_digimon() {
    for matching in [false, true] {
        let mut rookie = digimon("CHRONICLE", 3, 2000, false);
        rookie.traits.push("Chronicle".into());
        let mut r = builder(CARD).add_card(rookie).memory(5).start();
        r.place_on_field(0, CARD, Some(0));
        r.place_on_field(0, if matching { "CHRONICLE" } else { "TEXT-3" }, Some(0));
        r.game.enter_main_phase();
        assert_eq!(r.memory(), if matching { 6 } else { 5 });
    }
}

#[test]
fn security_plays_the_tamer_for_free_and_reveal_trashes_remaining_cards() {
    let mut antibody = digimon("X-ANTIBODY", 3, 2000, false);
    antibody.traits.push("X Antibody".into());
    let mut r = start(
        builder(CARD)
            .add_card(antibody)
            .deck(1, &["TEXT-3", "TEXT-4", "X-ANTIBODY"])
            .security(1, &[CARD]),
    );
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    r.attack_player(attacker, 1, false);
    r.assert_prompt(SelectionKind::EffectChoice, false, 1);
    r.execute_branch(0).unwrap();
    assert_eq!(candidates(&r), ["X-ANTIBODY"]);
    r.choose_prompt_card("X-ANTIBODY", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].hand), ["X-ANTIBODY"]);
    let mut trash = ids(&r, &r.game.players[1].trash);
    trash.sort();
    assert_eq!(trash, ["TEXT-3", "TEXT-4"]);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.memory(), 10);
}

#[test]
fn on_play_can_choose_to_put_a_chronicle_rookie_in_breeding() {
    let mut rookie = digimon("CHRONICLE", 3, 2000, false);
    rookie.traits.push("Chronicle".into());
    let mut r = start(
        builder(CARD)
            .add_card(rookie)
            .hand(0, &[CARD, "CHRONICLE", "TEXT-3"]),
    );
    r.play(0, 0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["CHRONICLE"]);
    r.choose_prompt_card("CHRONICLE", None).unwrap();
    assert!(r.game.players[0].breeding_area.is_some());
    assert_eq!(r.memory(), 6);
}
