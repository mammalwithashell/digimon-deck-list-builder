use super::support::*;
use digimon_engine::determinize::{beliefs::*, closed::materialize_closed};
use digimon_engine::search::{
    determinized_search_sampled, DeterminizationMode, DeterminizedConfig, UniformEvaluator,
};

fn prior() -> WeightedDeckPrior {
    let cards: Vec<String> = D1.iter().map(|c| c.to_string()).collect();
    WeightedDeckPrior::new(
        PriorSnapshot::new(
            "standard",
            "starter-counting-fixture",
            cards.iter().cloned().collect(),
            vec![CandidateDeck { cards, weight: 1.0 }],
            0.1,
            128,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn public_security_reveals_are_sampled_at_known_slots_until_shuffle() {
    with_big_stack(|| {
        let mut game = new_game_with(93, D1, D1);
        game.enable_belief_knowledge().unwrap();
        game.start_game();
        game.mulligan_pending.clear();
        game.current_phase = digimon_engine::enums::GamePhase::Main;
        game.reveal_security(1);
        let known: Vec<_> = game.players[1]
            .security
            .iter()
            .enumerate()
            .map(|(index, card)| {
                (
                    card.handle(),
                    card.card_id(&game.card_data).to_string(),
                    game.players[1].security.len() - index - 1,
                )
            })
            .collect();

        for viewer in 0..2 {
            let memory = game.belief_knowledge(viewer).unwrap();
            for (_, id, from_top) in &known {
                assert!(memory.known_positions(1).contains(&(
                    id.clone(),
                    KnowledgeZone::Security,
                    *from_top
                )));
            }
            for seed in 0..8 {
                let world = materialize_closed(&game, viewer, memory, &prior(), seed).unwrap();
                for (handle, id, from_top) in &known {
                    let index = world.players[1].security.len() - from_top - 1;
                    let card = &world.players[1].security[index];
                    assert_eq!(card.handle(), *handle);
                    assert_eq!(card.card_id(&world.card_data), id);
                }
            }
        }

        game.shuffle_security(1);
        assert!(game.players[1]
            .security
            .iter()
            .all(|card| !game.security_card_is_public(1, card.handle())));
        let memory = game.belief_knowledge(0).unwrap();
        assert!(!memory
            .known_positions(1)
            .iter()
            .any(|(_, zone, _)| { *zone == KnowledgeZone::Security }));

        let mut alternate = game.clone();
        alternate.players[1].security.swap(0, 1);
        let original = materialize_closed(&game, 0, memory, &prior(), 117).unwrap();
        let sampled_alternate = materialize_closed(&alternate, 0, memory, &prior(), 117).unwrap();
        assert_eq!(original.to_scenario(), sampled_alternate.to_scenario());
    });
}

fn nested_security_game(
    private_parent: bool,
) -> (
    digimon_engine::Game,
    digimon_engine::card_source::CardHandle,
) {
    use digimon_engine::{
        enums::GamePhase,
        selection::{PendingEffectSecurityRemoval, PendingSecurity, SecurityRemovalDestination},
        trigger_context::EventCause,
    };
    let mut g = new_game_with(93, D1, D1);
    g.enable_belief_knowledge().unwrap();
    g.start_game();
    g.mulligan_pending.clear();
    g.current_phase = GamePhase::Main;
    let parent = g.players[1].security.pop().unwrap();
    let parent_handle = parent.handle();
    let current = g.players[1].security.pop().unwrap();
    g.pending_security = Some(PendingSecurity {
        defender: 1,
        card: current,
        played: false,
    });
    g.pending_effect_security_removal = vec![
        PendingEffectSecurityRemoval {
            defender: 1,
            observer_player: 0,
            source_player: 0,
            cause: EventCause::OwnEffect,
            destination: if private_parent {
                SecurityRemovalDestination::Hand(1)
            } else {
                SecurityRemovalDestination::Trash
            },
            previous_pending_security: None,
            discard_security_fired: true,
        },
        PendingEffectSecurityRemoval {
            defender: 1,
            observer_player: 0,
            source_player: 0,
            cause: EventCause::OwnEffect,
            destination: SecurityRemovalDestination::Trash,
            previous_pending_security: Some(PendingSecurity {
                defender: 1,
                card: parent,
                played: false,
            }),
            discard_security_fired: true,
        },
    ];
    (g, parent_handle)
}

#[test]
fn nested_public_security_holders_remain_resolvable_and_conserve_both_worlds() {
    with_big_stack(|| {
        let (g, parent) = nested_security_game(false);
        assert!(
            g.card_source_for_handle(parent).is_some(),
            "a parked public security card still exists"
        );
        for viewer in 0..2 {
            let memory = g.belief_knowledge(viewer).unwrap();
            let w = materialize_closed(&g, viewer, memory, &prior(), 17).unwrap();
            assert_eq!(
                w.card_data_for_handle(parent).unwrap().card_id,
                g.card_data_for_handle(parent).unwrap().card_id
            );
            digimon_engine::determinize::closed::check_hypothesis(&w).unwrap();
        }
    });
}

#[test]
fn nested_private_security_parent_is_not_a_public_reveal() {
    with_big_stack(|| {
        let (g, _) = nested_security_game(true);
        let memory = g.belief_knowledge(0).unwrap();
        let evidence = BeliefEvidence::extract(&g, 0, memory).unwrap();
        assert_eq!(evidence.accounted.values().sum::<usize>(), 1);
        let mut other = g.clone();
        other.pending_effect_security_removal[1]
            .previous_pending_security
            .as_mut()
            .unwrap()
            .card
            .data_index = other.players[0].hand[0].data_index;
        assert_eq!(
            evidence,
            BeliefEvidence::extract(&other, 0, memory).unwrap()
        );
        assert!(materialize_closed(&g, 0, memory, &prior(), 17)
            .err()
            .unwrap()
            .contains("concealed security transit"));
    });
}

#[test]
fn unrevealed_security_return_tracks_position_without_learning_identity() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, enums::GamePhase};
        let mut g = new_game_with(93, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.start_game();
        g.mulligan_pending.clear();
        g.current_phase = GamePhase::Main;
        let remembered = g.players[1].hand.last().unwrap().handle();
        let own_source = g.players[1].hand[0].handle();
        assert!(EffectContext::new(&mut g, own_source, None, 1)
            .return_hand_card_to_deck(1, remembered, false));
        g.shuffle_deck(1); // A membership-only known card must not consume the later opaque pin.
        let card = g.players[1].security.last().unwrap().clone();
        let source = g.players[1].hand[0].handle();
        let mut twin = g.clone();
        {
            let p = &mut twin.players[1];
            let different = p
                .deck
                .iter()
                .position(|c| c.card_index != remembered.0 && c.data_index != card.data_index)
                .unwrap();
            let top = p.security.len() - 1;
            std::mem::swap(
                &mut p.deck[different].data_index,
                &mut p.security[top].data_index,
            );
        }
        assert!(
            EffectContext::new(&mut g, source, None, 1).return_security_card_to_deck(
                1,
                card.handle(),
                true
            )
        );
        assert!(
            EffectContext::new(&mut twin, source, None, 1).return_security_card_to_deck(
                1,
                card.handle(),
                true
            )
        );
        for viewer in 0..2 {
            let memory = g.belief_knowledge(viewer).unwrap().clone();
            assert_eq!(
                &memory,
                twin.belief_knowledge(viewer).unwrap(),
                "the move cannot learn an unrevealed ID, including historical minima"
            );
            let json = serde_json::to_value(&memory).unwrap();
            assert!(!json["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["instance"] == card.card_index));
            assert!(json["concealed_positions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["instance"] == card.card_index && c["zone"] == "deck"));
            let mut ids = std::collections::BTreeSet::new();
            for seed in 0..32 {
                let w = materialize_closed(&g, viewer, &memory, &prior(), seed).unwrap();
                assert_eq!(w.players[1].deck[0].handle(), card.handle());
                ids.insert(w.players[1].deck[0].card_id(&w.card_data).to_string());
                let mut alternate = g.clone();
                if viewer == 0 {
                    alternate.players[1].deck[0].data_index =
                        alternate.players[0].hand[0].data_index;
                } else {
                    // The owner's unseen joint multiset is unchanged. Only
                    // the real assignment to the concealed role is changed.
                    let p = &mut alternate.players[1];
                    std::mem::swap(&mut p.deck[0].data_index, &mut p.security[0].data_index);
                }
                let other =
                    materialize_closed(&alternate, viewer, &memory, &prior(), seed).unwrap();
                assert_eq!(full_zone_dump(&w), full_zone_dump(&other));
            }
            assert!(
                ids.len() > 1,
                "the unknown returned identity must remain uncertain on both seats"
            );
        }
        // A long game can reach that same bottom card. Its lineage follows a
        // publicly determined draw without revealing it to the other player.
        let count = g.players[1].deck.len() - 1;
        g.reveal_top_deck(1, count as u8);
        assert!(g.draw_one_for_player(1).unwrap());
        let memory = g.belief_knowledge(0).unwrap();
        let json = serde_json::to_value(memory).unwrap();
        assert!(json["concealed_positions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["instance"] == card.card_index && c["zone"] == "hand"));
        let w = materialize_closed(&g, 0, memory, &prior(), 9).unwrap();
        assert!(w.players[1]
            .hand
            .iter()
            .any(|c| c.handle() == card.handle()));

        // A captured event reference must resolve against the hypothesis,
        // including after that unknown card has been drawn into a hidden hand.
        use digimon_dsl::compiled::{CompiledFormula, CompiledStep};
        use digimon_engine::{
            action::space::HAND_EFFECT_START,
            dsl_cards::{bindings::Bindings, step::StepRuntime},
            enums::EffectSourceKind,
            resume::{ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack},
            selection::{PendingSelection, SelectionKind},
            trigger_context::TriggerContext,
        };
        use std::sync::Arc;
        let source_card = g.players[0].hand[0].handle();
        g.turn_order = vec![0, 1];
        g.turn_player_idx = 0;
        g.memory = 0;
        g.current_phase = GamePhase::EffectChoice;
        g.pending_selection = Some(PendingSelection {
            card_choice: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![HAND_EFFECT_START],
            is_optional: false,
            prompt: "resolve concealed event reference".into(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| panic!("must use typed resume")),
            on_decline: None,
            zone_owner: None,
        });
        let mut event_context = TriggerContext::default();
        event_context.event_card = Some(card.handle());
        g.pending_selection_resume = Some(ResumeStack {
            frames: vec![ResumeFrame::RunTail {
                prov: ResumeProvenance {
                    source_card,
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    controller: 0,
                    override_pin: None,
                },
                select_kind: ResumeSelectKind::EffectChoice { post: None },
                bind_as: None,
                inner_tail: Arc::new(vec![CompiledStep::GainMemoryFn {
                    formula: CompiledFormula::EventTargetLevel,
                }]),
                outer_conts: vec![],
                bindings: Bindings::new(),
                runtime: StepRuntime::default(),
                trigger_context: Some(event_context),
                decline: ResumeDecline::None,
            }],
        });
        for seed in 0..8 {
            let mut w =
                materialize_closed(&g, 0, g.belief_knowledge(0).unwrap(), &prior(), seed).unwrap();
            let expected = w
                .card_data_for_handle(card.handle())
                .unwrap()
                .level
                .unwrap_or(0) as i16;
            w.decode_action(HAND_EFFECT_START, 0);
            assert_eq!(w.memory, expected);
        }
    });
}

#[test]
fn face_up_security_return_retains_its_legitimate_identity() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, enums::GamePhase};
        let mut g = new_game_with(93, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.start_game();
        g.current_phase = GamePhase::Main;
        let card = g.players[1].security.last().unwrap().clone();
        g.players[1].face_up_security.insert(card.card_index);
        let source = g.players[1].hand[0].handle();
        assert!(
            EffectContext::new(&mut g, source, None, 1).return_security_card_to_deck(
                1,
                card.handle(),
                true
            )
        );
        for viewer in 0..2 {
            for seed in 0..8 {
                let w = materialize_closed(
                    &g,
                    viewer,
                    g.belief_knowledge(viewer).unwrap(),
                    &prior(),
                    seed,
                )
                .unwrap();
                assert_eq!(
                    w.players[1].deck[0].card_id(&w.card_data),
                    card.card_id(&g.card_data)
                );
                assert_eq!(w.players[1].deck[0].handle(), card.handle());
            }
        }
    });
}

#[test]
fn field_to_security_preserves_public_or_owner_only_identity_entitlements() {
    with_big_stack(|| {
        use digimon_engine::{
            effect_context::EffectContext,
            enums::{CardSourceRef, GamePhase, StackPosition},
            permanent::{Permanent, PermanentHandle},
        };
        for concealed in [false, true] {
            let mut g = new_game_with(93, D1, D1);
            g.enable_belief_knowledge().unwrap();
            g.start_game();
            g.mulligan_pending.clear();
            g.current_phase = GamePhase::Main;
            let carrier = g.players[1].hand.pop().unwrap();
            let carrier_card = carrier.handle();
            g.players[1].battle_area.push(Permanent::new(carrier, 0));
            let host = PermanentHandle {
                player: 1,
                index: 0,
            };
            if concealed {
                EffectContext::new(&mut g, carrier_card, Some(host), 1)
                    .training_place_deck_top_under_self_face_down(host);
            }
            let card = g.players[1].battle_area[0].card_sources[0].clone();
            let source = g.players[0].hand[0].handle();
            assert!(
                EffectContext::new(&mut g, source, None, 0).place_on_security(
                    1,
                    CardSourceRef::Material(host, 0),
                    StackPosition::Bottom,
                    false
                )
            );
            for viewer in 0..2 {
                let memory = g.belief_knowledge(viewer).unwrap();
                let json = serde_json::to_value(memory).unwrap();
                let identified = json["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["instance"] == card.card_index && c["zone"] == "security");
                assert_eq!(identified, !concealed || viewer == 1);
                for seed in 0..8 {
                    let w = materialize_closed(&g, viewer, memory, &prior(), seed).unwrap();
                    assert_eq!(w.players[1].security[0].handle(), card.handle());
                    if identified {
                        assert_eq!(
                            w.players[1].security[0].card_id(&w.card_data),
                            card.card_id(&g.card_data)
                        );
                    }
                }
            }
        }
    });
}

#[test]
fn security_to_hand_reveals_only_to_the_owner_and_samples_the_foreign_reference() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, enums::GamePhase};
        let mut g = new_game_with(93, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.start_game();
        g.mulligan_pending.clear();
        g.current_phase = GamePhase::Main;
        let card = g.players[1].security.last().unwrap().clone();
        let source = g.players[0].hand[0].handle();
        assert!(
            EffectContext::new(&mut g, source, None, 0).add_to_hand_from_security(1, card.handle())
        );
        g.draw_one_for_player(0).unwrap(); // Refresh completed movement entitlements.
        for viewer in 0..2 {
            let memory = g.belief_knowledge(viewer).unwrap();
            let json = serde_json::to_value(memory).unwrap();
            assert_eq!(
                json["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["instance"] == card.card_index),
                viewer == 1
            );
            let mut ids = std::collections::BTreeSet::new();
            for seed in 0..16 {
                let w = materialize_closed(&g, viewer, memory, &prior(), seed).unwrap();
                let sampled = w.players[1]
                    .hand
                    .iter()
                    .find(|c| c.handle() == card.handle())
                    .unwrap();
                ids.insert(sampled.card_id(&w.card_data).to_string());
            }
            if viewer == 0 {
                assert!(ids.len() > 1);
            } else {
                assert_eq!(
                    ids,
                    [card.card_id(&g.card_data).to_string()]
                        .into_iter()
                        .collect()
                );
            }
        }
    });
}

#[test]
fn concealed_field_identity_is_sampled_and_owner_inspection_is_preserved() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, permanent::Permanent};
        let mut game = new_game_with(177, D1, D1);
        game.enable_belief_knowledge().unwrap();
        for owner in 0..2 {
            let card = game.players[owner].hand.pop().unwrap();
            game.players[owner]
                .battle_area
                .push(Permanent::new(card, 0));
            let host = digimon_engine::permanent::PermanentHandle {
                player: owner as u8,
                index: 0,
            };
            let source = game.players[owner].battle_area[0].top_card().handle();
            EffectContext::new(&mut game, source, Some(host), owner as u8)
                .training_place_deck_top_under_self_face_down(host);
        }
        // The next decode refreshes the opt-in memory, including owner peeks.
        let player = decision_player(&game);
        let action = Lcg(9)
            .pick(&digimon_engine::build_action_mask(&game, player))
            .unwrap();
        game.decode_action(action, player);
        let memory = game.belief_knowledge(0).unwrap();
        let evidence = BeliefEvidence::extract(&game, 0, memory).unwrap();
        assert_eq!(evidence.field_count, 1);
        assert!(memory
            .known_hidden(0)
            .iter()
            .any(|(_, z)| *z == KnowledgeZone::Field));
        assert!(!memory
            .known_hidden(1)
            .iter()
            .any(|(_, z)| *z == KnowledgeZone::Field));
        let mut other = game.clone();
        other.players[1].battle_area[0].card_sources[0].data_index = other
            .card_data
            .iter()
            .position(|c| c.card_id == "ST2-02")
            .unwrap();
        let a = materialize_closed(&game, 0, memory, &prior(), 117).unwrap();
        let b = materialize_closed(&other, 0, memory, &prior(), 117).unwrap();
        assert_eq!(a.to_scenario(), b.to_scenario());
        assert_eq!(
            a.players[0].battle_area[0].card_sources[0].data_index,
            game.players[0].battle_area[0].card_sources[0].data_index
        );
        assert!(a.players[1].battle_area[0].card_sources[0].face_down);
        digimon_engine::determinize::closed::check_hypothesis(&a).unwrap();
    });
}

