//! ST26-03 Tenkomon — Digimon Lv.4 Blue, Cost 4, DP 5000
//! (Rule) Trait: Has [Ice-Snow] Type.
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Valner Dragnogh] from your hand without paying the cost.
//! Inherited: [End of Attack] [Once Per Turn] Gain 1 memory.

use crate::alysion_common::{action, candidates, fixture, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn st26_03_plays_valner_free_at_most_one_tamer_and_inherited_gains_memory() {
    for accept in [false, true] {
        let mut r = start(fixture(&["ST26-03"]).hand(0, &["ST26-03", "VALNER", "BLUE-TAMER"]));
        r.place_stack(0, &["BLUE-TAMER"]);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), ["VALNER"]);
        if accept {
            r.choose_prompt_card("VALNER", None).unwrap();
            assert_eq!(r.battle_area_size(0), 3);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.battle_area_size(0), 2);
        }
    }

    // With 2 Tamers, [When Digivolving] does not prompt.
    let mut r = start(fixture(&["ST26-03"]).hand(0, &["ST26-03", "VALNER"]));
    r.place_stack(0, &["BLUE-TAMER"]);
    r.place_stack(0, &["BLUE-TAMER"]);
    let base = r.place_stack(0, &["BLUE-LV3"]);
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.pending_selection().is_none());

    // Inherited [End of Attack][OPT] +1 memory.
    let mut r2 = start(fixture(&["ST26-03"]).memory(5));
    let attacker = r2.place_stack(0, &["ST26-03", "PLAIN-12K"]);
    let mem_before = r2.memory();
    action(&mut r2, 0, encode_attack(attacker.index as u16, SECURITY_TARGET));
    assert_eq!(r2.memory(), mem_before + 1);
}
