//! Adapter-level checks, additional to production belief tests. Not full qualification.
#[path = "../examples/search_quality/frozen_modes.rs"]
mod frozen_modes;
#[path = "determinization/support.rs"]
mod support;
use digimon_engine::{
    card_registry::CardRegistry,
    determinize::beliefs::*,
    search::{decision_player, DeterminizedConfig, UniformEvaluator},
};
use frozen_modes::InformationRegime;

fn prior() -> WeightedDeckPrior {
    let cards = support::owned(support::D1);
    WeightedDeckPrior::new(
        PriorSnapshot::new(
            "standard",
            "adapter-unit-fixture",
            cards.iter().cloned().collect(),
            vec![CandidateDeck { cards, weight: 1.0 }],
            0.1,
            128,
        )
        .unwrap(),
    )
    .unwrap()
}

fn full_budget() -> DeterminizedConfig {
    let mut cfg = DeterminizedConfig::default();
    cfg.worlds = 4;
    cfg.search.simulations = 320;
    cfg.search.dirichlet_epsilon = 0.;
    cfg.search.seed = 77;
    cfg
}

#[test]
fn adapter_search_and_dispatch_preserve_pending_digivolve_cost_choices() {
    support::with_big_stack(|| {
        use digimon_engine::{card_data::EvoCost, enums::{CardColor, GamePhase, PlaySource},
            game::Game, permanent::Permanent, rules::Rules};
        // Synthetic costs on starter identities isolate the continuation mechanism;
        // this does not assert that these are the printed costs of those cards.
        let mut data = support::card_data().clone();
        data.get_mut("ST1-02").unwrap().colors = vec![CardColor::Red, CardColor::Blue];
        data.get_mut("ST1-04").unwrap().evo_costs = vec![
            EvoCost { card_color: 0, level: 3, memory_cost: 0 },
            EvoCost { card_color: 1, level: 3, memory_cost: 1 }];
        let registry = CardRegistry::from_cards(&data);
        for viewer in 0..2u8 {
            let mut g = Game::new(&[support::owned(support::D1),support::owned(support::D1)],
                &data, Rules::standard(), Some(71)).unwrap();
            let p=&mut g.players[viewer as usize];
            let mut available=std::mem::take(&mut p.hand);available.append(&mut p.deck);available.append(&mut p.security);
            let i=available.iter().position(|c|c.card_id(&g.card_data)=="ST1-04").unwrap();
            p.hand.push(available.remove(i));
            let i=available.iter().position(|c|c.card_id(&g.card_data)=="ST1-02").unwrap();
            p.battle_area.push(Permanent::new(available.remove(i),0));p.deck=available;
            g.enable_belief_knowledge().unwrap();
            g.start_game(); g.mulligan_pending.clear(); g.current_phase=GamePhase::Main;
            g.turn_order=vec![viewer,1-viewer];g.turn_player_idx=0;g.memory_pair=(viewer,1-viewer);g.memory=10;
            assert!(!g.digivolve_from_hand(viewer,0,0,PlaySource::ByHand));
            let choices=g.pending_selection.as_ref().unwrap().effect_choices.as_ref().unwrap();
            assert_eq!(choices.len(),2);
            let before=g.to_scenario();
            let (result,_) = frozen_modes::search(&registry,&g,&mut UniformEvaluator,&full_budget(),
                InformationRegime::ClosedList,Some(&prior())).unwrap();
            assert_eq!(result.simulations_run,320);
            assert_eq!(result.root_plans.len(),2);
            for (route,_,_) in &result.root_plans {
                let mut clone=g.clone();
                for action in route {
                    let actor=decision_player(&clone);
                    assert!(digimon_engine::build_action_mask(&clone,actor)[*action as usize]>0.5);
                    clone.decode_action(*action,actor);
                }
                assert_eq!(clone.players[viewer as usize].battle_area[0].top_card().card_id(&clone.card_data),"ST1-04");
                assert!(clone.pending_selection.is_none());
                assert!(clone.memory==10 || clone.memory==9);
            }
            assert_eq!(before,g.to_scenario());
        }
    });
}

