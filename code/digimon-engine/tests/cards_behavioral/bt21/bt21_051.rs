//! BT21-051 Puppetmon (ACE) — behavioral tests.
//!
//! Printed:
//!   {Hand} [Counter] <Blast Digivolve>
//!   <Reboot> <Blocker>
//!   [On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's
//!     Digimon. Then, return 1 of their suspended Digimon to the bottom of the
//!     deck.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn bt21_051_keywords_and_on_play_dedigivolves_2_then_bottoms_suspended_opp() {
    let mut lv3 = make_test_card("BT21-051-L3", "Lv3");
    lv3.card_kind = CardKind::Digimon;
    lv3.level = Some(3);
    lv3.colors = vec![CardColor::Green];
    lv3.dp = Some(2000);

    let mut lv4 = make_test_card("BT21-051-L4", "Lv4");
    lv4.card_kind = CardKind::Digimon;
    lv4.level = Some(4);
    lv4.colors = vec![CardColor::Green];
    lv4.dp = Some(5000);

    let mut lv5 = make_test_card("BT21-051-L5", "Lv5");
    lv5.card_kind = CardKind::Digimon;
    lv5.level = Some(5);
    lv5.colors = vec![CardColor::Green];
    lv5.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT21-051")
        .unwrap()
        .add_card(lv3)
        .add_card(lv4)
        .add_card(lv5)
        .start();
    r.skip_mulligan();
    let pup = r.place_on_field(0, "BT21-051", Some(0));
    let opp = r.place_stack(1, &["BT21-051-L3", "BT21-051-L4", "BT21-051-L5"]);
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;

    assert!(r.game.has_keyword(pup, Keyword::BlastDigivolve));
    assert!(r.game.has_keyword(pup, Keyword::Reboot));
    assert!(r.game.has_keyword(pup, Keyword::Blocker));

    r.fire_on_play(0, pup.index as usize);
    // Step 1: De-Digivolve 2 (choose target, then confirm max count if prompted).
    let sel1 = r
        .pending_selection_view()
        .expect("BT21-051 must prompt for <De-Digivolve 2>");
    let p1 = *sel1.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, p1).unwrap();
    if let Some(sel_cnt) = r.pending_selection_view() {
        if r.game.current_phase == digimon_engine::enums::GamePhase::EffectChoice {
            let max_pick = *sel_cnt.valid_action_ids.last().unwrap();
            r.execute_action(0, max_pick).unwrap();
        }
    }
    assert_eq!(r.game.players[1].trash.len(), 2);

    // Step 2: Return 1 suspended opp Digimon to bottom of deck.
    let sel2 = r
        .pending_selection_view()
        .expect("BT21-051 must prompt to bottom-deck suspended opp Digimon");
    let p2 = *sel2.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, p2).unwrap();

    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(r.game.players[1].deck.len(), 1);
}
