//! CR 4.3 §§4-21-5, 15-1-9, 15-1-10: tokens leave successfully but never
//! inhabit off-field areas or count as actual additions to those areas.
use digimon_engine::{
    action::build_action_mask,
    card_data::CardData,
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    enums::{GamePhase, StackPosition},
    runners::replay::ReplayRunner,
};

fn board(token: bool) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("BODY", "Body"))
        .memory(3)
        .start();
    let h = r.place_on_field(0, if token { "TOKEN_KOTENKEN" } else { "BODY" }, Some(0));
    (r, h)
}

#[test]
fn individual_hand_return_removes_token_but_reports_processed_return() {
    for token in [false, true] {
        let (mut r, h) = board(token);
        let card = r.top_card(h);
        assert_eq!(r.game.return_to_hand(h), Some(card));
        assert!(r.game.players[0].battle_area.is_empty());
        assert_eq!(r.game.players[0].hand.len(), usize::from(!token));
        assert_eq!(r.game.players[0].contains_card(card), !token);
    }
}

#[test]
fn preventing_departure_keeps_the_token_in_play() {
    use digimon_engine::{
        card_source::CardHandle,
        effect::{CardEffect, Effect, EffectBuilder},
        enums::EffectTiming,
    };
    struct PreventLeave;
    impl CardEffect for PreventLeave {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![
                EffectBuilder::new(card, EffectTiming::WhenWouldLeaveBattleArea)
                    .replacement_process(|ctx| ctx.cancel())
                    .build(),
            ]
        }
    }
    for deck in [false, true] {
        let (mut r, h) = board(true);
        r.register_effect("TOKEN_KOTENKEN", std::sync::Arc::new(PreventLeave));
        let card = r.top_card(h);
        if deck {
            assert!(!r.game.return_to_deck(h, StackPosition::Bottom));
        } else {
            assert!(r.game.return_to_hand(h).is_none());
        }
        assert_eq!(r.top_card(h), card);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert!(r.game.players[0].hand.is_empty());
        assert!(r.game.players[0].deck.is_empty());
    }
}

#[test]
fn individual_deck_returns_remove_tokens_for_every_position_and_stack_mode() {
    for position in [
        StackPosition::Top,
        StackPosition::Bottom,
        StackPosition::Random,
    ] {
        for include_sources in [false, true] {
            for token in [false, true] {
                let (mut r, h) = board(token);
                let card = r.top_card(h);
                let moved = if include_sources {
                    r.game.return_stack_to_deck(h, position)
                } else {
                    r.game.return_to_deck(h, position)
                };
                assert!(moved);
                assert!(r.game.players[0].battle_area.is_empty());
                assert_eq!(r.game.players[0].deck.len(), usize::from(!token));
                assert_eq!(r.game.players[0].contains_card(card), !token);
            }
        }
    }
}

#[test]
fn both_security_entry_paths_remove_token_without_security_or_face_up_growth() {
    for self_placement in [false, true] {
        for position in [
            StackPosition::Top,
            StackPosition::Bottom,
            StackPosition::Random,
        ] {
            for token in [false, true] {
                let (mut r, h) = board(token);
                let card = r.top_card(h);
                let mut ctx = EffectContext::new(&mut r.game, card, Some(h), 0);
                if self_placement {
                    ctx.place_self_at_security(position, true);
                } else {
                    ctx.place_permanent_on_security(0, h, position, true);
                }
                assert!(r.game.players[0].battle_area.is_empty());
                assert_eq!(r.game.players[0].security.len(), usize::from(!token));
                assert_eq!(
                    r.game.players[0].face_up_security.len(),
                    usize::from(!token)
                );
                assert_eq!(r.game.players[0].contains_card(card), !token);
            }
        }
    }
}

#[test]
fn malformed_token_hand_never_advertises_a_play() {
    let (mut r, h) = board(true);
    let token = r.game.players[0]
        .battle_area
        .remove(h.index as usize)
        .card_sources
        .pop()
        .unwrap();
    // Deliberate invalid imported state; bypass zone-entry safeguards.
    r.game.players[0].hand.push(token);
    r.set_phase(GamePhase::Main);
    assert_eq!(build_action_mask(&r.game, 0)[0], 0.0);
}

