//! Source: `data/card_bundles/BT27-037.md` (printed text).
//!
//! ### Effect
//! ＜Detach ([TS] trait)＞ [All Turns] [Once Per Turn] When any of your Digimon get linked, ＜Draw 1＞.
//!
//! ### Inherited Effect
//! ＜Detach ([TS] trait)＞ (When this Digimon would leave the battle area other than by your effects, by trashing 1 of its specified link cards, it doesn't leave.)

use super::common::*;
use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-037";

#[test]
fn guardromon_draws_once_when_an_ally_gets_linked() {
    let mut r = start(
        builder(CARD)
            .hand(0, &["TEXT-3", "TEXT-3"])
            .link_requirement("TEXT-3", 0),
    );
    r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "TEXT-4", Some(0));
    action(&mut r, 0, HAND_EFFECT_START);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert_eq!(r.hand_size(0), 2);
    r.place_on_field(0, "TEXT-5", Some(0));
    action(&mut r, 0, HAND_EFFECT_START);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn detach_discards_only_a_ts_link_to_prevent_battle_deletion() {
    for inherited in [false, true] {
        let mut ts = digimon("TS-LINK", 3, 2000, false);
        ts.traits.push("TS".into());
        let mut r = start(
            builder(CARD)
                .add_card(ts)
                .hand(0, &["TS-LINK"])
                .link_requirement("TS-LINK", 0),
        );
        let host = if inherited {
            r.place_stack(0, &[CARD, "TEXT-5"])
        } else {
            r.place_on_field(0, CARD, Some(0))
        };
        let enemy = r.place_on_field(1, "TEXT-6", Some(0));
        action(&mut r, 0, HAND_EFFECT_START);
        r.choose_prompt_card(if inherited { "TEXT-5" } else { CARD }, None)
            .unwrap();
        r.battle_digimon(host, enemy);
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.accept_optional_trigger().unwrap();
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(0).unwrap();
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(ids(&r, &r.game.players[0].trash), ["TS-LINK"]);
    }
}