#[test]
fn adapter_search_and_dispatch_preserve_pending_reveal_bucket() {
    support::with_big_stack(|| {
        use digimon_dsl::compiled::{CompiledBindingRef,CompiledPlayerRef,CompiledRevealBucket,CompiledStep};
        use digimon_engine::{effect_context::EffectContext,enums::GamePhase,
            dsl_cards::{bindings::Bindings,step::{run_steps,RunOutcome}}};
        let registry=CardRegistry::from_cards(support::card_data());
        for viewer in 0..2u8 {
            let mut g=support::new_game_with(34,support::D1,support::D1);
            g.enable_belief_knowledge().unwrap();
            g.start_game();g.mulligan_pending.clear();g.current_phase=GamePhase::Main;
            g.turn_order=vec![viewer,1-viewer];g.turn_player_idx=0;g.memory_pair=(viewer,1-viewer);g.memory=5;
            let source=g.players[viewer as usize].hand[0].handle();
            let cards=g.reveal_top_deck(viewer,3);
            let steps=vec![CompiledStep::SelectRevealBuckets {
                from:"revealed".into(),buckets:vec![CompiledRevealBucket { bind_as:"picked".into(),filter:None,min:1,max:1 }],
                no_duplicate_cards:true,prompt:Some("Adapter fixture: choose one revealed card".into()) },
                CompiledStep::AddToHandFromReveal { of:CompiledPlayerRef::You,card:CompiledBindingRef::Named("picked".into()) }];
            let mut bindings=Bindings::new();bindings.insert_card_list("revealed",cards);
            assert_eq!(run_steps(&steps,&mut EffectContext::new(&mut g,source,None,viewer),&mut bindings),RunOutcome::Parked);
            assert!(g.pending_selection.is_some());
            let before=g.to_scenario();let hand=g.players[viewer as usize].hand.len();
            let (result,_) = frozen_modes::search(&registry,&g,&mut UniformEvaluator,&full_budget(),
                InformationRegime::ClosedList,Some(&prior())).unwrap();
            assert!(!result.root_plans.is_empty());
            for (route,_,_) in &result.root_plans {
                let mut clone=g.clone();
                for action in route {
                    let actor=decision_player(&clone);
                    assert!(digimon_engine::build_action_mask(&clone,actor)[*action as usize]>0.5);
                    clone.decode_action(*action,actor);
                }
                assert!(clone.pending_selection.is_none());
                assert_eq!(clone.players[viewer as usize].hand.len(),hand+1);
            }
            assert_eq!(before,g.to_scenario());
        }
    });
}

#[test]
fn adapter_rejects_transformed_concealed_cards_without_exact_list_fallback() {
    support::with_big_stack(|| {
        let registry = CardRegistry::from_cards(support::card_data());
        for viewer in 0..2u8 {
            let mut game = support::new_game_with(11, support::D1, support::D1);
            game.mulligan_pending = vec![viewer, 1-viewer];
            game.enable_belief_knowledge().unwrap();
            game.players[(1-viewer) as usize].hand[0].also_treated_as.push("concealed-transform".into());
            let before = game.to_scenario();
            let error = frozen_modes::search(&registry, &game, &mut UniformEvaluator, &full_budget(),
                InformationRegime::ClosedList, Some(&prior())).err().expect("unsupported state must fail");
            assert!(error.contains("transformed concealed"), "{error}");
            assert_eq!(before, game.to_scenario());
        }
    });
}

