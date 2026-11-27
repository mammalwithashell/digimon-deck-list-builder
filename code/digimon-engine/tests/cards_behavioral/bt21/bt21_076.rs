//! BT21-076 WarGrowlmon — behavioral tests.
//!
//! Printed:
//!   [On Play] [When Digivolving] Trash the top 2 cards of your deck. Then,
//!     this Digimon gains <Raid> and <Retaliation> until your opponent's turn
//!     ends.
//!   [When Attacking] [Once Per Turn] This Digimon may digivolve into
//!     [Megidramon] or [ChaosGallantmon] in the hand with the digivolution cost
//!     reduced by 1 for every 10 total cards in both players' trashes.
//!   Inherited: [On Deletion] Trash your opponent's top security card.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt21_076_on_play_mills_2_grants_raid_retaliation_and_inherited_trashes_sec() {
    let mut filler = make_test_card("BT21-076-FILL", "Filler");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.colors = vec![CardColor::Purple];
    filler.dp = Some(2000);

    let mut host = make_test_card("BT21-076-HOST", "Megidramon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(6);
    host.colors = vec![CardColor::Purple];
    host.dp = Some(13000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT21-076")
        .unwrap()
        .add_card(filler)
        .add_card(host)
        .deck(0, &["BT21-076-FILL", "BT21-076-FILL", "BT21-076-FILL"])
        .security(1, &["BT21-076-FILL", "BT21-076-FILL"])
        .start();
    r.skip_mulligan();

    let wg = r.place_on_field(0, "BT21-076", Some(0));
    r.fire_on_play(0, wg.index as usize);
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert!(r.game.has_keyword(wg, Keyword::Raid));
    assert!(r.game.has_keyword(wg, Keyword::Retaliation));

    // Inherited [On Deletion]: trash opp top security.
    let stacked = r.place_stack(0, &["BT21-076", "BT21-076-HOST"]);
    let _ = r.game.delete_permanent_with_cause(stacked, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.game.players[1].security.len(),
        1,
        "BT21-076 inherited [On Deletion] must trash opponent's top security"
    );
}
