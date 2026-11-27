//! EX10-026 SkullKnightmon — behavioral tests.
//!
//! Printed:
//!   [On Play] [When Digivolving] By trashing 1 card in your hand, delete 1 of
//!     your opponent's Digimon with a play cost of 4 or less.
//!   [On Deletion] <Save>
//!   Inherited: <Blocker>

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn ex10_026_on_play_trashes_1_hand_to_delete_opp_cost_4_and_has_save_inherited_blocker() {
    let mut opp = make_test_card("EX10-026-OPP", "OppCost4");
    opp.card_kind = CardKind::Digimon;
    opp.level = Some(4);
    opp.colors = vec![CardColor::Blue];
    opp.play_cost = 4;
    opp.dp = Some(5000);

    let mut filler = make_test_card("EX10-026-FILL", "Filler");
    filler.level = Some(3);

    let mut host = make_test_card("EX10-026-HOST", "DarkKnightmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(5);
    host.colors = vec![CardColor::Black];
    host.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX10-026")
        .unwrap()
        .add_card(opp)
        .add_card(filler)
        .add_card(host)
        .hand(0, &["EX10-026-FILL"])
        .start();
    r.skip_mulligan();

    let sk = r.place_on_field(0, "EX10-026", Some(0));
    r.place_on_field(1, "EX10-026-OPP", Some(0));
    assert!(r.game.has_keyword(sk, Keyword::Save));

    r.fire_on_play(0, sk.index as usize);
    let sel_hand = r
        .pending_selection_view()
        .expect("EX10-026 [On Play] must prompt to trash 1 card from hand");
    let p_hand = *sel_hand
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, p_hand).unwrap();

    let sel_del = r
        .pending_selection_view()
        .expect("EX10-026 [On Play] must prompt to delete opp cost <=4 Digimon");
    let p_del = *sel_del
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, p_del).unwrap();

    assert!(r.game.players[1].battle_area.is_empty());

    let stacked = r.place_stack(0, &["EX10-026", "EX10-026-HOST"]);
    assert!(r.game.has_keyword(stacked, Keyword::Blocker));
}
