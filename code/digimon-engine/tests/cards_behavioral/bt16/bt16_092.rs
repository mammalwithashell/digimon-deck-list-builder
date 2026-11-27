//! BT16-092 Invincible Dragon-Insect Fusion — behavioral tests.
//!
//! Printed:
//!   [Main] You may play 1 [ExVeemon] or [Stingmon] from your hand without
//!     paying the cost. Then, 2 of your Digimon may DNA digivolve into a
//!     Digimon card in the hand. The Digimon this effect DNA digivolved can't
//!     be deleted in battle and gains <Blocker> until the end of your
//!     opponent's turn.
//!   [Security] You may play 1 [Veemon] or [Wormmon] from your hand or trash
//!     without paying the cost. Then, add this card to the hand.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

#[test]
fn bt16_092_main_plays_exveemon_free_from_hand() {
    let mut exvee = make_test_card("BT16-092-EXV", "ExVeemon");
    exvee.card_kind = CardKind::Digimon;
    exvee.level = Some(4);
    exvee.colors = vec![CardColor::Blue];
    exvee.play_cost = 5;
    exvee.dp = Some(5000);

    let mut green = make_test_card("BT16-092-GRN", "Stingmon");
    green.card_kind = CardKind::Digimon;
    green.level = Some(4);
    green.colors = vec![CardColor::Green];
    green.dp = Some(5000);

    let mut blue = make_test_card("BT16-092-BLU", "Veemon");
    blue.card_kind = CardKind::Digimon;
    blue.level = Some(3);
    blue.colors = vec![CardColor::Blue];
    blue.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-092")
        .unwrap()
        .add_card(exvee)
        .add_card(green)
        .add_card(blue)
        .hand(0, &["BT16-092", "BT16-092-EXV"])
        .start();
    r.skip_mulligan();
    r.game.memory = 5;
    r.place_on_field(0, "BT16-092-BLU", Some(0));
    r.place_on_field(0, "BT16-092-GRN", Some(0));
    r.game.current_phase = GamePhase::Main;

    let opt_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT16-092")
        .unwrap();
    r.set_first_player(0);
    r.game.decode_action(opt_idx as u16, 0);

    let sel = r
        .pending_selection_view()
        .expect("BT16-092 [Main] must offer [ExVeemon] from hand");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        3,
        "BT16-092 [Main] must play [ExVeemon] free"
    );
}
