//! `GameEvent::Reveal` emission tests. Covers the `engine-event-emission`
//! requirement "Rust engine emits reveal events for non-security reveals"
//! (change `enrich-match-log-card-identity`): reveal-deck-top and
//! trash-from-deck-top.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::events::{GameEvent, RevealZone};
// NOTE: reveal-from-hand has no engine primitive, so RevealZone::Hand does
// not exist and is intentionally untested here.

fn reveals(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::Reveal { .. }))
        .collect()
}

#[test]
fn reveal_top_deck_emits_deck_top_reveal() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TOP", "Logimon"))
        .deck(0, &["TOP"])
        .start();
    r.game.drain_events();

    r.game.reveal_top_deck(0, 1);

    let evs = reveals(&mut r);
    assert_eq!(evs.len(), 1, "one reveal per revealed card; got {evs:?}");
    let GameEvent::Reveal {
        player,
        card_id,
        card_name,
        source_zone,
        ..
    } = evs[0].clone()
    else {
        unreachable!()
    };
    assert_eq!(player, 0);
    assert_eq!(card_id, "TOP");
    assert_eq!(card_name, "Logimon");
    assert!(matches!(source_zone, RevealZone::DeckTop));
}

/// On-play effect that mills the top two cards of the controller's deck.
struct MillTwo;
impl CardEffect for MillTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Mill 2")
            .process(|ctx| {
                let p = ctx.player;
                ctx.trash_from_top(p, 2);
            })
            .build()]
    }
}

#[test]
fn trash_from_deck_top_emits_per_card_reveals_in_order() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("MILLER", "Miller"))
        .add_card(make_test_card("A", "Bootmon"))
        .add_card(make_test_card("B", "Effecmon"))
        // Deck top is the LAST pushed (pop() takes the tail), so order the
        // deck so "A" then "B" come off the top.
        .deck(0, &["B", "A"])
        .start();
    r.register_effect("MILLER", Arc::new(MillTwo));

    let h = r.place_on_field(0, "MILLER", Some(0));
    r.game.drain_events();

    r.fire_on_play(0, h.index as usize);

    let evs = reveals(&mut r);
    assert_eq!(evs.len(), 2, "one reveal per milled card; got {evs:?}");
    for ev in &evs {
        let GameEvent::Reveal { source_zone, .. } = ev else {
            unreachable!()
        };
        assert!(
            matches!(source_zone, RevealZone::TrashFromDeckTop),
            "mill reveals carry the TrashFromDeckTop discriminator"
        );
    }
    let ids: Vec<String> = evs
        .iter()
        .map(|e| match e {
            GameEvent::Reveal { card_id, .. } => card_id.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(ids, vec!["A".to_string(), "B".to_string()], "top-first order");
}
