//! BT13-093 Omekamon - Digimon, Lv.4, White.
//!
//! Supported slice:
//! - [On Play] <Draw 1>.
//!
//! Gap-routed:
//! - [On Deletion] Place 1 [Royal Knight] Digimon card from hand under a
//!   [King Drasil_7D6] in breeding as bottom source. The current
//!   `select_own_breeding_permanent` step cannot filter the breeding target to
//!   King Drasil, so the clause stays omitted rather than approximated.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;

fn filler(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![CardColor::White];
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card(filler("FILLER"))
        .memory(10)
        .start()
}

#[test]
fn bt13_093_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT13-093")
        .expect("BT13-093 compiled card present");

    assert_eq!(card.name, "Omekamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert!(card.traits.iter().any(|name| name == "X Antibody"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));
}

#[test]
fn bt13_093_authors_only_supported_on_play_draw_slice() {
    let runner = runner();
    let card = runner
        .compiled_card("BT13-093")
        .expect("BT13-093 compiled card present");

    assert_eq!(
        card.effects.len(),
        1,
        "On Deletion King Drasil source placement is intentionally omitted"
    );
    let triggered = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Play clause exists");
    assert!(
        triggered
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. })),
        "On Play must be Draw 1"
    );
}

#[test]
fn bt13_093_on_play_draws_one_after_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card(filler("FILLER"))
        .hand(0, &["BT13-093"])
        .deck(0, &["FILLER"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Omekamon plays from hand");

    assert_eq!(runner.memory(), 6, "pay 4 memory, then draw 1");
    assert_eq!(
        runner.hand_size(0),
        1,
        "draw replaces the played card in hand"
    );
    assert!(runner.pending_selection().is_none());
}

#[test]
#[ignore = "pending: RK-G001 — filtered select_own_breeding_permanent target for [King Drasil_7D6]"]
fn bt13_093_on_deletion_places_royal_knight_hand_card_under_king_drasil_only() {
    panic!("requires filtered breeding permanent selection before this clause can be authored");
}
