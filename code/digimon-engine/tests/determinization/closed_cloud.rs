//! Exercise bounded real cloud-list decision prefixes, including parked effects.
//! The separate native pool probe checks full-game completion/truncation.
use super::support::*;
use digimon_engine::determinize::{beliefs::*, closed::materialize_closed};

#[test]
fn native_chronomon_security_history_positions_remain_consistent() {
    with_big_stack(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/chronomon-security-history.json")).unwrap();
        let cards: Vec<String> = serde_json::from_value(fixture["cards"].clone()).unwrap();
        let refs: Vec<_> = cards.iter().map(String::as_str).collect();
        let mut game = new_game_with(fixture["game_seed"].as_u64().unwrap(), &refs, &refs);
        game.enable_belief_knowledge().unwrap();
        game.turn_count = 1;
        for (decision, step) in fixture["actions"].as_array().unwrap().iter().enumerate() {
            let player = step[0].as_u64().unwrap() as u8;
            let action = step[1].as_u64().unwrap() as u16;
            assert_eq!(decision_player(&game), player, "{step}");
            assert!(
                digimon_engine::build_action_mask(&game, player)[action as usize] > 0.5,
                "{step}"
            );
            game.decode_action(action, player);
            for seat in 0..2 {
                let facts = serde_json::to_value(game.belief_knowledge(seat).unwrap()).unwrap();
                for fact in facts["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .chain(facts["concealed_positions"].as_array().unwrap())
                    .filter(|c| !c["position"].is_null())
                {
                    let owner = fact["owner"].as_u64().unwrap() as usize;
                    let zone = match fact["zone"].as_str().unwrap() {
                        "deck" => &game.players[owner].deck,
                        "security" => &game.players[owner].security,
                        "digitama" => &game.players[owner].digitama_deck,
                        _ => continue,
                    };
                    let actual = zone
                        .iter()
                        .rev()
                        .position(|c| c.card_index == fact["instance"].as_u64().unwrap() as u16);
                    assert_eq!(
                        actual,
                        Some(fact["position"].as_u64().unwrap() as usize),
                        "after decision {decision} {step}, seat {seat}, stale {fact}"
                    );
                }
            }
        }
        let prior = WeightedDeckPrior::new(
            PriorSnapshot::new(
                "standard",
                "native-security-history",
                cards.iter().cloned().collect(),
                vec![CandidateDeck {
                    cards: cards.clone(),
                    weight: 1.0,
                }],
                0.1,
                128,
            )
            .unwrap(),
        )
        .unwrap();
        let viewer = decision_player(&game);
        materialize_closed(
            &game,
            viewer,
            game.belief_knowledge(viewer).unwrap(),
            &prior,
            88,
        )
        .unwrap();
    });
}

#[test]
fn event_card_liveness_proof_rejects_context_reads_and_unaudited_shapes() {
    use digimon_dsl::compiled::*;
    use digimon_engine::dsl_cards::context_liveness::ignores_event_card_identity as safe;
    let mut clause = CompiledTriggeredClause {
        when: vec![CompiledTiming::OnOwnSecurityRemoved],
        scope: CompiledScope::FaceUp,
        active_when: None,
        condition: None,
        trigger_if: None,
        timing_conditions: vec![],
        optional: true,
        outer_prompt: false,
        once_per_turn: false,
        max_per_turn: None,
        process: vec![CompiledStep::GainMemory(1)],
        summary: None,
        summary_key: None,
    };
    assert!(safe(&clause));
    clause.condition = Some(CompiledPredicate {
        event_card_name_contains: Some("Greymon".into()),
        ..Default::default()
    });
    assert!(!safe(&clause));
    clause.condition = None;
    clause.process = vec![CompiledStep::EffectInitiatedDigivolve {
        target: CompiledBindingRef::EventCard,
        from_hand: CompiledBindingRef::Named("selected".into()),
        cost: CompiledCostDelta::Free,
        ignore_requirements: false,
        ignore_level: false,
    }];
    assert!(!safe(&clause));
    clause.process = vec![CompiledStep::If {
        condition: CompiledPredicate::default(),
        then: vec![],
        else_branch: vec![],
    }];
    assert!(
        !safe(&clause),
        "unreviewed variants remain strict, even when empty"
    );
    clause.process = vec![CompiledStep::GainMemory(1)];
    clause.when = vec![CompiledTiming::OnPlay];
    assert!(!safe(&clause), "implicit timing gates also require review");
    assert!(
        !digimon_engine::effect::EffectBuilder::new(
            digimon_engine::card_source::CardHandle(0),
            digimon_engine::enums::EffectTiming::OnOwnSecurityRemoved
        )
        .build()
        .ignores_event_card_identity
    );
}