#[test]
fn unknown_field_slot_cannot_consume_a_later_pinned_singleton() {
    with_big_stack(|| {
        use digimon_engine::permanent::Permanent;
        let mut cards = owned(D1);
        let replace = cards.iter().position(|c| c == "ST1-11").unwrap();
        cards[replace] = "ST1-12".into();
        let refs: Vec<_> = cards.iter().map(String::as_str).collect();
        let mut game = new_game_with(181, D1, &refs);
        let prior = WeightedDeckPrior::new(
            PriorSnapshot::new(
                "standard",
                "field singleton fixture",
                cards.iter().cloned().collect(),
                vec![CandidateDeck { cards, weight: 1.0 }],
                0.1,
                128,
            )
            .unwrap(),
        )
        .unwrap();
        let p = &mut game.players[1];
        let mut all = std::mem::take(&mut p.hand);
        all.append(&mut p.deck);
        all.append(&mut p.security);
        let k = all
            .iter()
            .position(|c| c.card_id(&game.card_data) == "ST1-11")
            .unwrap();
        let mut known = all.remove(k);
        let mut unknown = all.pop().unwrap();
        let host = all.pop().unwrap();
        let pin = known.handle();
        known.face_down = true;
        unknown.face_down = true;
        let mut permanent = Permanent::new(host, 0);
        permanent.card_sources.insert(0, known);
        permanent.card_sources.insert(0, unknown);
        p.battle_area.push(permanent);
        p.hand = all.drain(..5).collect();
        p.security = all.drain(..5).collect();
        p.deck = all;
        let mut memory = KnowledgeLedger::new(0).unwrap();
        memory
            .observe(1, pin.0, "ST1-11", KnowledgeZone::Field, None)
            .unwrap();
        let mut exercised_singleton = false;
        for seed in 0..8 {
            let world = materialize_closed(&game, 0, &memory, &prior, seed).unwrap();
            assert_eq!(
                world.players[1].battle_area[0].card_sources[1].card_id(&world.card_data),
                "ST1-11"
            );
            if world.players[1]
                .original_deck
                .iter()
                .any(|r| r.card_id == "ST1-11" && r.count == 1)
            {
                exercised_singleton = true;
                assert_ne!(
                    world.players[1].battle_area[0].card_sources[0].card_id(&world.card_data),
                    "ST1-11"
                );
            }
        }
        assert!(exercised_singleton);
    });
}

