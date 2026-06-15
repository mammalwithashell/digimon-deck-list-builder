//! `GameEvent::MemoryChange` effect-source attribution tests. Covers the
//! `engine-event-emission` requirement "GameEvent::MemoryChange carries
//! optional effect-source identity" (change `enrich-match-log-card-identity`).

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::events::GameEvent;

/// On-play effect that gains 1 memory — stands in for a tamer's
/// start-of-turn `+memory`. The source card is whatever played it.
struct GainOneMemory;
impl CardEffect for GainOneMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 1 memory")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

fn memory_changes(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::MemoryChange { .. }))
        .collect()
}

#[test]
fn effect_driven_memory_gain_carries_source_identity() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TAMER", "Cyber Engage"))
        .start();
    r.register_effect("TAMER", Arc::new(GainOneMemory));

    let h = r.place_on_field(0, "TAMER", Some(0));
    r.game.drain_events();

    r.fire_on_play(0, h.index as usize);

    let changes = memory_changes(&mut r);
    assert!(
        !changes.is_empty(),
        "the on-play gain SHALL emit a MemoryChange"
    );
    let GameEvent::MemoryChange {
        source_card_id,
        source_card_name,
        ..
    } = changes[0].clone()
    else {
        unreachable!()
    };
    assert_eq!(
        source_card_id.as_deref(),
        Some("TAMER"),
        "effect-driven memory change attributes the source card id"
    );
    assert_eq!(
        source_card_name.as_deref(),
        Some("Cyber Engage"),
        "effect-driven memory change attributes the source card name"
    );
}

#[test]
fn structural_memory_change_is_unattributed() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("X", "X"))
        .start();
    r.game.drain_events();

    // A direct (non-effect) memory mutation — cost-payment / structural path.
    r.game.gain_memory_for_player(0, 1);

    let changes = memory_changes(&mut r);
    assert!(!changes.is_empty(), "a MemoryChange SHALL still emit");
    let GameEvent::MemoryChange {
        source_card_id,
        source_card_name,
        ..
    } = changes[0].clone()
    else {
        unreachable!()
    };
    assert_eq!(source_card_id, None, "structural change carries no source id");
    assert_eq!(
        source_card_name, None,
        "structural change carries no source name"
    );
}