#[test]
fn identity_independent_queued_observer_does_not_keep_a_concealed_event_card() {
    with_big_stack(|| {
        use digimon_engine::{
            enums::{EffectTiming, Zone},
            permanent::{Permanent, PermanentHandle},
            selection::TriggerSource,
            trigger_context::{EventSubject, TriggerContext},
        };
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../data/training_pools/ex13-20261026/distinct_training_decks.json"
        ))
        .unwrap();
        let cards: Vec<String> = serde_json::from_value(rows[2]["cards"].clone()).unwrap();
        let refs: Vec<_> = cards.iter().map(String::as_str).collect();
        let prior = WeightedDeckPrior::new(
            PriorSnapshot::new(
                "standard",
                "queued-context-regression",
                cards.iter().cloned().collect(),
                vec![CandidateDeck {
                    cards: cards.clone(),
                    weight: 1.0,
                }],
                0.1,
                128,
            )
            .unwrap(),
        )
        .unwrap();
        let mut g = new_game_with(92, &refs, &refs);
        let index = g.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&g.card_data) == "BT24-084")
            .or_else(|| {
                let i = g.players[0]
                    .deck
                    .iter()
                    .position(|c| c.card_id(&g.card_data) == "BT24-084")
                    .unwrap();
                let c = g.players[0].deck.remove(i);
                g.players[0].hand.push(c);
                Some(g.players[0].hand.len() - 1)
            })
            .unwrap();
        let card = g.players[0].hand.remove(index);
        g.players[0].battle_area.push(Permanent::new(card, 0));
        // Drive a real selection/evolution, including its cost and trigger chain.
        let index = g.players[0]
            .deck
            .iter()
            .position(|c| c.card_id(&g.card_data) == "BT24-031")
            .unwrap();
        let base = g.players[0].deck.remove(index);
        g.players[0].battle_area.push(Permanent::new(base, 0));
        let index = g.players[0]
            .deck
            .iter()
            .position(|c| c.card_id(&g.card_data) == "BT24-034")
            .unwrap();
        let evolution = g.players[0].deck.remove(index);
        g.players[0].hand.push(evolution);
        let hidden = g.players[1].hand[0].handle();
        g.enable_belief_knowledge().unwrap();
        let mut context = TriggerContext::default();
        context.event_card = Some(hidden);
        context.target_card = Some(hidden);
        context.subject = Some(EventSubject::Card {
            card: hidden,
            zone: Zone::Hand,
        });
        context.affected_player = Some(0);
        g.enqueue_triggered(
            EffectTiming::OnOwnSecurityRemoved,
            TriggerSource::Permanent(PermanentHandle {
                player: 0,
                index: 0,
            }),
        );
        assert!(!g.effect_queue.is_empty());
        g.effect_queue[0].trigger_context = Some(context);
        let memory = g.belief_knowledge(0).unwrap().clone();
        let a = materialize_closed(&g, 0, &memory, &prior, 88).unwrap();
        let mut changed = g.clone();
        changed.players[1].hand[0].data_index = changed
            .card_data
            .iter()
            .position(|c| c.card_id == "ST1-02")
            .unwrap();
        changed.players[1].original_deck.clear();
        let b = materialize_closed(&changed, 0, &memory, &prior, 88).unwrap();
        assert_eq!(full_zone_dump(&a), full_zone_dump(&b));
        assert!(g.effect_queue[0]
            .trigger_context
            .as_ref()
            .unwrap()
            .event_card
            .is_some());
        assert!(a.effect_queue[0]
            .trigger_context
            .as_ref()
            .unwrap()
            .event_card
            .is_none());
        // Hold the hypothetical physical state fixed so a draw is not a confounder.
        let mut original = a.clone();
        original.effect_queue[0].trigger_context = g.effect_queue[0].trigger_context.clone();
        let mut sampled = a;
        original.drain_effect_queue();
        sampled.drain_effect_queue();
        for _ in 0..8 {
            let player = decision_player(&original);
            let ma = digimon_engine::build_action_mask(&original, player);
            let mb = digimon_engine::build_action_mask(&sampled, player);
            assert_eq!(ma, mb);
            let Some(action) = ma.iter().position(|v| *v > 0.5) else {
                break;
            };
            if original.pending_selection.is_none() {
                break;
            }
            original.decode_action(action as u16, player);
            sampled.decode_action(action as u16, player);
            assert_eq!(
                original.players[0].battle_area[0].is_suspended,
                sampled.players[0].battle_area[0].is_suspended
            );
        }
        assert_eq!(original.to_scenario(), sampled.to_scenario());
        assert!(sampled.players[0].battle_area[0].is_suspended);
        assert!(sampled.players[0].battle_area[1].card_sources.len() > 1);
    });
}

