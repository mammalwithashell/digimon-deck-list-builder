//! A 0 DP deletion by the rule check (§17-1-3-1-1) is a deletion by the game
//! rules, not by a card effect. Official Q&A Q1348 (BT5-071) and Q2828
//! (BT17-068): a Digimon deleted because its DP became 0 was "deleted by the
//! game rules", so "deleted by an effect" conditions are not met.

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::replacement::ReplacementCause;
use std::sync::Arc;

/// "[On Deletion] Gain 1 memory if this Digimon was NOT deleted by an
/// effect; gain 5 memory if it was." The gain tells the test which branch ran.
struct ReportsDeletionCause;
impl CardEffect for ReportsDeletionCause {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(c)
            .process(|ctx| {
                let by_effect = ctx.was_deleted_by_effect()
                    || matches!(
                        ctx.deletion_cause(),
                        Some(ReplacementCause::OwnEffect | ReplacementCause::OpponentEffect)
                    );
                ctx.gain_memory(if by_effect { 5 } else { 1 })
            })
            .build()]
    }
}

#[test]
fn zero_dp_rule_check_deletion_is_not_a_deletion_by_an_effect() {
    let mut zero = make_test_card("RC-ZERO", "Zero DP");
    zero.dp = Some(0);
    let mut r = DebugRunner::builder().add_card(zero).memory(3).start();
    r.register_effect("RC-ZERO", Arc::new(ReportsDeletionCause));
    assert_eq!(r.game.turn_player(), 0);
    r.place_on_field(0, "RC-ZERO", Some(0));

    r.game.drain_effect_queue();

    assert!(r.game.players[0].battle_area.is_empty(), "deleted at 0 DP");
    assert_eq!(
        r.game.memory, 4,
        "Q1348 / Q2828: not deleted by an effect (was_deleted_by_effect false, \
         deletion_cause neither own nor opponent effect)"
    );
}
