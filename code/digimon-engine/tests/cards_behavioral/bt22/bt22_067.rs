//! BT22-067 LordKnightmon — behavioral tests.
//!
//! Printed:
//!   <Raid> <Reboot>
//!   [On Play] [When Digivolving] 1 of your Digimon gets +3000 DP until your
//!     opponent's turn ends. Then, 1 of your Digimon may attack a player.
//!   [All Turns] [Once Per Turn] When Digimon attack players, reveal the top 3
//!     cards of your deck. You may play 1 play cost 4 or lower black or red
//!     card among them without paying the cost. Trash the rest.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

#[test]
fn bt22_067_has_raid_reboot_and_on_play_buffs_dp_by_3000() {
    let mut r = DebugRunner::builder().dsl_card("BT22-067").unwrap().start();
    r.skip_mulligan();
    let lk = r.place_on_field(0, "BT22-067", Some(0));

    assert!(r.game.has_keyword(lk, Keyword::Raid));
    assert!(r.game.has_keyword(lk, Keyword::Reboot));

    r.fire_on_play(0, lk.index as usize);
    let sel_dp = r
        .pending_selection_view()
        .expect("BT22-067 [On Play] must prompt for +3000 DP target");
    let pick_dp = *sel_dp
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_dp).unwrap();

    assert_eq!(
        r.game.effective_dp(lk),
        Some(15000),
        "BT22-067 [On Play] must grant +3000 DP (12000 -> 15000)"
    );

    // Decline optional attack cleanly.
    if r.pending_selection_view().is_some() {
        r.execute_action(0, PASS).unwrap();
    }
}