#[test]
fn pre_field_knowledge_prior_is_not_silently_reused() {
    let mut snapshot = prior().snapshot().clone();
    snapshot.schema_version = 1;
    assert!(WeightedDeckPrior::new(snapshot).is_err());
}

#[test]
fn public_field_return_to_deck_retains_source_identity_for_parked_effects() {
    with_big_stack(|| {
        use digimon_engine::{enums::StackPosition, permanent::Permanent};
        let mut game = new_game_with(178, D1, D1);
        game.enable_belief_knowledge().unwrap();
        let card = game.players[1].hand.pop().unwrap();
        let id = card.card_id(&game.card_data).to_string();
        game.players[1].battle_area.push(Permanent::new(card, 0));
        let host = digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0,
        };
        assert!(game.return_to_deck(host, StackPosition::Top));
        for viewer in 0..2 {
            assert!(game
                .belief_knowledge(viewer)
                .unwrap()
                .known_hidden(1)
                .contains(&(id.clone(), KnowledgeZone::Deck)));
        }
        materialize_closed(&game, 0, game.belief_knowledge(0).unwrap(), &prior(), 8).unwrap();
    });
}

#[test]
fn departed_public_tokens_do_not_count_as_deck_cards_or_collide_with_hypothetical_handles() {
    with_big_stack(|| {
        use digimon_engine::effect_context::EffectContext;
        let mut game = new_game_with(180, D1, D1);
        game.enable_belief_knowledge().unwrap();
        let source = game.players[0].hand[0].handle();
        let host = EffectContext::new(&mut game, source, None, 0)
            .play_token(0, "petrification")
            .unwrap();
        let token = game.players[0]
            .battle_area
            .remove(host.index as usize)
            .top_card()
            .handle();
        let world =
            materialize_closed(&game, 0, game.belief_knowledge(0).unwrap(), &prior(), 8).unwrap();
        assert!(world.players[1]
            .hand
            .iter()
            .chain(&world.players[1].deck)
            .all(|c| c.card_index > token.0));
        assert!(!game
            .belief_knowledge(0)
            .unwrap()
            .known_hidden(0)
            .iter()
            .any(|(id, _)| id.starts_with("TOKEN")));
        digimon_engine::determinize::closed::check_hypothesis(&world).unwrap();
    });
}

