//! Source: `data/card_bundles/BT27-079.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! When this card would be played, by deleting 1 of your level 4 Digimon, reduce the cost by 6. ＜Blocker＞ ＜Retaliation＞ [All Turns] This Digimon can't digivolve. [On Deletion] You may play 1 play cost 5 or lower [Kurata's Army] trait card from your hand or trash without paying the cost.
//!
//! Official Q&A
//! - Yes, it can.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-079";

#[test]
fn playing_without_a_level_four_costs_nine_and_grants_both_keywords() {
    let mut r = start(builder("BT27-079").hand(0, &["BT27-079"]));
    let before = r.memory();
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(before - r.memory(), 9);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert!(r.game.has_keyword(h, Keyword::Retaliation));
    assert_cannot_digivolve(&r, h);
    assert!(r.pending_selection().is_none());
}

#[test]
fn retaliation_deletes_the_digimon_that_won_the_battle() {
    let mut r = start(builder(CARD));
    delete_in_battle(&mut r, CARD);
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(ids(&r, &r.game.players[1].trash), [HIGH]);
    assert!(r.pending_selection().is_none());
}

#[test]
fn level_four_in_battle_area_pays_fixed_reduction() {
    fixed_play_cost_payment(CARD, 4, 9, 6);
}

#[test]
fn deletion_offers_kuratas_army_from_hand_or_trash_at_cost_five_or_less() {
    for choice in [Some(TAMER), Some(KURATA), None] {
        let mut r = start(builder(CARD).hand(0, &[TAMER, BOUNDARY, CARD]));
        r.inject_trash(0, KURATA);
        delete_in_battle(&mut r, CARD);
        let kind = r.pending_selection().unwrap().kind.clone();
        assert!(matches!(kind, SelectionKind::UnionZone { .. }));
        r.assert_prompt(kind, true, 0);
        assert_eq!(candidates(&r), [KURATA, TAMER]);
        let before = r.memory();
        if let Some(id) = choice {
            r.choose_prompt_card(id, None).unwrap();
            assert_eq!(
                ids(&r, &r.game.players[0].battle_area[0].card_sources),
                [id]
            );
        } else {
            r.decline_prompt().unwrap();
            assert!(r.game.players[0].battle_area.is_empty());
        }
        assert_eq!(r.memory(), before);
        assert!(r.pending_selection().is_none());
    }
}
