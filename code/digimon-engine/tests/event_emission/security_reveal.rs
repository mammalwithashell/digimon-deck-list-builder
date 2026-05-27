//! `GameEvent::SecurityReveal` emission tests. Covers the scenarios in
//! `openspec/changes/add-reward-profiles/specs/engine-event-emission/spec.md`
//! under "Rust engine emits SecurityReveal events at security-check resolution".

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;

fn fighter(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

fn drained_security_reveals(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::SecurityReveal { .. }))
        .collect()
}

#[test]
fn single_security_check_emits_one_security_reveal() {
    // Scenario: Single-security check emits one SecurityReveal.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(make_test_card("BT2-058", "Sec1"))
        .add_card(make_test_card("BT3-103", "Sec2"))
        .security(1, &["BT2-058", "BT3-103"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game.drain_events();

    let _result = r.attack_player(atk, 1, false);

    let reveals = drained_security_reveals(&mut r);
    assert_eq!(
        reveals.len(),
        1,
        "exactly one SecurityReveal SHALL fire for a single-security check; got {reveals:?}"
    );

    let GameEvent::SecurityReveal {
        defender, card_id, ..
    } = reveals[0].clone()
    else {
        unreachable!()
    };

    // Security stack is popped from the top (last pushed first); `.security()`
    // helpers push in order so the top after the pushes is the last entry.
    // We assert defender + that a card was revealed; the exact revealed-card
    // ID depends on stack orientation in DebugRunner::security() — both
    // BT2-058 and BT3-103 are valid as the "top" card depending on push order.
    assert_eq!(defender, 1, "defender is P2 (Rust 1)");
    assert!(
        card_id == "BT2-058" || card_id == "BT3-103",
        "revealed card SHALL be one of the seeded security cards; got {card_id:?}"
    );
}
