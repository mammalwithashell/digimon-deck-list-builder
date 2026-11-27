//! BT21-068 Growlmon — behavioral tests.
//!
//! Printed:
//!   [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with
//!     4000 DP or less. If this effect didn't delete, trash the top 2 cards of
//!     your deck.
//!   Inherited: [On Deletion] Gain 1 memory.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt21_068_mills_2_when_no_delete_target_and_inherited_gains_1_memory() {
    let mut filler = make_test_card("BT21-068-FILL", "Filler");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.colors = vec![CardColor::Purple];
    filler.dp = Some(2000);

    let mut host = make_test_card("BT21-068-HOST", "WarGrowlmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(5);
    host.colors = vec![CardColor::Purple];
    host.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT21-068")
        .unwrap()
        .add_card(filler)
        .add_card(host)
        .deck(0, &["BT21-068-FILL", "BT21-068-FILL", "BT21-068-FILL"])
        .start();
    r.skip_mulligan();

    // No opponent Digimon -> didn't delete -> mills 2.
    let g = r.place_on_field(0, "BT21-068", Some(0));
    r.fire_on_play(0, g.index as usize);
    assert_eq!(
        r.game.players[0].trash.len(),
        2,
        "BT21-068 must trash top 2 of deck when no opponent Digimon was deleted"
    );

    // Inherited [On Deletion]: gain 1 memory.
    r.game.memory = 0;
    let stacked = r.place_stack(0, &["BT21-068", "BT21-068-HOST"]);
    let _ = r.game.delete_permanent_with_cause(stacked, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.game.memory, 1,
        "BT21-068 inherited [On Deletion] must gain 1 memory"
    );
}