#[test]
fn cloud_pool_continuations_are_composition_independent_and_resolve() {
    with_big_stack(|| {
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../data/training_pools/ex13-20261026/distinct_training_decks.json"
        ))
        .unwrap();
        let rows = rows.as_array().unwrap();
        let mut failures = Vec::new();
        let mut checked = 0;
        let mut selected = 0;
        let focus = std::env::var("DIGIMON_BELIEF_TEST_CASE")
            .ok()
            .map(|v| v.parse::<usize>().unwrap());
        for (case, row) in rows.iter().enumerate() {
            if focus.is_some_and(|f| f != case) {
                continue;
            }
            for offset in [0u64, 1000] {
                selected += 1;
                eprintln!(
                    "cloud continuation case {case}, seed offset {offset}: {}",
                    row["archetype"]
                );
                let cards: Vec<String> = serde_json::from_value(row["cards"].clone()).unwrap();
                let refs: Vec<&str> = cards.iter().map(String::as_str).collect();
                let prior = WeightedDeckPrior::new(
                    PriorSnapshot::new(
                        "standard",
                        "cloud-continuation-regression",
                        cards.iter().cloned().collect(),
                        vec![CandidateDeck {
                            cards: cards.clone(),
                            weight: 1.0,
                        }],
                        0.1,
                        128,
                    )
                    .unwrap(),
                )
                .unwrap();
                let mut game = new_game_with(20261029 + case as u64 + offset, &refs, &refs);
                game.enable_belief_knowledge().unwrap();
                let mut rng = Lcg(1949 + case as u64 + offset);
                for decision in 0..128 {
                    if game.game_over {
                        break;
                    }
                    let viewer = decision_player(&game);
                    let memory = game.belief_knowledge(viewer).unwrap().clone();
                    let seed = decision as u64 + 999;
                    let world = match materialize_closed(&game, viewer, &memory, &prior, seed) {
                        Ok(w) => w,
                        Err(e) => {
                            let locate = |handle| {
                                game.players.iter().find_map(|p| {
                                    [
                                        ("hand", &p.hand),
                                        ("deck", &p.deck),
                                        ("security", &p.security),
                                        ("trash", &p.trash),
                                    ]
                                    .into_iter()
                                    .find_map(
                                        |(zone, cards)| {
                                            cards
                                                .iter()
                                                .position(|c| c.handle() == handle)
                                                .map(|index| (p.id, zone, index))
                                        },
                                    )
                                })
                            };
                            if e.contains("position") {
                                let facts = serde_json::to_value(&memory).unwrap();
                                for fact in facts["cards"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .filter(|c| !c["position"].is_null())
                                {
                                    let instance = fact["instance"].as_u64().unwrap() as u16;
                                    eprintln!("position forensic fact={fact} location={:?} deck_counts={:?}",
                                    locate(digimon_engine::card_source::CardHandle(instance)),
                                    game.players.iter().map(|p| (p.deck.len(), p.security.len())).collect::<Vec<_>>());
                                }
                            }
                            if e.contains("effect queue") {
                                for q in game.effect_queue.iter() {
                                    eprintln!("queue forensic source={:?} controller={} card={} location={:?} attribution={:?}",
                                    q.source_card, q.controller, q.card_id, locate(q.source_card), q.attribution_source_card);
                                }
                            }
                            let location = match game
                                .pending_selection_resume
                                .as_ref()
                                .and_then(|s| s.frames.last())
                            {
                                Some(digimon_engine::resume::ResumeFrame::RunTail {
                                    prov, ..
                                }) => format!(" source location {:?}", locate(prov.source_card)),
                                _ => String::new(),
                            };
                            eprintln!("case {case} decision {decision}: {e}{location}");
                            if let Some(digimon_engine::resume::ResumeFrame::RunTail {
                                trigger_context: Some(c),
                                ..
                            }) = game
                                .pending_selection_resume
                                .as_ref()
                                .and_then(|s| s.frames.last())
                            {
                                for h in [
                                    c.source_effect.and_then(|e| e.source_card),
                                    c.target_card,
                                    c.event_card,
                                    c.event_source_card,
                                    c.event_host_card,
                                ]
                                .into_iter()
                                .flatten()
                                {
                                    eprintln!(
                                        "context forensic handle={h:?} location={:?}",
                                        locate(h)
                                    );
                                }
                            }
                            failures.push(format!("case {case} offset {offset} {} decision {decision}: {e}; frame {:?}",
                            row["archetype"], game.pending_selection_resume));
                            break;
                        }
                    };
                    // Corrupt only the source's UNENTITLED opposing composition. The
                    // same restricted evidence and seed must still give the same world.
                    let mut other = game.clone();
                    let entitled: serde_json::Value = serde_json::to_value(&memory).unwrap();
                    let known: std::collections::HashSet<u16> = entitled["cards"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|c| c["instance"].as_u64().unwrap() as u16)
                        .collect();
                    {
                        let replacement = other
                            .card_data
                            .iter()
                            .position(|c| c.card_id == "ST1-02")
                            .unwrap();
                        let opp = &mut other.players[(1 - viewer) as usize];
                        for c in opp
                            .hand
                            .iter_mut()
                            .chain(&mut opp.deck)
                            .chain(&mut opp.security)
                        {
                            if !known.contains(&c.card_index) {
                                c.data_index = replacement;
                            }
                        }
                        for c in opp
                            .battle_area
                            .iter_mut()
                            .chain(opp.breeding_area.iter_mut())
                            .flat_map(|p| p.card_sources.iter_mut().chain(&mut p.linked_cards))
                            .filter(|c| c.face_down && !known.contains(&c.card_index))
                        {
                            c.data_index = replacement;
                        }
                        opp.original_deck.clear();
                    }
                    let alternate =
                        materialize_closed(&other, viewer, &memory, &prior, seed).unwrap();
                    assert_eq!(full_zone_dump(&world), full_zone_dump(&alternate));
                    assert_eq!(world.to_scenario(), alternate.to_scenario());
                    let mask = digimon_engine::build_action_mask(&game, viewer);
                    let action = rng.pick(&mask).expect("nonterminal legal decision");
                    let mut a = world;
                    let mut b = alternate;
                    a.decode_action(action, viewer);
                    b.decode_action(action, viewer);
                    assert_eq!(a.to_scenario(), b.to_scenario());
                    checked += 1;
                    let before_frame = format!("{:?}", game.pending_selection_resume);
                    game.decode_action(action, viewer);
                    for seat in 0..2 {
                        let facts =
                            serde_json::to_value(game.belief_knowledge(seat).unwrap()).unwrap();
                        for fact in facts["cards"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .chain(facts["concealed_positions"].as_array().unwrap().iter())
                            .filter(|c| !c["position"].is_null())
                        {
                            let owner = fact["owner"].as_u64().unwrap() as usize;
                            let cards = match fact["zone"].as_str().unwrap() {
                                "deck" => &game.players[owner].deck,
                                "security" => &game.players[owner].security,
                                "digitama" => &game.players[owner].digitama_deck,
                                _ => continue,
                            };
                            let position = fact["position"].as_u64().unwrap() as usize;
                            let actual = cards.iter().rev().position(|c| {
                                c.card_index == fact["instance"].as_u64().unwrap() as u16
                            });
                            assert_eq!(actual, Some(position), "case {case} decision {decision} action {action} seat {seat} stale fact {fact}; before {before_frame}");
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        assert!(
            selected > 0 && checked >= selected * 2,
            "every selected list must exercise both seats"
        );
    });
}