#[test]
fn public_trash_return_and_private_hand_return_have_distinct_entitlements() {
    with_big_stack(|| {
        use digimon_engine::effect_context::EffectContext;
        let mut game = new_game_with(182, D1, D1);
        game.enable_belief_knowledge().unwrap();
        let public = game.players[1].hand.pop().unwrap();
        let public_id = public.card_id(&game.card_data).to_string();
        let public_handle = public.handle();
        game.players[1].trash.push(public);
        let private = game.players[0].hand[0].clone();
        let private_id = private.card_id(&game.card_data).to_string();
        let source = private.handle();
        let mut ctx = EffectContext::new(&mut game, source, None, 0);
        ctx.return_trash_cards_to_deck_bottom(1, &[public_handle]);
        ctx.return_hand_card_to_deck(0, source, false);
        for viewer in 0..2 {
            assert!(game
                .belief_knowledge(viewer)
                .unwrap()
                .known_hidden(1)
                .contains(&(public_id.clone(), KnowledgeZone::Deck)));
        }
        game.draw_one_for_player(1).unwrap();
        game.reveal_top_deck(1, 2);
        let bottom_position = game.players[1].deck.len() - 1;
        for viewer in 0..2 {
            assert!(game
                .belief_knowledge(viewer)
                .unwrap()
                .known_positions(1)
                .contains(&(public_id.clone(), KnowledgeZone::Deck, bottom_position)));
        }
        assert!(game
            .belief_knowledge(0)
            .unwrap()
            .known_positions(0)
            .contains(&(private_id.clone(), KnowledgeZone::Deck, 0)));
        assert!(!game
            .belief_knowledge(1)
            .unwrap()
            .known_hidden(0)
            .contains(&(private_id, KnowledgeZone::Deck)));
        let world =
            materialize_closed(&game, 0, game.belief_knowledge(0).unwrap(), &prior(), 8).unwrap();
        assert_eq!(world.players[0].deck.last().unwrap().handle(), source);
    });
}