#[test]
fn off_field_zone_entries_discard_tokens_without_corrupting_bulk_offsets() {
    for zone in 0..5 {
        let (mut r, h) = board(true);
        let token = r.game.players[0]
            .battle_area
            .remove(h.index as usize)
            .card_sources
            .pop()
            .unwrap();
        let handle = token.handle();
        let p = &mut r.game.players[0];
        match zone {
            0 => {
                p.put_in_hand(token.clone());
                p.insert_in_hand(0, token);
            }
            1 => {
                p.put_in_trash(token.clone());
                p.insert_in_trash(0, token);
            }
            2 => p.insert_all_in_deck(0, [token.clone(), token]),
            3 => p.insert_in_security(0, token),
            _ => p.insert_all_in_digi_egg_deck(0, [token.clone(), token]),
        }
        assert!(!p.contains_card(handle), "zone {zone}");
    }
}

#[test]
fn bulk_insertion_keeps_real_cards_in_order_around_removed_tokens() {
    for egg_deck in [false, true] {
        let (mut r, h) = board(true);
        let token = r.game.players[0]
            .battle_area
            .remove(h.index as usize)
            .card_sources
            .pop()
            .unwrap();
        let first = r.place_on_field(0, "BODY", Some(0));
        let first = r.game.players[0]
            .battle_area
            .remove(first.index as usize)
            .card_sources
            .pop()
            .unwrap();
        let second = r.place_on_field(0, "BODY", Some(0));
        let second = r.game.players[0]
            .battle_area
            .remove(second.index as usize)
            .card_sources
            .pop()
            .unwrap();
        let expected = vec![first.handle(), second.handle()];
        let cards = [token.clone(), first, token.clone(), second, token];
        if egg_deck {
            r.game.players[0].insert_all_in_digi_egg_deck(0, cards);
        } else {
            r.game.players[0].insert_all_in_deck(0, cards);
        }
        let result = if egg_deck {
            &r.game.players[0].digitama_deck
        } else {
            &r.game.players[0].deck
        };
        assert_eq!(
            result.iter().map(|c| c.handle()).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn removed_token_fires_leave_but_not_hand_add_observers() {
    let yaml = r#"
card: TOKEN-WATCH
name: Token Watch
kind: tamer
color: [blue]
cost: 0
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: victim
          filter: {kind: digimon}
          prompt: Return target
      - return_to_hand: {target: victim}
  - when: on_leave_field
    process:
      - gain_memory: 1
  - when: on_add_to_hand
    process:
      - gain_memory: 2
"#;
    for token in [false, true] {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(yaml)
            .unwrap()
            .add_card(make_test_card("BODY", "Body"))
            .memory(3)
            .start();
        r.place_on_field(0, "TOKEN-WATCH", Some(0));
        r.place_on_field(0, if token { "TOKEN_KOTENKEN" } else { "BODY" }, Some(0));
        r.fire_on_play(0, 0);
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), if token { 4 } else { 6 });
    }
}

fn return_probe(operation: &str) -> DebugRunner {
    let yaml = format!(
        r#"
card: TOKEN-RETURN-PROBE
name: Token Return Probe
kind: digimon
level: 3
color: [blue]
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      {operation}
      - if:
          condition: {{effect_returned_any_card: true}}
          then:
            - gain_memory: 1
      - if:
          condition: {{effect_added_any_card_to_hand: true}}
          then:
            - gain_memory: 2
  - when: on_add_to_deck
    process:
      - gain_memory: 2
"#
    );
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(make_test_card("BODY", "Body"))
        .memory(3)
        .start()
}

#[test]
fn single_and_grouped_returns_count_token_return_without_counting_a_hand_add() {
    for grouped in [false, true] {
        for token in [false, true] {
            let op = if grouped {
                "- return_all_to_hand:\n          over: {all_of: [{of: opponent}, {kind: digimon}]}"
            } else {
                "- select_opponent_permanent:\n          bind_as: victim\n          filter: {kind: digimon}\n          prompt: Return target\n      - return_to_hand: {target: victim}"
            };
            let mut r = return_probe(op);
            r.place_on_field(0, "TOKEN-RETURN-PROBE", Some(0));
            r.place_on_field(1, if token { "TOKEN_KOTENKEN" } else { "BODY" }, Some(0));
            r.fire_on_play(0, 0);
            r.auto_resolve().unwrap();
            assert_eq!(
                r.memory(),
                if token { 4 } else { 6 },
                "grouped={grouped}, token={token}"
            );
            assert_eq!(r.game.players[1].hand.len(), usize::from(!token));
            assert!(r.game.players[1].battle_area.is_empty());
        }
    }
}

#[test]
fn single_and_grouped_deck_returns_count_token_return_without_a_deck_add() {
    for grouped in [false, true] {
        for include_sources in [false, true] {
            for token in [false, true] {
                let operation = if grouped {
                    format!("- return_all_to_deck:\n          over: {{all_of: [{{of: opponent}}, {{kind: digimon}}]}}\n          position: bottom\n          include_sources: {include_sources}")
                } else {
                    format!("- select_opponent_permanent:\n          bind_as: victim\n          filter: {{kind: digimon}}\n          prompt: Return target\n      - return_to_deck: {{target: victim, position: bottom, include_sources: {include_sources}}}")
                };
                let mut r = return_probe(&operation);
                r.place_on_field(0, "TOKEN-RETURN-PROBE", Some(0));
                r.place_on_field(1, if token { "TOKEN_KOTENKEN" } else { "BODY" }, Some(0));
                r.fire_on_play(0, 0);
                r.auto_resolve().unwrap();
                assert_eq!(
                    r.memory(),
                    if token { 4 } else { 6 },
                    "grouped={grouped}, include_sources={include_sources}, token={token}"
                );
                assert_eq!(r.game.players[1].deck.len(), usize::from(!token));
                assert!(r.game.players[1].battle_area.is_empty());
            }
        }
    }
}

#[test]
fn recorded_interrupted_token_return_removes_token_and_clones_identically() {
    let recording: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/training/sw-toho-token-return-hand.json"
    ))
    .unwrap();
    let cards = CardData::load_from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cards.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut replay = ReplayRunner::new(recording.clone(), &cards, true).unwrap();
    // Recorded row 91 resumes MarineBullmon's return after Erlangmon's
    // would-leave play and Sagomon's material selection. Two mulligans are
    // excluded by the native replay adapter, so the pre-step cursor is 88.
    for _ in 0..88 {
        let result = replay.step();
        assert!(result.divergences.is_empty(), "{:?}", result.divergences);
    }
    let mut cloned = replay.game.clone();
    assert_eq!(build_action_mask(&cloned, 0)[62], 1.0);
    cloned.decode_action(62, 0);
    replay.step();
    assert_eq!(
        replay.game.verification_digest(),
        cloned.verification_digest()
    );
    for game in [&replay.game, &cloned] {
        assert!(game.players[0].hand.iter().all(|c| !c.is_token));
        assert!(game.players[0]
            .battle_area
            .iter()
            .all(|p| !p.top_card().is_token));
        assert!(game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&game.card_data) == "EX12-029"));
        assert!(
            game.pending_selection.is_some(),
            "on-play trigger ordering preserved"
        );
    }
    // The old recording's later hand indices describe the corrupt hand, so
    // continue with legal decisions from the fixed state instead of replaying
    // those obsolete indices. Both clones must progress identically to a win.
    for decision in 0..2000 {
        if replay.game.game_over {
            eprintln!("fixed seed 44524569 completed after {decision} continuation decisions");
            break;
        }
        if decision % 100 == 0 {
            eprintln!(
                "fixed seed 44524569: continuation decision {decision}, turn {}",
                replay.game.turn_count
            );
        }
        let player = replay
            .game
            .pending_selection
            .as_ref()
            .map_or_else(|| replay.game.turn_player(), |s| s.selecting_player);
        let mask = build_action_mask(&replay.game, player);
        let action = digimon_engine::policies::greedy::greedy_action(&replay.game, &mask);
        assert!(mask[action as usize] > 0.0);
        let before = replay.game.verification_digest();
        replay.game.decode_action(action, player);
        cloned.decode_action(action, player);
        assert_ne!(
            replay.game.verification_digest(),
            before,
            "action {action} must progress"
        );
        assert_eq!(
            replay.game.verification_digest(),
            cloned.verification_digest()
        );
        for p in &replay.game.players {
            assert!(p
                .hand
                .iter()
                .chain(&p.deck)
                .chain(&p.security)
                .chain(&p.trash)
                .chain(&p.digitama_deck)
                .all(|c| !c.is_token));
        }
    }
    assert!(
        replay.game.game_over,
        "fixed game must finish within the decision bound"
    );
}