#[test]
fn adapter_contradictory_candidate_uses_declared_population_fallback() {
    support::with_big_stack(|| {
        let registry = CardRegistry::from_cards(support::card_data());
        let mut deck = support::owned(support::D1);
        let replace = deck.iter().position(|id| id == "ST1-02").unwrap();
        deck[replace] = "ST1-08".into(); // Three copies; candidate prior has only two.
        let refs: Vec<_> = deck.iter().map(String::as_str).collect();
        for viewer in 0..2u8 {
            let mut game = if viewer == 0 {
                support::new_game_with(17, support::D1, &refs)
            } else {
                support::new_game_with(17, &refs, support::D1)
            };
            game.mulligan_pending = vec![viewer, 1-viewer];
            let p = &mut game.players[(1-viewer) as usize];
            for zone in [&mut p.hand, &mut p.deck, &mut p.security] {
                let mut i = 0;
                while i < zone.len() {
                    if zone[i].card_id(&game.card_data) == "ST1-08" { p.trash.push(zone.remove(i)); }
                    else { i += 1; }
                }
            }
            assert_eq!(p.trash.len(), 3);
            game.enable_belief_knowledge().unwrap();
            let before = game.to_scenario();
            let (result, telemetry) = frozen_modes::search(&registry, &game, &mut UniformEvaluator,
                &full_budget(), InformationRegime::ClosedList, Some(&prior())).unwrap();
            assert_eq!(result.simulations_run, 320);
            assert_eq!(telemetry["information"]["compatible_candidates"], 0);
            assert_eq!(telemetry["information"]["fallback_mass"], 1.0);
            assert_eq!(telemetry["information"]["fallback_worlds"], 4);
            assert_eq!(telemetry["information"]["sampled_worlds"], 4);
            assert_eq!(before, game.to_scenario());
        }
    });
}

#[test]
fn adapter_concealed_sources_are_sampled_without_leaking_opponent_identity() {
    support::with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, permanent::{Permanent, PermanentHandle}};
        let registry = CardRegistry::from_cards(support::card_data());
        for viewer in 0..2u8 {
            let mut game = support::new_game_with(177, support::D1, support::D1);
            game.enable_belief_knowledge().unwrap();
            for owner in 0..2u8 {
                let card = game.players[owner as usize].hand.pop().unwrap();
                game.players[owner as usize].battle_area.push(Permanent::new(card, 0));
                let host = PermanentHandle { player: owner, index: 0 };
                let source = game.players[owner as usize].battle_area[0].top_card().handle();
                EffectContext::new(&mut game, source, Some(host), owner)
                    .training_place_deck_top_under_self_face_down(host);
            }
            // Real decode refreshes owner inspection memory; fixtures then select either seat.
            let first = decision_player(&game);
            game.decode_action(0, first);
            game.mulligan_pending = vec![viewer, 1-viewer];
            let mut changed = game.clone();
            changed.players[(1-viewer) as usize].battle_area[0].card_sources[0].data_index = changed.card_data
                .iter().position(|c| c.card_id == "ST2-02").unwrap();
            let before = [game.to_scenario(), changed.to_scenario()];
            let (a, mut ta) = frozen_modes::search(&registry, &game, &mut UniformEvaluator, &full_budget(),
                InformationRegime::ClosedList, Some(&prior())).unwrap();
            let (b, mut tb) = frozen_modes::search(&registry, &changed, &mut UniformEvaluator, &full_budget(),
                InformationRegime::ClosedList, Some(&prior())).unwrap();
            assert_eq!(a,b);
            for t in [&mut ta, &mut tb] {
                t.as_object_mut().unwrap().remove("seconds");
                t["information"].as_object_mut().unwrap().remove("sampling_seconds");
            }
            assert_eq!(ta,tb);
            assert_eq!(before,[game.to_scenario(),changed.to_scenario()]);
        }
    });
}