#[test]
fn foreign_hand_bindings_follow_entitled_physical_pins_in_sampled_continuations() {
    with_big_stack(|| {
        use digimon_dsl::compiled::{CompiledFormula, CompiledStep};
        use digimon_engine::{
            action::space::HAND_EFFECT_START,
            dsl_cards::{bindings::Bindings, step::StepRuntime},
            enums::{EffectSourceKind, GamePhase},
            permanent::{Permanent, PermanentHandle},
            resume::{ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack},
            selection::{PendingSelection, SelectionKind},
        };
        use std::sync::Arc;
        let mut game = new_game_with(179, D1, D1);
        game.enable_belief_knowledge().unwrap();
        let source = game.players[0].hand.pop().unwrap();
        let source_card = source.handle();
        game.players[0].battle_area.push(Permanent::new(source, 0));
        let known = game.players[1].hand.remove(0);
        let handle = known.handle();
        let expected_cost = known.play_cost(&game.card_data) as i16;
        game.players[1].trash.push(known);
        game.add_to_hand_from_trash(1, handle);
        let original_index = game.players[1]
            .hand
            .iter()
            .position(|c| c.handle() == handle)
            .unwrap();
        let mut bindings = Bindings::new();
        bindings.insert_hand_index_pinned("known", 1, original_index as u16, Some(handle));
        let host = Some(PermanentHandle {
            player: 0,
            index: 0,
        });
        game.mulligan_pending.clear();
        game.current_phase = GamePhase::EffectChoice;
        game.turn_order = vec![0, 1];
        game.turn_player_idx = 0;
        game.memory = 0;
        game.pending_selection = Some(PendingSelection {
            card_choice: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![HAND_EFFECT_START],
            is_optional: false,
            prompt: "resolve pinned card formula".into(),
            effect_choices: None,
            source_card,
            source_permanent: host,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| panic!("must use typed resume")),
            on_decline: None,
            zone_owner: None,
        });
        game.pending_selection_resume = Some(ResumeStack {
            frames: vec![ResumeFrame::RunTail {
                prov: ResumeProvenance {
                    source_card,
                    source_permanent: host,
                    source_kind: EffectSourceKind::Digimon,
                    controller: 0,
                    override_pin: None,
                },
                select_kind: ResumeSelectKind::EffectChoice { post: None },
                bind_as: None,
                inner_tail: Arc::new(vec![CompiledStep::GainMemoryFn {
                    formula: CompiledFormula::BindingPlayCost("known".into()),
                }]),
                outer_conts: vec![],
                bindings,
                runtime: StepRuntime::default(),
                trigger_context: None,
                decline: ResumeDecline::None,
            }],
        });
        let mut moved = false;
        for seed in 0..8 {
            let mut world =
                materialize_closed(&game, 0, game.belief_knowledge(0).unwrap(), &prior(), seed)
                    .unwrap();
            let index = world.players[1]
                .hand
                .iter()
                .position(|c| c.handle() == handle)
                .unwrap();
            moved |= index != original_index;
            if let ResumeFrame::RunTail { bindings, .. } =
                &world.pending_selection_resume.as_ref().unwrap().frames[0]
            {
                assert_eq!(
                    bindings.get_hand_index_with_player("known"),
                    Some((1, index as u16))
                );
            }
            world.decode_action(HAND_EFFECT_START, 0);
            assert_eq!(world.memory, expected_cost);
        }
        assert!(moved, "exercise at least one changed foreign hand position");
    });
}

