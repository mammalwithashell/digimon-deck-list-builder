//! EX8-012 Growlmon (X Antibody) — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] <Draw 1> and trash 1 card in your hand. Then, if
//!     [Growlmon] or [X Antibody] is in this Digimon's digivolution cards,
//!     until the end of your opponent's turn, this Digimon gains "[On Deletion]
//!     You may play 1 card with [Guilmon] in its name from your trash without
//!     paying the cost."
//!   Inherited: [Your Turn] [Once Per Turn] When any of your opponent's
//!     Digimon are deleted, gain 1 memory.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn ex8_012_wd_draws_trashes_and_inherited_gains_memory_on_opp_deletion() {
    let mut filler = make_test_card("EX8-012-FILL", "Filler");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.dp = Some(2000);

    let mut host = make_test_card("EX8-012-HOST", "WarGrowlmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(5);
    host.colors = vec![CardColor::Red];
    host.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX8-012")
        .unwrap()
        .add_card(filler)
        .add_card(host)
        .deck(0, &["EX8-012-FILL", "EX8-012-FILL"])
        .start();
    r.skip_mulligan();

    let gx = r.place_on_field(0, "EX8-012", Some(0));
    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(gx));
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("EX8-012 [WD] must prompt to trash 1 card from hand after Draw 1");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    assert_eq!(r.game.players[0].trash.len(), 1);

    // Inherited: [Your Turn][OPT] when opponent's Digimon is deleted, gain 1 memory.
    r.game.memory = 0;
    r.place_stack(0, &["EX8-012", "EX8-012-HOST"]);
    let opp = r.place_on_field(1, "EX8-012-FILL", Some(0));
    let _ = r
        .game
        .delete_permanent_with_cause(opp, ReplacementCause::OwnEffect);
    assert_eq!(
        r.game.memory, 1,
        "EX8-012 inherited must gain 1 memory when opponent's Digimon is deleted"
    );
}
