//! Source: `data/card_bundles/BT27-047.md` (printed text).
//!
//! ### Effect
//! [When Moving] [On Play] Suspend 1 of your opponent's Digimon. [When Attacking] [Once Per Turn] This Digimon gets +5000 DP for the turn.
//!
//! ### Inherited Effect
//! ＜Piercing＞ (When this Digimon deletes your opponent's Digimon in battle while attacking, it checks security before the attack ends.)

use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-047";

#[test]
fn on_play_suspends_selected_enemy_and_attack_gets_five_thousand_dp() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let target = r.place_on_field(1, "TEXT-5", Some(0));
    r.play(0, 0);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert!(r.game.players[1].battle_area[target.index as usize].is_suspended);
    next_turn(&mut r);
    next_turn(&mut r);
    let source = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.attack_player(source, 1, false);
    assert_eq!(r.dp_of(source), Some(10000));
}

#[test]
fn inherited_piercing_checks_security_after_deleting_a_defender() {
    let mut r = start(builder(CARD));
    let carrier = r.place_stack(0, &[CARD, "TEXT-6"]);
    let defender = r.place_on_field(1, "TEXT-4", Some(0));
    r.game.suspend(defender);
    r.attack_digimon(carrier, defender, false);
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.security_count(1), 4);
}

#[test]
fn moving_from_breeding_suspends_an_opponents_digimon_not_their_tamer() {
    let mut r = start(builder(CARD));
    let target = r.place_on_field(1, "TEXT-5", Some(0));
    r.place_on_field(1, "TEXT-TAMER", Some(0));
    r.set_phase(digimon_engine::enums::GamePhase::Breeding);
    r.place_in_breeding(0, CARD);
    assert!(r.move_from_breeding(0));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert!(r.game.players[1].battle_area[target.index as usize].is_suspended);
}
