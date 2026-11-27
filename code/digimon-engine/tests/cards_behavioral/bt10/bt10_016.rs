//! BT10-016 Jesmon (X Antibody) — behavioral tests.
//!
//! Printed:
//!   <Piercing>
//!   [When Digivolving] You may play 1 Digimon card with [Sistermon] in its
//!     name from your hand or trash without paying its memory cost. Then, if
//!     [Jesmon] is in this Digimon's digivolution cards or you have a Digimon
//!     with [Sistermon] in its name in play, until the end of your opponent's
//!     turn, all of your Digimon can also attack your opponent's unsuspended
//!     Digimon and get +2000 DP.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};

#[test]
fn bt10_016_has_piercing_plays_sistermon_and_buffs_all_own_digimon() {
    let mut sistermon = make_test_card("BT10-016-SIS", "Sistermon Blanc");
    sistermon.card_kind = CardKind::Digimon;
    sistermon.level = Some(3);
    sistermon.colors = vec![CardColor::White];
    sistermon.play_cost = 3;
    sistermon.dp = Some(3000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT10-016")
        .unwrap()
        .add_card(sistermon)
        .hand(0, &["BT10-016-SIS"])
        .start();
    r.skip_mulligan();
    let jesmon = r.place_on_field(0, "BT10-016", Some(0));

    assert!(
        r.game.has_keyword(jesmon, Keyword::Piercing),
        "BT10-016 must have <Piercing>"
    );

    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(jesmon));
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("BT10-016 [WD] must offer Sistermon from hand/trash");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "Sistermon must be played for free"
    );
    assert_eq!(
        r.game.effective_dp(jesmon),
        Some(13000),
        "BT10-016 must get +2000 DP when Sistermon is in play"
    );
    assert!(
        r.game
            .modifiers
            .has(jesmon, ModifierType::CanAttackUnsuspended),
        "BT10-016 must grant CanAttackUnsuspended to your Digimon"
    );
}
