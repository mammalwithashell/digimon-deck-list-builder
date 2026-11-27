//! BT21-079 Megidramon — behavioral tests.
//!
//! Printed:
//!   (Rule) Name: Also treated as [ChaosGallantmon].
//!   <Security A. +1>
//!   [End of Attack] [Once Per Turn] Delete all Digimon.
//!   [On Deletion] You may play 1 play cost 3 or lower Digimon card with
//!     [Guilmon] or [Growlmon] in its name from your trash without paying the
//!     cost. For every 10 total cards in both players' trashes, add 2 to this
//!     effect's play cost maximum.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt21_079_has_sec_plus_1_and_on_deletion_plays_guilmon_from_trash() {
    let mut guilmon = make_test_card("BT21-079-GUIL", "Guilmon");
    guilmon.card_kind = CardKind::Digimon;
    guilmon.level = Some(3);
    guilmon.colors = vec![CardColor::Red];
    guilmon.play_cost = 3;
    guilmon.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT21-079")
        .unwrap()
        .add_card(guilmon)
        .start();
    r.skip_mulligan();
    for &cid in &["BT21-079-GUIL"] {
        r.inject_trash(0, cid);
    }
    let megi = r.place_on_field(0, "BT21-079", Some(0));

    assert!(r.game.has_keyword(megi, Keyword::SecurityAttackPlus(1)));

    let _ = r
        .game
        .delete_permanent_with_cause(megi, ReplacementCause::OpponentEffect);
    let sel = r
        .pending_selection_view()
        .expect("BT21-079 [On Deletion] must offer cost <=3 [Guilmon] in trash");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Guilmon must be played from trash without paying the cost"
    );
}
