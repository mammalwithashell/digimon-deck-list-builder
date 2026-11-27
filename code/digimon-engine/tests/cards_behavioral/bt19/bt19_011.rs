//! BT19-011 WarGrowlmon (ACE) — behavioral tests.
//!
//! Printed:
//!   {Hand} [Counter] <Blast Digivolve>
//!   [On Play] [When Digivolving] Delete up to 3000 DP total worth of your
//!     opponent's Digimon. For each of your opponent's Digimon, add 2000 to
//!     this DP deletion effect's maximum. Then, for each Digimon this effect
//!     deleted, gain 1 memory.
//!   Inherited: [All Turns] Add 3000 to this Digimon's DP deletion effects' maximums.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn bt19_011_on_play_deletes_within_scaled_dp_sum_and_gains_memory() {
    let mut opp1 = make_test_card("BT19-011-O1", "Opp1");
    opp1.card_kind = CardKind::Digimon;
    opp1.level = Some(3);
    opp1.colors = vec![CardColor::Blue];
    opp1.dp = Some(4000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT19-011")
        .unwrap()
        .add_card(opp1)
        .start();
    r.skip_mulligan();
    r.game.memory = 0;

    let wg = r.place_on_field(0, "BT19-011", Some(0));
    r.place_on_field(1, "BT19-011-O1", Some(0));

    assert!(r.game.has_keyword(wg, Keyword::BlastDigivolve));

    // With 1 opp Digimon: cap = 3000 + 2000*1 = 5000 >= 4000.
    r.fire_on_play(0, wg.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("BT19-011 [On Play] must prompt to delete opp Digimon within DP budget");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert!(
        r.game.players[1].battle_area.is_empty(),
        "4000 DP opp Digimon must be deleted (budget 5000)"
    );
    assert_eq!(
        r.game.memory, 1,
        "Deleting 1 Digimon must gain 1 memory"
    );
}
