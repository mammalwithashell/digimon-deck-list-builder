//! BT12-072 Chaosdramon (X Antibody) — behavioral tests.
//!
//! Printed:
//!   [Start of Your Main Phase] Place 1 Digimon card with the [Cyborg] or
//!     [Machine] trait from your trash as this Digimon's bottom digivolution card.
//!   [All Turns] This Digimon gains all the effects of [Machinedramon] and
//!     [Chaosdramon] in its digivolution cards.
//!   [All Turns][Once Per Turn] When this Digimon would be deleted, trash your
//!     opponent's top security card.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

#[test]
fn bt12_072_start_of_main_places_cyborg_from_trash_as_bottom_source() {
    let mut cyborg = make_test_card("BT12-072-CYB", "Guardromon");
    cyborg.card_kind = CardKind::Digimon;
    cyborg.level = Some(4);
    cyborg.colors = vec![CardColor::Black];
    cyborg.dp = Some(5000);
    cyborg.traits = vec!["Cyborg".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT12-072")
        .unwrap()
        .add_card(cyborg)
        .start();
    r.skip_mulligan();
    for &cid in &["BT12-072-CYB"] {
        r.inject_trash(0, cid);
    }
    let chaos = r.place_on_field(0, "BT12-072", Some(0));

    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        digimon_engine::selection::TriggerSource::Permanent(chaos),
    );
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("BT12-072 [Start of Main] must prompt for [Cyborg]/[Machine] in trash");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[0].battle_area[chaos.index as usize]
            .card_sources
            .len(),
        2,
        "Trash [Cyborg] card must be placed as bottom digivolution card"
    );
    assert!(r.game.players[0].trash.is_empty());
}
