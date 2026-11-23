//! Source: `data/card_bundles/BT27-076.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! When this card would be played, by deleting 1 of your level 3 Digimon, reduce the cost by 4. [On Play] You may play 1 [Akihiro Kurata] from your hand without paying the cost. [All Turns] This Digimon can't digivolve. [On Deletion] By returning 2 cards with [Gizmon] in their names from your trash to the bottom of the deck, you may play 1 [Gizmon: XT] from your trash without paying the cost.
//!
//! Official Q&A
//! - Yes, it can.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-076";

#[test]
fn may_play_akihiro_kurata_from_hand_for_free_or_decline() {
    for accept in [true, false] {
        let mut r = start(builder(CARD).hand(0, &[CARD, KURATA, TAMER]));
        let before = r.memory();
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), [KURATA]);
        if accept {
            r.choose_prompt_card(KURATA, None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.game.players[0].battle_area.len(), 1 + usize::from(accept));
        assert_eq!(before - r.memory(), 6);
        assert!(r.pending_selection().is_none());
    }
}
