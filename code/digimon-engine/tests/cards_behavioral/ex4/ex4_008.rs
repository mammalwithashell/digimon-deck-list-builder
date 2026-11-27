//! EX4-008 BlackGrowlmon — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] Trash the top 2 cards of both players' decks. Then,
//!     you may return 1 [Guilmon] or 1 Digimon card with [Growlmon] or
//!     [Gallantmon] in its name from your trash to your hand.
//!   Inherited: [On Deletion] You may return 1 [Guilmon] or 1 Digimon card
//!     with [Growlmon] or [Gallantmon] in its name from your trash to your hand.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn ex4_008_when_digivolving_mills_both_decks_and_recovers_guilmon() {
    let mut guilmon = make_test_card("EX4-008-GUIL", "Guilmon");
    guilmon.card_kind = CardKind::Digimon;
    guilmon.level = Some(3);
    guilmon.colors = vec![CardColor::Red];
    guilmon.dp = Some(2000);

    let mut filler = make_test_card("EX4-008-FILL", "Filler");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX4-008")
        .unwrap()
        .add_card(guilmon)
        .add_card(filler)
        .deck(0, &["EX4-008-GUIL", "EX4-008-GUIL", "EX4-008-GUIL"])
        .deck(1, &["EX4-008-FILL", "EX4-008-FILL", "EX4-008-FILL"])
        .start();
    r.skip_mulligan();
    let bg = r.place_on_field(0, "EX4-008", Some(0));

    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(bg));
    r.game.drain_effect_queue();
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert_eq!(r.game.players[1].trash.len(), 2);

    let sel = r
        .pending_selection_view()
        .expect("EX4-008 [WD] must offer [Guilmon] from trash");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX4-008-GUIL"),
        "Guilmon must be recovered from trash to hand"
    );
}
