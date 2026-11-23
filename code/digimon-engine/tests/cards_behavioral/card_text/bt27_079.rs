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
    assert!(r.pending_selection().is_none());
}
