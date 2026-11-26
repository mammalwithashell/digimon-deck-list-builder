//! ST26-05 Yukinamon / Ice Beast Caricatures — DUAL (Digimon Lv.6 Blue DP 12000 / Option Cost 5 Blue)
//! <Iceclad> <Piercing> <Blocker>
//! [When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon. Then, if your opponent has no Digimon with digivolution cards, 1 of your [Ice-Snow] trait Digimon may attack without suspending. (Q7580, Q7581)
//! [When Attacking] You may return 1 of your opponent's Digimon with no digivolution cards to the bottom of the deck.
//! Option [Main] Trash all cards from under 1 of your opponent's Digimon or Tamers. Then, 2 of their Digimon or Tamers can't suspend until their turn ends. <Arts Digivolve>

use crate::alysion_common::{action, fixture, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;

#[test]
fn st26_05_when_digivolving_strips_and_may_attack_without_suspending_plus_when_attacking_bottom_deck() {
    let mut r = start(fixture(&["ST26-05"]).hand(0, &["ST26-05"]));
    let base = r.place_stack(0, &["BLUE-LV5"]);
    let opp = r.place_stack(1, &["FILLER", "PLAIN-5K"]); // 1 source -> 0 after strip

    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.game.has_keyword(base, Keyword::Iceclad));
    assert!(r.game.has_keyword(base, Keyword::Piercing));
    assert!(r.game.has_keyword(base, Keyword::Blocker));

    // Trash 1 available source.
    let src_act = r.pending_selection().unwrap().valid_action_ids[0];
    action(&mut r, 0, src_act);
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 1);

    // Opponent has no Digimon with sources -> [Ice-Snow] Digimon may attack without suspending!
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card("ST26-05", None).unwrap();
    // Choose Security as attack target.
    action(&mut r, 0, encode_attack(base.index as u16, SECURITY_TARGET));
    // [When Attacking] may return opponent's sourceless Digimon to bottom of deck.
    r.assert_prompt(SelectionKind::OppField, true, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 0);
    // Attacked WITHOUT suspending!
    assert!(!r.game.players[0].battle_area[base.index as usize].is_suspended);
    assert_eq!(r.security_count(1), 2);
}

#[test]
fn st26_05_option_side_strips_all_under_and_locks_two_permanents_then_arts_digivolve() {
    let mut r = start(fixture(&["ST26-05"]).hand(0, &["ST26-05"]));
    let _lv5 = r.place_stack(0, &["BLUE-LV5"]);
    let opp_digimon = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);
    let opp_tamer = r.place_stack(1, &["BLUE-TAMER"]);

    action(&mut r, 0, PLAY_HAND_START);
    // Trash all cards from under 1 opponent Digimon or Tamer.
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp_digimon.index as usize].card_sources.len(), 1);

    // Lock 2 of opponent's Digimon or Tamers from suspending.
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    r.choose_prompt_card("BLUE-TAMER", None).unwrap();
    assert!(r.game.modifiers.has(opp_digimon, ModifierType::CannotSuspend));
    assert!(r.game.modifiers.has(opp_tamer, ModifierType::CannotSuspend));

    // <Arts Digivolve> prompt appears after [Main] finishes.
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    action(&mut r, 0, PASS);
    assert!(r.pending_selection().is_none());
}
