//! Concealed source attribution must become a checked reference, never a hidden identity leak.
#[path = "determinization/support.rs"]
#[allow(dead_code)]
mod support;
use digimon_engine::{
    card_source::CardHandle,
    determinize::{beliefs::*, closed::materialize_closed},
    enums::{EffectTiming, GamePhase},
    permanent::{Permanent, PermanentHandle},
    selection::TriggerSource,
    trigger_context::TriggerContext,
    Game,
};
fn fixture(viewer: u8, cause: bool, nested: bool) -> (Game, CardHandle, WeightedDeckPrior) {
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
    let attribution = digimon_engine::trigger_context::EffectAttribution {
        controller: owner, source_card: Some(event).into(), source_permanent: None,
    };
    if cause { context.event_cause_effect = Some(attribution); }
    else { context.source_effect = Some(attribution); }
    if nested {
        let mut parent = TriggerContext::default();
        parent.simultaneous_play_members.push(context);
        context = parent;
    }
    queued.trigger_context = Some(context);
    (g, event, prior)
}

#[test]
fn concealed_attribution_alias_is_guarded_in_queued_and_cloned_contexts() {
    support::with_big_stack(|| {
        for viewer in [0, 1] { for cause in [false, true] { for nested in [false, true] {
            let (root, event, prior) = fixture(viewer, cause, nested);
            let memory_before = root.belief_knowledge(viewer).unwrap().clone();
            let mut memory = memory_before.clone();
            memory.forget_zone(1-viewer, KnowledgeZone::Hand);
            let mut changed = root.clone();
            let owner = &mut changed.players[(1-viewer) as usize];
            let index = owner.hand.iter().position(|c| c.handle()==event).unwrap();
            std::mem::swap(&mut owner.hand[index].data_index, &mut owner.deck[0].data_index);
            for seed in [3, 57] {
                let world = materialize_closed(&root, viewer, &memory, &prior, seed).unwrap();
                let alternate = materialize_closed(&changed, viewer, &memory, &prior, seed).unwrap();
                assert_eq!(support::full_zone_dump(&world), support::full_zone_dump(&alternate));
                assert_eq!(digimon_engine::build_action_mask(&root, viewer), digimon_engine::build_action_mask(&world, viewer));
                for candidate in [world.clone(), world] {
                    let context = candidate.effect_queue.iter().find(|e| e.card_id=="BT24-003").unwrap().trigger_context.as_ref().unwrap();
                    let leaf = if nested { &context.simultaneous_play_members[0] } else { context };
                    let attribution = if cause { leaf.event_cause_effect.unwrap() } else { leaf.source_effect.unwrap() };
                    assert_eq!(attribution.controller, 1-viewer);
                    assert_eq!(attribution.source_permanent, None);
                    assert!(!format!("{attribution:?}").contains(&format!("CardHandle({})", event.0)));
                    assert!(std::panic::catch_unwind(|| attribution.source_card.is_some()).is_err(), "presence reads must reject, not silently become absent");
                    assert!(std::panic::catch_unwind(|| attribution.source_card.map(|h| h.0)).is_err());
                }
            }
            assert_eq!(root.belief_knowledge(viewer).unwrap(), &memory_before);
            let context = root.effect_queue.iter().find(|e| e.card_id=="BT24-003").unwrap().trigger_context.as_ref().unwrap();
            let leaf = if nested { &context.simultaneous_play_members[0] } else { context };
            let attribution = if cause { leaf.event_cause_effect.unwrap() } else { leaf.source_effect.unwrap() };
            assert_eq!(attribution.source_card, Some(event).into());
        } } }
    });
}

#[test]
fn entitled_attribution_identity_and_absence_are_preserved() {
    support::with_big_stack(|| {
        for viewer in [0,1] { for cause in [false,true] {
            let (root,event,prior)=fixture(viewer,cause,false);
            let world=materialize_closed(&root,viewer,root.belief_knowledge(viewer).unwrap(),&prior,57).unwrap();
            let context=world.effect_queue.iter().find(|e|e.card_id=="BT24-003").unwrap().trigger_context.as_ref().unwrap();
            let attribution=if cause {context.event_cause_effect.unwrap()} else {context.source_effect.unwrap()};
            assert_eq!(attribution.source_card,Some(event).into());
            assert_eq!(attribution.controller,1-viewer);
            assert!(if cause {context.source_effect.is_none()} else {context.event_cause_effect.is_none()});
        } }
    });
}
