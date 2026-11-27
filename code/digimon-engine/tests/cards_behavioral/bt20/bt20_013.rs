//! BT20-013 BaoHuckmon — behavioral tests.
//!
//! Printed:
//!   [Main] [Once Per Turn] You may play 1 Digimon card with [Sistermon] or
//!     [Gankoomon] in its name from your hand with the play cost reduced by 2.
//!   Inherited: [Your Turn] All of your Digimon get +1000 DP.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

#[test]
fn bt20_013_main_opt_plays_sistermon_reduced_by_2_and_inherited_buffs_dp() {
    let mut sistermon = make_test_card("BT20-013-SIS", "Sistermon Blanc");
    sistermon.card_kind = CardKind::Digimon;
    sistermon.level = Some(3);
    sistermon.colors = vec![CardColor::White];
    sistermon.play_cost = 3;
    sistermon.dp = Some(3000);

    let mut host = make_test_card("BT20-013-HOST", "SaviorHuckmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(5);
    host.colors = vec![CardColor::Red];
    host.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT20-013")
        .unwrap()
        .add_card(sistermon)
        .add_card(host)
        .hand(0, &["BT20-013-SIS"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.memory = 3;
    let bao = r.place_on_field(0, "BT20-013", Some(0));
    r.game.current_phase = GamePhase::Main;

    // Activate [Main] [Once Per Turn] on BaoHuckmon (slot 0 -> FIELD_EFFECT_START + 2 = 1002).
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    let main_act = (1000u16..1014)
        .find(|&a| mask[a as usize] > 0.5)
        .expect("BT20-013 [Main][OPT] must be reachable in Main phase");
    r.game.decode_action(main_act, 0);

    let sel = r
        .pending_selection_view()
        .expect("BT20-013 [Main] must prompt for [Sistermon]/[Gankoomon] in hand");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "Sistermon must be played from hand"
    );
    assert_eq!(
        r.game.memory, 2,
        "Play cost 3 reduced by 2 = 1 memory paid (3 -> 2)"
    );
    let _ = bao;

    // Inherited: [Your Turn] all of your Digimon get +1000 DP.
    let stacked = r.place_stack(0, &["BT20-013", "BT20-013-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.effective_dp(stacked),
        Some(8000),
        "BT20-013 inherited must grant +1000 DP to your Digimon"
    );
}
