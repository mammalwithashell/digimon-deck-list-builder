//! Synthetic audit characterization. No model, scored seed, request or action
//! prefix is loaded. These tests do not certify projection or repair a failure.
#[path = "determinization/support.rs"]
#[allow(dead_code)]
mod support;

use digimon_engine::{
    card_source::CardHandle,
    determinize::{beliefs::*, closed::materialize_closed},
    enums::{EffectTiming, GamePhase, Zone},
    permanent::{Permanent, PermanentHandle},
    selection::TriggerSource,
    trigger_context::{EventSubject, TriggerContext},
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
    context.affected_player = Some(owner);
    queued.trigger_context = Some(context);
    (g, event, prior)
}

#[test]
fn printed_shaman_observer_is_not_currently_certified() {
    support::with_big_stack(|| {
        let file =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt24/BT24-003.yaml");
        let compiled =
            digimon_dsl::compile::compile(&digimon_dsl::loader::load_file(&file).unwrap()).unwrap();
        let mut count = 0;
        for clause in &compiled.effects {
            if let digimon_dsl::compiled::CompiledClause::Triggered(c) = clause {
                assert!(
                    !digimon_engine::dsl_cards::context_liveness::ignores_event_card_identity(c)
                );
                count += 1;
            }
        }
        assert_eq!(count, 1);
    });
}

#[test]
fn tracked_queued_context_stays_readable_and_untracked_reads_reject() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (g, event, prior) = fixture(viewer);
            let original = g.belief_knowledge(viewer).unwrap().clone();
            let world = materialize_closed(&g, viewer, &original, &prior, 57).unwrap();
            assert!(world.effect_queue.iter().any(|e| e.card_id == "BT24-003"
                && e.trigger_context.as_ref().unwrap().event_card.read() == Some(event)));
            let mut forgotten = original.clone();
            forgotten.forget_zone(1 - viewer, KnowledgeZone::Hand);
            let world = materialize_closed(&g, viewer, &forgotten, &prior, 57).unwrap();
            let saved = world.effect_queue.iter().find(|e| e.card_id == "BT24-003")
                .unwrap().trigger_context.as_ref().unwrap();
            assert!(std::panic::catch_unwind(|| saved.event_card.read()).is_err());
            assert!(std::panic::catch_unwind(|| saved.subject.read()).is_err());
            assert!(!format!("{saved:?}").contains(&format!("CardHandle({})", event.0)));
            assert_eq!(g.belief_knowledge(viewer).unwrap(), &original);
            assert!(g.effect_queue.iter().any(|e| e.card_id == "BT24-003"
                && e.trigger_context.as_ref().unwrap().event_card.read() == Some(event)));
        }
    });
}

#[test]
fn queued_context_does_not_reveal_true_concealed_identity() {
    support::with_big_stack(|| {
        for viewer in 0..2 {
            let (g, event, prior) = fixture(viewer);
            let memory = g.belief_knowledge(viewer).unwrap();
            let mut changed = g.clone();
            let owner = &mut changed.players[(1 - viewer) as usize];
            let hand = owner.hand.iter().position(|c| c.handle() == event).unwrap();
            std::mem::swap(
                &mut owner.hand[hand].data_index,
                &mut owner.deck[0].data_index,
            );
            for seed in 0..8 {
                let a = materialize_closed(&g, viewer, memory, &prior, seed).unwrap();
                let b = materialize_closed(&changed, viewer, memory, &prior, seed).unwrap();
                assert_eq!(support::full_zone_dump(&a), support::full_zone_dump(&b));
            }
        }
    });
}
