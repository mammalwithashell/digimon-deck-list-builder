//! Materialization must remove concealed event aliases and reject actual reads.
#[path = "determinization/support.rs"]
#[allow(dead_code)]
mod support;

use digimon_engine::{
    card_source::CardHandle,
    determinize::{beliefs::*, closed::materialize_closed},
    enums::{EffectTiming, GamePhase, Zone},
    permanent::{Permanent, PermanentHandle},
    selection::TriggerSource,
    trigger_context::{EventSubject, MovedCardSet, TriggerContext},
    Game,
};

fn fixture(viewer: u8) -> (Game, CardHandle, WeightedDeckPrior) {
    let deck: Vec<_> = support::D1
        .iter()
        .map(|id| if *id == "ST1-01" { "BT24-003" } else { *id })
        .collect();
    let cards = support::owned(&deck);
    let prior = WeightedDeckPrior::new(
        PriorSnapshot::new(
            "standard",
            "synthetic-queued-shaman-context",
            cards.iter().cloned().collect(),
            vec![CandidateDeck { cards, weight: 1.0 }],
            0.1,
            128,
        )
        .unwrap(),
    )
    .unwrap();
    let mut g = support::new_game_with(730131, &deck, &deck);
    g.enable_belief_knowledge().unwrap();
    g.start_game();
    g.mulligan_pending.clear();
    let owner = 1 - viewer;
    let event = g.players[owner as usize].security.last().unwrap().handle();
    assert!(g.add_to_hand_from_security(owner, event));
    // Install a public inherited source after the movement, then enqueue the
    // real printed observer explicitly. This is not a historical replay.
    let egg = g.players[owner as usize].digitama_deck.pop().unwrap();
    let source = egg.handle();
    let index = g.players[owner as usize]
        .hand
        .iter()
        .position(|c| c.handle() != event)
        .unwrap();
    let top = g.players[owner as usize].hand.remove(index);
    let mut permanent = Permanent::new(top, 0);
    permanent.card_sources.insert(0, egg);
    g.players[owner as usize].battle_area.push(permanent);
    g.turn_order = vec![owner, viewer];
    g.turn_player_idx = 0;
    g.current_phase = GamePhase::Main;
    g.effect_queue.clear();
    g.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::Permanent(PermanentHandle {
            player: owner,
            index: 0,
        }),
    );
    let queued = g
        .effect_queue
        .iter_mut()
        .find(|e| e.source_card == source && e.card_id == "BT24-003")
        .unwrap();
    let mut context = TriggerContext::default();
    context.event_card = (Some(event)).into();
    context.subject = (Some(EventSubject::Card {
        card: event,
        zone: Zone::Security,
    })).into();
    context.moved_card_sets.push(MovedCardSet { cards: vec![event], from: Some(Zone::Security), to: None });
    context.affected_player = Some(owner);
    queued.trigger_context = Some(context);
    (g, event, prior)
}


#[test]
fn queued_untracked_aliases_are_checked_values_and_hidden_composition_is_irrelevant() {
    support::with_big_stack(|| {
        for viewer in [0, 1] { for cloned in [false, true] {
            let (root, event, prior) = fixture(viewer);
            let root = if cloned { root.clone() } else { root };
            let mut memory = root.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let mut changed = root.clone();
            let p = &mut changed.players[(1-viewer) as usize];
            let index = p.hand.iter().position(|c| c.handle() == event).unwrap();
            std::mem::swap(&mut p.hand[index].data_index, &mut p.deck[0].data_index);
            for seed in [3, 57] {
                let a = materialize_closed(&root, viewer, &memory, &prior, seed).unwrap();
                let b = materialize_closed(&changed, viewer, &memory, &prior, seed).unwrap();
                assert_eq!(support::full_zone_dump(&a), support::full_zone_dump(&b));
                assert_eq!(digimon_engine::build_action_mask(&root, viewer), digimon_engine::build_action_mask(&a, viewer));
                for world in [&a, &a.clone(), &b] {
                    let context = world.effect_queue.iter().find(|e| e.card_id == "BT24-003").unwrap().trigger_context.as_ref().unwrap();
                    assert!(!format!("{context:?}").contains(&format!("CardHandle({})",event.0)));
                    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| context.event_card.read())).is_err());
                    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| context.subject.read())).is_err());
                    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| context.moved_card_sets.len())).is_err());
                }
            }
            assert!(root.effect_queue.iter().any(|e| e.card_id == "BT24-003" && e.trigger_context.as_ref().unwrap().event_card.read() == Some(event)));
        } }
    });
}

#[test]
fn opaque_queued_factory_is_not_executed_to_decide_projection() {
    support::with_big_stack(|| {
        use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
        struct Opaque(Arc<AtomicUsize>);
        impl digimon_engine::effect::CardEffect for Opaque {
            fn effects(&self, _: CardHandle) -> Vec<digimon_engine::effect::Effect> {
                self.0.fetch_add(1, Ordering::SeqCst); Vec::new()
            }
        }
        for viewer in [0, 1] {
            let (mut root, event, prior) = fixture(viewer);
            let calls = Arc::new(AtomicUsize::new(0));
            root.effect_registry.insert("BT24-003", Arc::new(Opaque(calls.clone())));
            let mut memory = root.belief_knowledge(viewer).unwrap().clone();
            memory.forget_zone(1-viewer, KnowledgeZone::Hand);
            assert!(materialize_closed(&root, viewer, &memory, &prior, 57).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 0, "admission cannot execute an opaque factory");
            assert!(root.effect_queue.iter().any(|e| e.card_id == "BT24-003" && e.trigger_context.as_ref().unwrap().event_card.read() == Some(event)));
        }
    });
}
