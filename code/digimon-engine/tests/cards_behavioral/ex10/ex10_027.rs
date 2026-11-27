//! EX10-027 DeadlyAxemon — behavioral tests.
//!
//! Printed:
//!   [On Play] [When Digivolving] By trashing 1 card in your hand, you may
//!     return 1 Digimon card with [Knightmon] in its text or the [Bagra Army]
//!     or [Twilight] trait from your trash to the hand.
//!   [On Deletion] <Save>
//!   Inherited: <Retaliation>

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn ex10_027_on_play_trashes_1_to_recover_twilight_and_has_save_inherited_retaliation() {
    let mut twilight = make_test_card("EX10-027-TW", "SkullKnightmon");
    twilight.card_kind = CardKind::Digimon;
    twilight.level = Some(4);
    twilight.colors = vec![CardColor::Black];
    twilight.dp = Some(5000);
    twilight.traits = vec!["Twilight".to_string()];

    let mut filler = make_test_card("EX10-027-FILL", "Filler");
    filler.level = Some(3);

    let mut host = make_test_card("EX10-027-HOST", "DarkKnightmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(5);
    host.colors = vec![CardColor::Black];
    host.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX10-027")
        .unwrap()
        .add_card(twilight)
        .add_card(filler)
        .add_card(host)
        .hand(0, &["EX10-027-FILL"])
        .start();
    r.skip_mulligan();
    for &cid in &["EX10-027-TW"] {
        r.inject_trash(0, cid);
    }

    let da = r.place_on_field(0, "EX10-027", Some(0));
    assert!(r.game.has_keyword(da, Keyword::Save));

    r.fire_on_play(0, da.index as usize);
    let sel_hand = r
        .pending_selection_view()
        .expect("EX10-027 [On Play] must prompt to trash 1 card from hand");
    let p_hand = *sel_hand
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, p_hand).unwrap();

    let sel_rec = r
        .pending_selection_view()
        .expect("EX10-027 [On Play] must offer [Twilight] Digimon in trash");
    let p_rec = *sel_rec
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, p_rec).unwrap();

    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "EX10-027-TW"));

    let stacked = r.place_stack(0, &["EX10-027", "EX10-027-HOST"]);
    assert!(r.game.has_keyword(stacked, Keyword::Retaliation));
}