#[test]
fn pinned_hand_formula_never_reads_a_replacement_card_from_an_old_slot() {
    with_big_stack(|| {
        use digimon_dsl::compiled::CompiledFormula;
        use digimon_engine::{
            dsl_cards::{bindings::Bindings, formula_eval},
            effect_context::{EffectContext, EffectReadContext},
            permanent::PermanentHandle,
        };
        let mut game = new_game_with(183, D1, D1);
        let p = &mut game.players[0];
        let mut cards = std::mem::take(&mut p.hand);
        cards.append(&mut p.deck);
        cards.append(&mut p.security);
        let a = cards
            .iter()
            .position(|c| c.card_id(&game.card_data) == "ST1-02")
            .unwrap();
        let first = cards.remove(a);
        let b = cards
            .iter()
            .position(|c| c.card_id(&game.card_data) == "ST1-10")
            .unwrap();
        let second = cards.remove(b);
        let handle = first.handle();
        let cost = first.play_cost(&game.card_data) as i32;
        assert_ne!(cost, second.play_cost(&game.card_data) as i32);
        p.hand = vec![first, second];
        p.deck = cards;
        let mut bindings = Bindings::new();
        bindings.insert_hand_index_pinned("picked", 0, 0, Some(handle));
        p.hand.swap(0, 1);
        let formula = CompiledFormula::BindingPlayCost("picked".into());
        let target = PermanentHandle {
            player: 0,
            index: 0,
        };
        assert_eq!(
            formula_eval::evaluate_with_bindings(
                &formula,
                &EffectContext::new(&mut game, handle, None, 0),
                target,
                Some(&bindings)
            ),
            cost
        );
        let card = game.players[0].hand.pop().unwrap();
        assert_eq!(card.handle(), handle);
        game.players[0].trash.push(card);
        assert_eq!(
            formula_eval::evaluate_with_bindings(
                &formula,
                &EffectContext::new(&mut game, handle, None, 0),
                target,
                Some(&bindings)
            ),
            0
        );
        assert_eq!(
            formula_eval::evaluate_read_with_bindings(
                &formula,
                &EffectReadContext::new(&game, handle, None, 0),
                target,
                Some(&bindings)
            ),
            0
        );
    });
}

#[test]
fn different_true_compositions_produce_identical_closed_worlds_and_search() {
    with_big_stack(|| {
        let mut a = new_game_with(11, D1, D1);
        a.mulligan_pending = vec![0, 1];
        let mut b = new_game_with(11, D1, D3);
        b.mulligan_pending = vec![0, 1];
        let memory = KnowledgeLedger::new(0).unwrap();
        let p = prior();
        let wa = materialize_closed(&a, 0, &memory, &p, 77).unwrap();
        let wb = materialize_closed(&b, 0, &memory, &p, 77).unwrap();
        assert_eq!(wa.to_scenario(), wb.to_scenario());
        assert!(wb.players[1]
            .hand
            .iter()
            .chain(&wb.players[1].deck)
            .any(|c| c.card_id(&wb.card_data).starts_with("ST1-")));
        assert_eq!(full_zone_dump(&wa), full_zone_dump(&wb));
        let registry = digimon_engine::card_registry::CardRegistry::from_cards(card_data());
        for mode in [DeterminizationMode::Pimc, DeterminizationMode::IsMcts] {
            let mut config = DeterminizedConfig::default();
            config.mode = mode;
            config.worlds = 2;
            config.search.simulations = 4;
            let ra = determinized_search_sampled(
                &registry,
                &a,
                0,
                &mut UniformEvaluator,
                &config,
                1,
                &mut |seed| materialize_closed(&a, 0, &memory, &p, seed),
            )
            .unwrap();
            let rb = determinized_search_sampled(
                &registry,
                &b,
                0,
                &mut UniformEvaluator,
                &config,
                1,
                &mut |seed| materialize_closed(&b, 0, &memory, &p, seed),
            )
            .unwrap();
            assert_eq!(ra, rb);
        }
    });
}

#[test]
fn a_public_hand_departure_preserves_remembered_cards_of_other_identities() {
    with_big_stack(|| {
        let mut g = new_game_with(22, D1, D1);
        g.enable_belief_knowledge().unwrap();
        let card = g.players[1].hand.pop().unwrap();
        let handle = card.handle();
        let id = card.card_id(&g.card_data).to_string();
        g.players[1].trash.push(card);
        assert!(g.add_to_hand_from_trash(1, handle));
        let other = g.players[1]
            .hand
            .iter()
            .position(|c| c.card_id(&g.card_data) != id)
            .unwrap();
        assert!(g.trash_from_hand_by_index(1, other).is_some());
        // A later facade refresh must not confuse the public departure with
        // an unknown hand shrink and erase the independently remembered card.
        g.draw_one_for_player(0).unwrap();
        let memory = g.belief_knowledge(0).unwrap();
        assert!(memory
            .known_hidden(1)
            .contains(&(id.clone(), KnowledgeZone::Hand)));
        let world = materialize_closed(&g, 0, memory, &prior(), 25).unwrap();
        assert!(world.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&world.card_data) == id));
        let same = g.players[1]
            .hand
            .iter()
            .position(|c| c.handle() == handle)
            .unwrap();
        assert!(g.trash_from_hand_by_index(1, same).is_some());
        g.draw_one_for_player(0).unwrap();
        assert!(!g
            .belief_knowledge(0)
            .unwrap()
            .known_hidden(1)
            .contains(&(id, KnowledgeZone::Hand)));
    });
}

