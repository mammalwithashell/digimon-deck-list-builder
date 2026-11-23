//! Source: `data/card_bundles/BT27-075.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! When this card would be played, by deleting 1 of your level 2 Digimon in the breeding area, reduce the cost by 2. ＜Blocker＞ [All Turns] This Digimon can't digivolve. [On Deletion] By returning 2 cards with [Gizmon] in their names from your trash to the bottom of the deck, you may play 1 [Gizmon: AT] from your trash without paying the cost.
//!
//! Official Q&A
//! - No, you can't.

use super::bt27_common::*;
use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::enums::Keyword;

const CARD: &str = "BT27-075";

#[test]
fn playing_with_no_egg_costs_three_and_grants_blocker() {
    let mut r = start(builder(CARD).hand(0, &[CARD, EVOLUTION]));
    let before = r.memory();
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(before - r.memory(), 3);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
        0.0
    );
    assert!(r.pending_selection().is_none());
}
