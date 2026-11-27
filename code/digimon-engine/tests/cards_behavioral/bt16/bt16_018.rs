//! BT16-018 ExVeemon — behavioral tests.
//!
//! Printed:
//!   <Raid>
//!   [On Play] [When Digivolving] 1 of your Digimon can't be deleted in battle
//!     until the end of your opponent's turn.
//!   Inherited: [Your Turn] This Digimon gets +2000 DP.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};

#[test]
fn bt16_018_has_raid_grants_battle_immunity_and_inherited_2000_dp() {
    let mut host = make_test_card("BT16-018-HOST", "Paildramon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(5);
    host.colors = vec![CardColor::Blue, CardColor::Green];
    host.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-018")
        .unwrap()
        .add_card(host)
        .start();
    r.skip_mulligan();
    let exvee = r.place_on_field(0, "BT16-018", Some(0));

    assert!(r.game.has_keyword(exvee, Keyword::Raid));

    r.fire_on_play(0, exvee.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("BT16-018 [On Play] must prompt for 1 of your Digimon");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    assert!(
        r.game
            .modifiers
            .has(exvee, ModifierType::CannotBeDestroyedByBattle),
        "BT16-018 [On Play] must grant CannotBeDestroyedByBattle"
    );

    // Inherited: [Your Turn] +2000 DP.
    let stacked = r.place_stack(0, &["BT16-018", "BT16-018-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.effective_dp(stacked),
        Some(10000),
        "BT16-018 inherited must grant +2000 DP on Your Turn"
    );
}
