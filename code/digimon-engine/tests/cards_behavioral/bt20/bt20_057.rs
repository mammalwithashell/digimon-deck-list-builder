//! BT20-057 Gankoomon — behavioral tests.
//!
//! Printed:
//!   When this card would be played, if you have a Digimon with [Huckmon],
//!     [Jesmon] or [Sistermon] in its name, reduce the play cost by 4.
//!   <Reboot>
//!   <Blocker>
//!   [On Play] [When Digivolving] 1 of your Digimon may digivolve into a
//!     level 6 or lower Digimon card with [Huckmon] in its name or the
//!     [Royal Knight] trait in the hand or trash without paying the cost.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

#[test]
fn bt20_057_reduces_play_cost_by_4_with_huckmon_and_has_reboot_blocker() {
    let mut huckmon = make_test_card("BT20-057-HUCK", "Huckmon");
    huckmon.card_kind = CardKind::Digimon;
    huckmon.level = Some(3);
    huckmon.colors = vec![CardColor::Red];
    huckmon.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT20-057")
        .unwrap()
        .add_card(huckmon)
        .hand(0, &["BT20-057"])
        .start();
    r.skip_mulligan();
    r.game.memory = 10;
    r.place_on_field(0, "BT20-057-HUCK", Some(0));
    r.game.current_phase = GamePhase::Main;

    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT20-057")
        .unwrap();
    r.set_first_player(0);
    r.game.decode_action(hand_idx as u16, 0);

    // Base cost 12 - 4 = 8 -> memory 10 - 8 = 2.
    assert_eq!(
        r.game.memory, 2,
        "BT20-057 must reduce play cost by 4 (12 -> 8) when Huckmon is in play"
    );
    let gankoo = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 1,
    };
    assert!(r.game.has_keyword(gankoo, Keyword::Reboot));
    assert!(r.game.has_keyword(gankoo, Keyword::Blocker));
}
