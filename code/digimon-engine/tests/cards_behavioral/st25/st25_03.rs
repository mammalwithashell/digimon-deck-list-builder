//! ST25-03 Liskmon — Digimon Lv.4 Red, Cost 4, DP 5000
//! [Digivolve] Lv.3 w/[Gemmon] in text: Cost 2
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Kanata Hondo] from your hand without paying the cost.
//! Inherited: <Raid>

use crate::alysion_common::{action, assert_gemmon_text_digivolve, candidates, fixture, ids, start};
use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn st25_03_alt_digivolve_from_gemmon_text_lv3() {
    assert_gemmon_text_digivolve("ST25-03", 3, 2);
}

#[test]
fn st25_03_plays_kanata_hondo_free_when_at_most_one_tamer_and_grants_inherited_raid() {
    for accept in [false, true] {
        let mut r = start(
            fixture(&["ST25-03"]).hand(0, &["ST25-03", "KANATA", "RED-TAMER"]),
        );
        r.place_stack(0, &["RED-TAMER"]);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), ["KANATA"]);
        if accept {
            r.choose_prompt_card("KANATA", None).unwrap();
            assert_eq!(r.battle_area_size(0), 3);
            assert_eq!(ids(&r, &r.game.players[0].battle_area[2].card_sources), ["KANATA"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.battle_area_size(0), 2);
        }
    }

    // With 2 Tamers already on field, [When Digivolving] does not prompt.
    let mut r = start(fixture(&["ST25-03"]).hand(0, &["ST25-03", "KANATA"]));
    r.place_stack(0, &["RED-TAMER"]);
    r.place_stack(0, &["RED-TAMER"]);
    let base = r.place_stack(0, &["RED-LV3"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.pending_selection().is_none());

    // Inherited <Raid>.
    let host = r.place_stack(0, &["ST25-03", "PLAIN-5K"]);
    assert!(r.game.has_keyword(host, Keyword::Raid));
}
