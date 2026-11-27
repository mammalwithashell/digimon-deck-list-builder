//! EX4-011 ChaosGallantmon — behavioral tests.
//!
//! Printed:
//!   [Trash][End of Your Turn] By deleting 1 of your Digimon with digivolution
//!     cards and [Gallantmon] in its name, you may play this card without paying
//!     the cost.
//!   [On Play] Delete 1 of your opponent's Digimon with 7000 DP or less. For
//!     every 10 total cards in both players' trashes, add 2000 to the maximum
//!     this DP-based deletion effect can delete.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn ex4_011_on_play_deletes_scaled_dp_opponent_digimon() {
    let mut opp_9k = make_test_card("EX4-011-O9K", "Opp9000");
    opp_9k.card_kind = CardKind::Digimon;
    opp_9k.level = Some(5);
    opp_9k.colors = vec![CardColor::Blue];
    opp_9k.dp = Some(9000);

    let mut filler = make_test_card("EX4-011-FILL", "Filler");
    filler.level = Some(3);

    let trash_10 = vec!["EX4-011-FILL"; 10];
    let mut r = DebugRunner::builder()
        .dsl_card("EX4-011")
        .unwrap()
        .add_card(opp_9k)
        .add_card(filler)
        .start();
    r.skip_mulligan();
    for &cid in &trash_10 {
        r.inject_trash(0, cid);
    }

    let cg = r.place_on_field(0, "EX4-011", Some(0));
    r.place_on_field(1, "EX4-011-O9K", Some(0));

    // With 10 cards in trash: 7000 + 2000 = 9000 DP max -> can delete Opp9000.
    r.fire_on_play(0, cg.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("EX4-011 [On Play] must target 9000 DP opp Digimon with 10 cards in trash");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert!(
        r.game.players[1].battle_area.is_empty(),
        "Opp9000 must be deleted by EX4-011 [On Play]"
    );
}