#[test]
fn public_recovery_is_remembered_by_both_seats_and_survives_clone() {
    with_big_stack(|| {
        let mut g = new_game_with(22, D1, D1);
        g.enable_belief_knowledge().unwrap();
        let card = g.players[1].hand.pop().unwrap();
        let handle = card.handle();
        let id = card.card_id(&g.card_data).to_string();
        g.players[1].trash.push(card);
        assert!(g.add_to_hand_from_trash(1, handle));
        for viewer in 0..2 {
            assert!(g
                .belief_knowledge(viewer)
                .unwrap()
                .known_hidden(1)
                .contains(&(id.clone(), KnowledgeZone::Hand)));
        }
        let clone = g.clone();
        assert_eq!(
            g.belief_knowledge(0).unwrap(),
            clone.belief_knowledge(0).unwrap()
        );
        let p = prior();
        let world = materialize_closed(&g, 0, g.belief_knowledge(0).unwrap(), &p, 25).unwrap();
        assert!(world.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&world.card_data) == id));
    });
}

#[test]
fn closed_selfplay_generates_provenanced_shards_on_both_seats() {
    with_big_stack(|| {
        let mut config = digimon_engine::selfplay::SelfPlayConfig::default();
        config.games = 2;
        config.seed = 71;
        config.search.worlds = 2;
        config.search.search.simulations = 4;
        config.belief_prior = Some(prior().snapshot().clone());
        let dir = std::env::temp_dir().join(format!("closed-selfplay-{}", std::process::id()));
        let result = digimon_engine::selfplay::run_self_play(
            card_data(),
            &[D1.iter().map(|c| c.to_string()).collect()],
            &[],
            &mut UniformEvaluator,
            &config,
            &dir,
            "uniform",
        )
        .unwrap();
        assert_eq!(result.games, 2);
        let m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(m["information"]["regime"], "closed_decklist");
        assert!(m["information"]["prior_hash"].as_str().unwrap().len() == 64);
    });
}

#[test]
fn remembered_own_topdeck_is_pinned_without_revealing_it_to_opponent() {
    with_big_stack(|| {
        let g = new_game_with(55, D1, D1);
        let c = g.players[0].deck.last().unwrap();
        let mut memory = KnowledgeLedger::new(0).unwrap();
        memory
            .observe(
                0,
                c.card_index,
                c.card_id(&g.card_data),
                KnowledgeZone::Deck,
                Some(0),
            )
            .unwrap();
        for seed in 0..8 {
            let w = materialize_closed(&g, 0, &memory, &prior(), seed).unwrap();
            assert_eq!(
                w.players[0].deck.last().unwrap().card_id(&w.card_data),
                c.card_id(&g.card_data)
            );
            digimon_engine::determinize::closed::check_hypothesis(&w).unwrap();
        }
    });
}

#[test]
fn own_membership_cannot_occupy_a_remembered_exact_position() {
    with_big_stack(|| {
        let g = new_game_with(55, D1, D1);
        let mut memory = KnowledgeLedger::new(0).unwrap();
        let top = g.players[0].deck.last().unwrap();
        let member = g.players[0]
            .deck
            .iter()
            .min_by_key(|c| c.card_index)
            .unwrap();
        assert_ne!(top.card_index, member.card_index);
        for (card, pin) in [(member, None), (top, Some(0))] {
            memory
                .observe(
                    0,
                    card.card_index,
                    card.card_id(&g.card_data),
                    KnowledgeZone::Deck,
                    pin,
                )
                .unwrap();
        }
        for seed in 0..128 {
            let w = materialize_closed(&g, 0, &memory, &prior(), seed).unwrap();
            assert_eq!(w.players[0].deck.last().unwrap().card_index, top.card_index);
        }
    });
}

#[test]
fn return_then_recover_preserves_bottom_and_known_top_positions() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, enums::GamePhase};
        let mut g = new_game_with(39, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.start_game();
        g.current_phase = GamePhase::Main;
        let card = g.players[0].hand.pop().unwrap();
        let handle = card.handle();
        g.players[0].trash.push(card);
        let source = g.players[0].hand[0].handle();
        let bottom_position = g.players[0].deck.len();
        let mut ctx = EffectContext::new(&mut g, source, None, 0);
        assert_eq!(
            ctx.return_trash_cards_to_deck_bottom(0, &[handle]),
            vec![handle]
        );
        assert_eq!(ctx.recover_from_deck(0, 1), 1);
        for viewer in 0..2 {
            let positions = g.belief_knowledge(viewer).unwrap().known_positions(0);
            assert!(positions
                .iter()
                .any(|(_, z, p)| *z == KnowledgeZone::Deck && *p == bottom_position - 1));
            materialize_closed(&g, viewer, g.belief_knowledge(viewer).unwrap(), &prior(), 1)
                .unwrap();
        }
        let top = g.players[0].deck.last().unwrap().clone();
        // A public top-deck return makes the next recovered identity known.
        g.players[0].deck.pop();
        g.players[0].trash.push(top.clone());
        let mut ctx = EffectContext::new(&mut g, source, None, 0);
        ctx.return_trash_cards_to_deck_top(0, &[top.handle()]);
        assert_eq!(ctx.recover_from_deck(0, 1), 1);
        for viewer in 0..2 {
            assert!(g
                .belief_knowledge(viewer)
                .unwrap()
                .known_positions(0)
                .contains(&(top.card_id(&g.card_data).into(), KnowledgeZone::Security, 0)));
        }
    });
}

