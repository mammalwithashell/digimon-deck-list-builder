//! Source: `data/card_bundles/BT27-102.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! [Main] You may play 1 [Kurata's Army] trait Tamer card from your hand or trash without paying the cost. Then, place this card in the battle area. [All Turns] When any Digimon on the field are deleted, ＜Delay＞. ・1 of your Tamers may digivolve into a [Kurata's Army] trait Digimon card in the hand or trash without paying the cost.
//!
//! Security Effect
//! [Security] You may play 1 play cost 5 or lower [Kurata's Army] trait card from your hand or trash without paying the cost.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-102";

#[test]
fn main_can_decline_playing_a_tamer_but_still_places_the_option() {
    let mut r = start(builder(CARD).hand(0, &[CARD, TAMER, BOUNDARY]));
    r.place_on_field(0, TAMER, Some(0));
    let before = r.memory();
    action(&mut r, 0, PLAY_HAND_START);
    let kind = r.pending_selection().unwrap().kind.clone();
    assert!(matches!(kind, SelectionKind::UnionZone { .. }));
    r.assert_prompt(kind, true, 0);
    assert_eq!(candidates(&r), [TAMER]);
    r.decline_prompt().unwrap();
    assert_eq!(before - r.memory(), 3);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
    assert!(r.pending_selection().is_none());
}
