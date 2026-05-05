//! EX7-025 ShoeShoemon.
//! Printed text covered here: [When Digivolving] if you have 1 or fewer
//! Tamers, you may play 1 Arisa Kinosaki from your hand without paying cost.
//!
//! Partial: inherited opponent security Digimon -3000 DP aura is blocked by
//! G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn ex7_025_when_digivolving_may_play_arisa_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .hand(0, &["ARISA"])
        .memory(10)
        .start();
    let shoe = runner.place_stack(0, &["BASE", "EX7-025"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shoe),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("Arisa hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "play-Arisa selection is optional"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select Arisa");
    runner.auto_resolve().expect("finish play");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_name(&runner.game.card_data) == "Arisa Kinosaki"));
}

fn make_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = digimon_engine::enums::CardKind::Tamer;
    card
}
