//! ST25-05 Rubeusdramon / Burning Drive — DUAL (Digimon Lv.6 Red DP 12000 / Option Cost 5 Red)
//! [Digivolve] Lv.5 w/[Gemmon] in text: Cost 3
//! <Progress> <Piercing>
//! [When Digivolving] Delete up to 7000 DP total worth of your opponent's Digimon. For every 8000 DP this Digimon has, add 4000 to this DP deletion effect's maximum. (Q7554, Q7555)
//! [Your Turn] For every 8000 DP this Digimon has, it gains <Security A. +1>. (Q7556)
//! Option [Main] 1 of your [ALYSION] trait Digimon gets +10000 DP for the turn. Then, delete 1 of your opponent's Digimon with as much DP as 1 of your Digimon or less. (Q7557, Q7558)
//! <Arts Digivolve>

use crate::alysion_common::{action, assert_gemmon_text_digivolve, fixture, start};
use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START,
};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn st25_05_alt_digivolve_from_gemmon_text_lv5() {
    assert_gemmon_text_digivolve("ST25-05", 5, 3);
}

#[test]
fn st25_05_when_digivolving_scales_dp_budget_and_requires_at_least_one_pick() {
    // At 12000 DP: floor(12000/8000) = 1 -> budget = 7000 + 4000 = 11000 DP.
    let mut r = start(fixture(&["ST25-05"]).hand(0, &["ST25-05"]));
    let base = r.place_stack(0, &["RED-LV5"]);
    r.place_stack(1, &["PLAIN-6K"]);
    r.place_stack(1, &["PLAIN-5K"]);
    r.place_stack(1, &["PLAIN-12K"]); // exceeds 11000 DP budget

    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert!(r.game.has_keyword(base, Keyword::Progress));
    assert!(r.game.has_keyword(base, Keyword::Piercing));
    // First pick is mandatory (Q7555: cannot decline with 0 picks).
    r.assert_prompt(SelectionKind::DpBudget { remaining_dp: 11000, picked: 0 }, false, 0);
    action(&mut r, 0, ATTACK_START);
    // Second pick is optional (Q7554: may stop after 1 pick or continue up to 11000).
    r.assert_prompt(SelectionKind::DpBudget { remaining_dp: 5000, picked: 1 }, true, 0);
    action(&mut r, 0, ATTACK_START + 1);
    assert!(r.pending_selection().is_none());
    // Both 6K + 5K (= 11000) deleted; 12K survives.
    assert_eq!(r.battle_area_size(1), 1);

    // Dynamic [Your Turn] SA+1 per 8000 DP (Q7556): 12000 DP => +1 (2 checks); 16000 DP => +2 (3 checks); 7000 DP => +0 (1 check).
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    assert_eq!(r.game.effective_security_strike(base), 2);
    r.game.modifiers.add(base, ModifierEntry::simple(ModifierType::ChangeDp, 4000, Expiry::EndOfTurn, 0));
    assert_eq!(r.game.effective_security_strike(base), 3);
    r.game.modifiers.add(base, ModifierEntry::simple(ModifierType::ChangeDp, -9000, Expiry::EndOfTurn, 0));
    assert_eq!(r.game.effective_security_strike(base), 1);
}

#[test]
fn st25_05_option_side_boosts_alysion_deletes_by_any_ally_dp_then_arts_digivolve() {
    let mut r = start(fixture(&["ST25-05"]).hand(0, &["ST25-05"]).memory(10));
    let alysion = r.place_stack(0, &["RED-ALYSION"]); // 5000 -> 15000
    let _lv5 = r.place_stack(0, &["RED-LV5"]); // 7000
    r.place_stack(1, &["PLAIN-12K"]); // <= 15000
    r.place_stack(1, &["PLAIN-6K"]);  // <= 7000

    action(&mut r, 0, PLAY_HAND_START);
    // Step 1: +10000 DP to 1 of your [ALYSION] Digimon.
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("RED-ALYSION", None).unwrap();
    assert_eq!(r.effective_dp(alysion), Some(15000));

    // Step 2 (Q7558): choose ANY of your Digimon as comparator (even the other one).
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("RED-ALYSION", None).unwrap();

    // Step 3: delete 1 opponent Digimon with DP <= 15000.
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("PLAIN-12K", None).unwrap();
    assert_eq!(r.battle_area_size(1), 1);

    // Step 4 (Q7557): <Arts Digivolve> occurs AFTER full [Main] resolution.
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    action(&mut r, 0, PASS); // decline Arts Digivolve
    assert!(r.pending_selection().is_none());
}
