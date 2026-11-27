//! BT11-032 UlforceVeedramon — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] You may play 1 blue Tamer card from your hand without
//!     paying the cost.
//!   [Your Turn] When any of your blue Tamers are played, this Digimon unsuspends.
//!   [Your Turn][Once Per Turn] When this Digimon unsuspends, return 1 of your
//!     opponent's level 3 or lower Digimon to the hand. For each of your blue
//!     Tamers, add 1 to this effect's level maximum.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn bt11_032_plays_blue_tamer_unsuspends_and_bounces_scaled_level_digimon() {
    let mut blue_tamer = make_test_card("BT11-032-TAMER", "Rina Shinomiya");
    blue_tamer.card_kind = CardKind::Tamer;
    blue_tamer.colors = vec![CardColor::Blue];
    blue_tamer.play_cost = 4;

    let mut opp_lv4 = make_test_card("BT11-032-OPP4", "OppLv4");
    opp_lv4.card_kind = CardKind::Digimon;
    opp_lv4.level = Some(4);
    opp_lv4.colors = vec![CardColor::Red];
    opp_lv4.dp = Some(5000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT11-032")
        .unwrap()
        .add_card(blue_tamer)
        .add_card(opp_lv4)
        .hand(0, &["BT11-032-TAMER"])
        .start();
    r.skip_mulligan();
    let ulforce = r.place_on_field(0, "BT11-032", Some(0));
    r.game.players[0].battle_area[ulforce.index as usize].is_suspended = true;
    r.place_on_field(1, "BT11-032-OPP4", Some(0));

    // [WD] plays blue Tamer from hand free -> triggers unsuspend -> triggers bounce Lv<=4 (3+1).
    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(ulforce));
    r.game.drain_effect_queue();
    let sel_tamer = r
        .pending_selection_view()
        .expect("BT11-032 [WD] must offer blue Tamer in hand");
    let pick_tamer = *sel_tamer
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_tamer).unwrap();

    assert!(
        !r.game.players[0].battle_area[ulforce.index as usize].is_suspended,
        "Playing a blue Tamer must unsuspend BT11-032"
    );

    let sel_bounce = r
        .pending_selection_view()
        .expect("Unsuspending BT11-032 must prompt to bounce Lv.4 opp Digimon (max 3+1)");
    let pick_bounce = *sel_bounce
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_bounce).unwrap();

    assert!(
        r.game.players[1].battle_area.is_empty(),
        "Opp Lv.4 Digimon must be returned to hand"
    );
    assert_eq!(r.game.players[1].hand.len(), 1);
}