#[test]
fn adapter_preserves_seat_private_topdeck_memory_and_shuffle_invalidation() {
    support::with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, determinize::closed::materialize_closed};
        use sha2::{Digest, Sha256};
        let registry = CardRegistry::from_cards(support::card_data());
        let prior = prior();
        for viewer in 0..2u8 {
            let mut g = support::new_game_with(182, support::D1, support::D1);
            g.mulligan_pending = vec![viewer, 1-viewer];
            g.enable_belief_knowledge().unwrap();
            let card = g.players[viewer as usize].hand[0].clone();
            let id = card.card_id(&g.card_data).to_string();
            assert!(EffectContext::new(&mut g, card.handle(), None, viewer)
                .return_hand_card_to_deck(viewer, card.handle(), false));
            assert!(g.belief_knowledge(viewer).unwrap().known_positions(viewer)
                .contains(&(id.clone(), KnowledgeZone::Deck, 0)));
            assert!(!g.belief_knowledge(1-viewer).unwrap().known_hidden(viewer)
                .contains(&(id.clone(), KnowledgeZone::Deck)));
            let world = materialize_closed(&g, viewer, g.belief_knowledge(viewer).unwrap(), &prior, 77).unwrap();
            assert_eq!(world.players[viewer as usize].deck.last().unwrap().handle(), card.handle());
            for shuffled in [false, true] {
                if shuffled {
                    g.shuffle_deck(viewer);
                    assert!(g.belief_knowledge(viewer).unwrap().known_positions(viewer).is_empty());
                    assert!(!g.belief_knowledge(1-viewer).unwrap().known_hidden(viewer)
                        .contains(&(id.clone(), KnowledgeZone::Deck)));
                }
                let memories = [g.belief_knowledge(0).unwrap().clone(),g.belief_knowledge(1).unwrap().clone()];
                let scenario = g.to_scenario();
                let evidence = BeliefEvidence::extract(&g, viewer, &memories[viewer as usize]).unwrap();
                let expected_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&evidence).unwrap()));
                let mut cfg = DeterminizedConfig::default();
                cfg.worlds=4; cfg.search.simulations=320; cfg.search.dirichlet_epsilon=0.; cfg.search.seed=77;
                let (_, telemetry) = frozen_modes::search(&registry,&g,&mut UniformEvaluator,&cfg,
                    InformationRegime::ClosedList,Some(&prior)).unwrap();
                assert_eq!(telemetry["viewer"],viewer);
                assert_eq!(telemetry["information"]["evidence_sha256"],expected_hash);
                assert_eq!(g.to_scenario(),scenario);
                for seat in 0..2 {assert_eq!(g.belief_knowledge(seat).unwrap(),&memories[seat as usize]);}
            }
        }
    });
}

#[test]
fn adapter_public_recovery_enters_each_sampled_opponent_hand() {
    support::with_big_stack(|| {
        let registry = CardRegistry::from_cards(support::card_data());
        let prior = prior();
        for viewer in 0..2u8 {
            let mut g = support::new_game_with(22,support::D1,support::D1);
            g.mulligan_pending=vec![viewer,1-viewer];
            g.enable_belief_knowledge().unwrap();
            let owner=1-viewer;
            let c=g.players[owner as usize].hand.pop().unwrap();
            let handle=c.handle();let id=c.card_id(&g.card_data).to_string();
            g.players[owner as usize].trash.push(c);
            assert!(g.add_to_hand_from_trash(owner,handle));
            for seat in 0..2 {assert!(g.belief_knowledge(seat).unwrap().known_hidden(owner)
                .contains(&(id.clone(),KnowledgeZone::Hand)));}
            let mut cfg=DeterminizedConfig::default();
            cfg.worlds=4;cfg.search.simulations=320;cfg.search.dirichlet_epsilon=0.;cfg.search.seed=77;
            let (_,t)=frozen_modes::search(&registry,&g,&mut UniformEvaluator,&cfg,
                InformationRegime::ClosedList,Some(&prior)).unwrap();
            assert_eq!(t["information"]["sampled_worlds"],4);
            assert!(t["information"]["sampled_opponent_hand_counts"][&id].as_u64().unwrap()>=4);
        }
    });
}

#[test]
fn closed_mode_requires_prior_and_complete_knowledge() {
    support::with_big_stack(|| {
        let game = support::new_game_with(7, support::D1, support::D1);
        let registry = CardRegistry::from_cards(support::card_data());
        let mut config = DeterminizedConfig::default();
        config.worlds = 4;
        config.search.simulations = 320;
        config.search.dirichlet_epsilon = 0.;
        assert!(frozen_modes::search(
            &registry,
            &game,
            &mut UniformEvaluator,
            &config,
            InformationRegime::ClosedList,
            None
        )
        .is_err());
        assert!(frozen_modes::search(
            &registry,
            &game,
            &mut UniformEvaluator,
            &config,
            InformationRegime::ClosedList,
            Some(&prior())
        )
        .is_err());
    });
}

