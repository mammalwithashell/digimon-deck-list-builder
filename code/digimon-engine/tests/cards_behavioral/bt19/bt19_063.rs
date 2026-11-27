//! BT19-063 DarkKnightmon — behavioral tests.
//!
//! Printed:
//!   <Material Save 1>
//!   [On Play] [When Digivolving] <De-Digivolve 1> 1 of your opponent's
//!     Digimon. Then, if DigiXrosing with 2 cards, you may delete 1 play cost
//!     3 or lower Digimon or Tamer.
//!   [On Deletion] You may play 1 level 4 or lower Digimon card with
//!     [Knightmon] in its text from under your Tamers without paying the cost.
//!   Inherited: [On Deletion] You may play 1 level 4 or lower Digimon card
//!     with [Knightmon] in its text from your trash without paying the cost.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn bt19_063_has_material_save_and_wd_dedigivolves_opponent() {
    let mut opp_lv3 = make_test_card("BT19-063-O3", "OppLv3");
    opp_lv3.card_kind = CardKind::Digimon;
    opp_lv3.level = Some(3);
    opp_lv3.colors = vec![CardColor::Red];
    opp_lv3.dp = Some(2000);

    let mut opp_lv4 = make_test_card("BT19-063-O4", "OppLv4");
    opp_lv4.card_kind = CardKind::Digimon;
    opp_lv4.level = Some(4);
    opp_lv4.colors = vec![CardColor::Red];
    opp_lv4.dp = Some(5000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT19-063")
        .unwrap()
        .add_card(opp_lv3)
        .add_card(opp_lv4)
        .start();
    r.skip_mulligan();
    let dk = r.place_on_field(0, "BT19-063", Some(0));
    let opp = r.place_stack(1, &["BT19-063-O3", "BT19-063-O4"]);

    assert!(r.game.has_keyword(dk, Keyword::MaterialSave(1)));

    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(dk));
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("BT19-063 [WD] must prompt to De-Digivolve 1 opp Digimon");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "BT19-063 [WD] must De-Digivolve 1 opponent Digimon"
    );
}