#[test]
fn full_public_trash_counts_exceed_the_current_policy_identity_window() {
    with_big_stack(|| {
        let mut a = new_game_with(90, D1, D1);
        a.start_game();
        a.turn_order = vec![0, 1];
        a.turn_player_idx = 0;
        a.current_phase = digimon_engine::enums::GamePhase::Main;
        let p = &mut a.players[1];
        let mut main = std::mem::take(&mut p.hand);
        main.append(&mut p.deck);
        main.append(&mut p.security);
        p.trash = main.drain(..46).collect();
        p.deck = main;
        let mut b = a.clone();
        let different = b.players[1]
            .deck
            .iter()
            .position(|c| c.card_id(&b.card_data) != b.players[1].trash[45].card_id(&b.card_data))
            .unwrap();
        let p = &mut b.players[1];
        std::mem::swap(&mut p.deck[different], &mut p.trash[45]);
        let registry = digimon_engine::card_registry::CardRegistry::from_cards(card_data());
        let profile = digimon_engine::observation::default_observation_profile();
        let oa = digimon_engine::observation::build_observation_tensor(&a, 0, &registry, profile);
        let ob = digimon_engine::observation::build_observation_tensor(&b, 0, &registry, profile);
        assert_eq!(
            oa, ob,
            "this probe documents the current 45-identity policy limitation"
        );
        let memory = KnowledgeLedger::new(0).unwrap();
        assert_ne!(
            BeliefEvidence::extract(&a, 0, &memory).unwrap().accounted,
            BeliefEvidence::extract(&b, 0, &memory).unwrap().accounted
        );
    });
}

#[test]
fn inspected_security_transfer_retains_entitled_identity_and_remaining_positions() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, enums::GamePhase};
        let mut g = new_game_with(29, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.start_game();
        g.current_phase = GamePhase::Main;
        let source = g.players[0].hand[0].handle();
        EffectContext::new(&mut g, source, None, 0).select_security(
            1,
            "Inspect",
            false,
            |_, _| true,
            |_, _| {},
        );
        let n = g.players[1].security.len();
        g.pending_selection
            .as_mut()
            .unwrap()
            .card_choice
            .as_mut()
            .unwrap()
            .grant_security_inspection(0, &(0..n).collect::<Vec<_>>());
        let card = g.players[1].security[0].clone();
        let id = card.card_id(&g.card_data).to_string();
        // The transfer facade refreshes the live inspection before moving it.
        assert!(g.add_to_hand_from_security(1, card.handle()));
        let memory = g.belief_knowledge(0).unwrap();
        assert!(memory.known_hidden(1).contains(&(id, KnowledgeZone::Hand)));
        assert_eq!(
            memory
                .known_positions(1)
                .iter()
                .filter(|(_, z, _)| *z == KnowledgeZone::Security)
                .count(),
            n - 1
        );
        let restored: KnowledgeLedger =
            serde_json::from_str(&serde_json::to_string(memory).unwrap()).unwrap();
        assert_eq!(restored, *memory);
    });
}

#[test]
fn a_card_absent_from_the_true_list_can_be_legally_played_in_the_hypothesis() {
    with_big_stack(|| {
        let mut g = new_game_with(11, D1, D3);
        g.start_game();
        let mut w =
            materialize_closed(&g, 0, &KnowledgeLedger::new(0).unwrap(), &prior(), 88).unwrap();
        w.mulligan_pending.clear();
        w.current_phase = digimon_engine::enums::GamePhase::Main;
        w.turn_order = vec![1, 0];
        w.turn_player_idx = 0;
        w.memory = 10;
        let mask = digimon_engine::build_action_mask(&w, 1);
        let action = (digimon_engine::action::PLAY_HAND_START
            ..digimon_engine::action::PLAY_HAND_END)
            .find(|a| mask[*a as usize] > 0.5)
            .expect("sampled list has a playable card");
        let card =
            w.players[1].hand[(action - digimon_engine::action::PLAY_HAND_START) as usize].clone();
        assert!(card.card_id(&w.card_data).starts_with("ST1-"));
        assert!(!g.players[1]
            .original_deck
            .iter()
            .any(|r| r.card_id == card.card_id(&w.card_data)));
        w.decode_action(action, 1);
        assert!(w.players[1]
            .hand
            .iter()
            .all(|c| c.handle() != card.handle()));
        digimon_engine::determinize::closed::check_hypothesis(&w).unwrap();
    });
}

#[test]
fn concealed_transforms_fail_explicitly_and_replay_reset_drops_old_knowledge() {
    with_big_stack(|| {
        let mut g = new_game_with(11, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.players[1].hand[0]
            .also_treated_as
            .push("concealed-transform".into());
        assert!(
            materialize_closed(&g, 0, g.belief_knowledge(0).unwrap(), &prior(), 0)
                .err().unwrap()
                .contains("transformed concealed")
        );
        g.reset_for_replay();
        assert!(g.belief_knowledge(0).is_err());
    });
}
