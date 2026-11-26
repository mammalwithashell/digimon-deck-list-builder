//! ED01-02 Yukinamon — Digimon Lv.6 Blue, Cost 13, DP 13000
//! <Blocker>
//! [When Digivolving] Trash all digivolution cards of 1 of your opponent's Digimon. Then, 1 of their Digimon can't attack until their turn ends.
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon with as many digivolution cards as this Digimon or fewer are deleted, if you have [Valner Dragnogh], trash their top security card. (Q7585)

use crate::alysion_common::{action, advance_turn, delete_perm, fixture, start};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

#[test]
fn ed01_02_when_digivolving_strips_all_locks_attack_and_all_turns_trashes_security() {
    let mut r = start(fixture(&["ED01-02"]).hand(0, &["ED01-02"]));
    let base = r.place_stack(0, &["BLUE-LV5"]); // 1 source after digivolve
    let opp1 = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-5K"]);
    let opp2 = r.place_stack(1, &["PLAIN-4K"]);

    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.game.has_keyword(base, Keyword::Blocker));

    // Trash all digivolution cards of 1 opponent Digimon.
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-5K", None).unwrap();
    assert_eq!(r.game.players[1].battle_area[opp1.index as usize].card_sources.len(), 1);

    // 1 opponent Digimon can't attack until their turn ends.
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-4K", None).unwrap();
    assert!(r.game.modifiers.has(opp2, ModifierType::CannotAttack));

    // Without [Valner Dragnogh], deletion does NOT trash security.
    delete_perm(&mut r, opp2);
    assert_eq!(r.security_count(1), 3);

    // With [Valner Dragnogh], deleting opponent Digimon with MORE sources (2 > 1) does NOT trash security.
    r.place_stack(0, &["VALNER"]);
    let opp_more = r.place_stack(1, &["FILLER", "FILLER", "PLAIN-6K"]);
    delete_perm(&mut r, opp_more);
    assert_eq!(r.security_count(1), 3);

    // Deleting opponent Digimon with <= sources (0 <= 1) trashes top security (OPT).
    delete_perm(&mut r, PermanentHandle { player: 1, index: 0 });
    assert_eq!(r.security_count(1), 2);

    // OPT prevents second trigger in same turn.
    let opp_extra = r.place_stack(1, &["PLAIN-2K"]);
    delete_perm(&mut r, opp_extra);
    assert_eq!(r.security_count(1), 2);

    // Next turn OPT resets (All Turns).
    advance_turn(&mut r);
    let opp_next = r.place_stack(1, &["PLAIN-2K"]);
    delete_perm(&mut r, opp_next);
    assert_eq!(r.security_count(1), 1);
}
