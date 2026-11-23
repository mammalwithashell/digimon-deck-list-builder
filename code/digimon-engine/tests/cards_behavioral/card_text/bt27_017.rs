//! Source: `data/card_bundles/BT27-017.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Red Lv.5 / cost 4
//! - Purple Lv.5 / cost 4
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] [Stegomon] w/[Bio] trait: Cost 5 [Digivolve] Tamer w/[Kurata's Army] trait: Cost 7
//!
//! Printed text (official)
//! Effect
//! ＜Security A. +1＞ ＜Armor Purge＞ ＜Engage＞ (At the end of your turn, this Digimon may attack.) [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with as much DP as this Digimon or less. [All Turns] This Digimon can't digivolve. [Opponent's Turn] While this Digimon is suspended, none of your opponent's Digimon can attack players.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-017";

#[test]
fn on_play_deletes_at_most_its_own_dp() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(1, BOUNDARY, Some(0));
    r.place_on_field(1, HIGH, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [BOUNDARY]);
    r.choose_prompt_card(BOUNDARY, None).unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(r.trash_size(1), 1);
    assert!(r.pending_selection().is_none());
}

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_does_not_trigger_when_digivolving() {
    tamer_evolution(
        CARD,
        7,
        &[
            Keyword::ArmorPurge,
            Keyword::Engage,
            Keyword::SecurityAttackPlus(1),
        ],
    );
}
