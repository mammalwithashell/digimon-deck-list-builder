//! BT20-101 Zephagamon (ACE) — behavioral tests.
//!
//! Printed:
//!   {Hand} [Counter] <Blast Digivolve>
//!   <Piercing> <Vortex> <Blocker>
//!   [All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend.
//!   [On Play] [When Digivolving] You may suspend 1 Digimon. Then, for every 2
//!     suspended Digimon, you may return 1 of your opponent's suspended Digimon
//!     to the bottom of the deck.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn bt20_101_keywords_and_on_play_suspends_and_bottoms_opp_digimon() {
    let mut opp1 = make_test_card("BT20-101-O1", "Opp1");
    opp1.card_kind = CardKind::Digimon;
    opp1.level = Some(4);
    opp1.colors = vec![CardColor::Green];
    opp1.dp = Some(5000);

    let mut opp2 = make_test_card("BT20-101-O2", "Opp2");
    opp2.card_kind = CardKind::Digimon;
    opp2.level = Some(4);
    opp2.colors = vec![CardColor::Green];
    opp2.dp = Some(5000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT20-101")
        .unwrap()
        .add_card(opp1)
        .add_card(opp2)
        .start();
    r.skip_mulligan();
    let zeph = r.place_on_field(0, "BT20-101", Some(0));
    let o1 = r.place_on_field(1, "BT20-101-O1", Some(0));
    let o2 = r.place_on_field(1, "BT20-101-O2", Some(0));
    // Pre-suspend opp1 so suspending opp2 gives 2 suspended Digimon -> 1 bottom-deck bounce.
    r.game.players[1].battle_area[o1.index as usize].is_suspended = true;
    let _ = o2;

    assert!(r.game.has_keyword(zeph, Keyword::BlastDigivolve));
    assert!(r.game.has_keyword(zeph, Keyword::Piercing));
    assert!(r.game.has_keyword(zeph, Keyword::Vortex));
    assert!(r.game.has_keyword(zeph, Keyword::Blocker));

    r.fire_on_play(0, zeph.index as usize);
    // Step 1: suspend 1 Digimon (pick opp2 — last non-PASS valid action or any opp action).
    let sel_susp = r
        .pending_selection_view()
        .expect("BT20-101 [On Play] must prompt to suspend 1 Digimon");
    let pick_susp = *sel_susp
        .valid_action_ids
        .iter()
        .rev()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_susp).unwrap();

    // Step 2: 2 suspended Digimon -> prompt to bottom-deck 1 opp suspended Digimon.
    let sel_bot = r
        .pending_selection_view()
        .expect("With 2 suspended Digimon, BT20-101 must prompt to bottom-deck 1 opp suspended Digimon");
    let pick_bot = *sel_bot
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_bot).unwrap();

    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "1 opponent suspended Digimon must be returned to deck bottom"
    );
    assert_eq!(r.game.players[1].deck.len(), 1);
}
