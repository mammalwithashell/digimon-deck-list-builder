//! ST12-10 Jesmon — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] <Blitz>
//!   [When Attacking] You may play 1 Digimon card with [Sistermon] in its name
//!     from your hand without paying its memory cost.
//!   [Your Turn][Once Per Turn] When you play another Digimon by an effect,
//!     this Digimon gets +3000 DP and gains <Security Attack +1> for the turn.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn st12_10_when_attacking_plays_sistermon_and_triggers_dp_and_sec_plus_1() {
    let mut sistermon = make_test_card("ST12-10-SIS", "Sistermon Blanc");
    sistermon.card_kind = CardKind::Digimon;
    sistermon.level = Some(3);
    sistermon.colors = vec![CardColor::White];
    sistermon.play_cost = 3;
    sistermon.dp = Some(3000);

    let mut r = DebugRunner::builder()
        .dsl_card("ST12-10")
        .unwrap()
        .add_card(sistermon)
        .hand(0, &["ST12-10-SIS"])
        .start();
    r.skip_mulligan();

    let jes = r.place_on_field(0, "ST12-10", Some(0));
    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenAttacking, digimon_engine::selection::TriggerSource::Permanent(jes));
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("ST12-10 [When Attacking] must offer [Sistermon] in hand");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "Sistermon must be played for free"
    );
    assert_eq!(
        r.game.effective_dp(jes),
        Some(15000),
        "Playing another Digimon by effect must give ST12-10 +3000 DP (12000 -> 15000)"
    );
    assert_eq!(
        (r.game.effective_security_strike(jes) - 1),
        1,
        "Playing another Digimon by effect must give ST12-10 <Security Attack +1>"
    );
}