#[test]
fn actor_closed_search_ignores_true_hidden_order_and_future_rng() {
    support::with_big_stack(|| {
        use rand::SeedableRng;
        let mut game = support::new_game_with(7, support::D1, support::D1);
        game.enable_belief_knowledge().unwrap();
        let viewer = decision_player(&game);
        let before = digimon_engine::serialization::to_ui_json(&game);
        let mut changed = game.clone();
        changed.players[(1 - viewer) as usize].deck.reverse();
        changed.rng = rand::rngs::StdRng::seed_from_u64(18191);
        let registry = CardRegistry::from_cards(support::card_data());
        let mut config = DeterminizedConfig::default();
        config.worlds = 4;
        config.search.simulations = 320;
        config.search.dirichlet_epsilon = 0.;
        config.search.seed = 77;
        let prior = prior();
        let (a, mut ad) = frozen_modes::search(
            &registry,
            &game,
            &mut UniformEvaluator,
            &config,
            InformationRegime::ClosedList,
            Some(&prior),
        )
        .unwrap();
        let (b, mut bd) = frozen_modes::search(
            &registry,
            &changed,
            &mut UniformEvaluator,
            &config,
            InformationRegime::ClosedList,
            Some(&prior),
        )
        .unwrap();
        assert_eq!(a, b);
        for value in [&mut ad, &mut bd] {
            value.as_object_mut().unwrap().remove("seconds");
            value["information"]
                .as_object_mut()
                .unwrap()
                .remove("sampling_seconds");
        }
        assert_eq!(ad, bd);
        assert_eq!(before, digimon_engine::serialization::to_ui_json(&game));
    });
}

#[test]
fn adapter_is_invariant_to_true_opponent_composition_for_either_seat() {
    support::with_big_stack(|| {
        use digimon_engine::{
            action::build_action_mask,
            determinize::closed::materialize_closed,
            observation::{build_observation_tensor, ObservationProfileId},
        };
        let registry = CardRegistry::from_cards(support::card_data());
        let prior = prior();
        for viewer in 0..2u8 {
            let mut a = support::new_game_with(11, support::D1, support::D1);
            let mut b = if viewer == 0 {
                support::new_game_with(11, support::D1, support::D3)
            } else {
                support::new_game_with(11, support::D3, support::D1)
            };
            // Synthetic decision-order fixture; no claimed full-game coverage.
            for game in [&mut a, &mut b] {
                game.mulligan_pending = vec![viewer, 1 - viewer];
                game.enable_belief_knowledge().unwrap();
            }
            assert_eq!(decision_player(&a), viewer);
            assert_eq!(decision_player(&b), viewer);
            let originals = [a.to_scenario(), b.to_scenario()];
            assert_ne!(originals[0], originals[1]);
            assert_eq!(build_action_mask(&a, viewer), build_action_mask(&b, viewer));
            assert_eq!(
                build_observation_tensor(
                    &a,
                    viewer,
                    &registry,
                    ObservationProfileId::StandardLiteV2
                ),
                build_observation_tensor(
                    &b,
                    viewer,
                    &registry,
                    ObservationProfileId::StandardLiteV2
                )
            );
            for seed in [77, 78, 79, 80] {
                let wa = materialize_closed(
                    &a,
                    viewer,
                    a.belief_knowledge(viewer).unwrap(),
                    &prior,
                    seed,
                )
                .unwrap();
                let wb = materialize_closed(
                    &b,
                    viewer,
                    b.belief_knowledge(viewer).unwrap(),
                    &prior,
                    seed,
                )
                .unwrap();
                assert_eq!(wa.to_scenario(), wb.to_scenario());
            }
            let mut config = DeterminizedConfig::default();
            config.worlds = 4;
            config.search.simulations = 320;
            config.search.dirichlet_epsilon = 0.;
            config.search.seed = 77;
            let (ra, mut da) = frozen_modes::search(
                &registry,
                &a,
                &mut UniformEvaluator,
                &config,
                InformationRegime::ClosedList,
                Some(&prior),
            )
            .unwrap();
            let (rb, mut db) = frozen_modes::search(
                &registry,
                &b,
                &mut UniformEvaluator,
                &config,
                InformationRegime::ClosedList,
                Some(&prior),
            )
            .unwrap();
            assert_eq!(ra, rb);
            for d in [&mut da, &mut db] {
                d.as_object_mut().unwrap().remove("seconds");
                d["information"]
                    .as_object_mut()
                    .unwrap()
                    .remove("sampling_seconds");
            }
            assert_eq!(da, db);
            assert_eq!(originals, [a.to_scenario(), b.to_scenario()]);
        }
    });
}
