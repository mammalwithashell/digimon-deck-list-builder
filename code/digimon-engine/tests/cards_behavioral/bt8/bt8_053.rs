//! BT8-053 Lighdramon — behavioral tests.
//!
//! Printed:
//!   <Armor Purge>
//!   [When Digivolving] Suspend 1 of your opponent's level 4 or lower Digimon.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn bt8_053_has_armor_purge_and_when_digivolving_suspends_opp_lv4_or_lower() {
    let mut opp_lv4 = make_test_card("BT8-053-OPP4", "OppLv4");
    opp_lv4.card_kind = CardKind::Digimon;
    opp_lv4.level = Some(4);
    opp_lv4.colors = vec![CardColor::Green];
    opp_lv4.dp = Some(4000);

    let mut opp_lv5 = make_test_card("BT8-053-OPP5", "OppLv5");
    opp_lv5.card_kind = CardKind::Digimon;
    opp_lv5.level = Some(5);
    opp_lv5.colors = vec![CardColor::Green];
    opp_lv5.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT8-053")
        .unwrap()
        .add_card(opp_lv4)
        .add_card(opp_lv5)
        .start();
    r.skip_mulligan();
    let self_perm = r.place_on_field(0, "BT8-053", Some(0));
    let target_lv4 = r.place_on_field(1, "BT8-053-OPP4", Some(0));
    let target_lv5 = r.place_on_field(1, "BT8-053-OPP5", Some(0));

    assert!(
        r.game.has_keyword(self_perm, Keyword::ArmorPurge),
        "BT8-053 must have <Armor Purge>"
    );

    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(self_perm));
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("BT8-053 [WD] must prompt to suspend 1 opp Lv.4 or lower Digimon");
    assert_eq!(
        sel.valid_action_ids.len(),
        1,
        "only Lv.4 opponent Digimon should be selectable (not Lv.5)"
    );
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert!(r.game.players[1].battle_area[target_lv4.index as usize].is_suspended);
    assert!(!r.game.players[1].battle_area[target_lv5.index as usize].is_suspended);
}
