//! Source: `data/card_bundles/BT27-026.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Blue Lv.3 / cost 3
//! - Purple Lv.3 / cost 3
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] Tamer w/[Kurata's Army] trait: Cost 2
//!
//! Printed text (official)
//! Effect
//! ＜Armor Purge＞ [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may trash any 2 digivolution cards from your opponent's Digimon. Then, by choosing 1 of your Digimon, delete the chosen Digimon and 1 of their Digimon can't suspend until their turn ends.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_costs_two_and_does_not_trigger() {
    tamer_evolution("BT27-026", 2, &[Keyword::ArmorPurge]);
}

#[test]
fn may_decline_choosing_a_digimon_to_delete_without_opposing_sources() {
    let mut r = start(builder("BT27-026").hand(0, &["BT27-026"]));
    r.place_on_field(1, HIGH, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BT27-026"]);
    r.decline_prompt().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(r.pending_selection().is_none());
}
