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

#[test]
fn deletion_recycles_exactly_two_gizmon_to_play_at() {
    gizmon_recycles_two_to_play_from_trash(CARD, GIZMON);
}

#[test]
#[ignore = "G-ENGINE-BREEDING-DELETION-PAYMENT"]
fn playing_may_delete_the_level_two_in_breeding_for_two_memory() {
    let mut egg = digimon_engine::debug_runner::make_test_card("TEXT-EGG", "Level 2");
    egg.card_kind = digimon_engine::enums::CardKind::DigiEgg;
    egg.level = Some(2);
    let mut r = start(builder(CARD).add_card(egg).hand(0, &[CARD]));
    r.place_in_breeding(0, "TEXT-EGG");
    let before = r.memory();
    action(&mut r, 0, PLAY_HAND_START);
    r.accept_optional_trigger().unwrap();
    action(
        &mut r,
        0,
        digimon_engine::action::space::BREEDING_SELECT_PLAYER_0,
    );
    assert_eq!(before - r.memory(), 1);
    assert!(r.game.players[0].breeding_area.is_none());
}
