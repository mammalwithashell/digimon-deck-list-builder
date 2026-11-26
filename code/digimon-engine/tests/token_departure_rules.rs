//! CR 4.3 §§4-21-5, 15-1-9, 15-1-10: tokens leave successfully but never
//! inhabit off-field areas or count as actual additions to those areas.
use digimon_engine::{
    action::build_action_mask,
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    enums::{GamePhase, StackPosition},
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
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    // Stages the exact scenario originally captured in sw-toho-token-return-hand.json
    // (whose early trace relied on EX12-057 Takutoumon erroneously parsing `<Guard>`
    // from its `[Paishu] Token` parenthetical before 262228452 fixed
    // `parse_printed_keywords`): MarineBullmon (EX12-031) returns a Token to hand,
    // interrupted by Erlangmon (EX12-034)'s would-leave replacement playing
    // Sagomon (EX12-029) with a one-element DigiXros material selection.
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-031")
        .unwrap()
        .dsl_card("EX12-034")
        .unwrap()
        .dsl_card("EX12-029")
        .unwrap()
        .dsl_card("EX12-043")
        .unwrap()
        .dsl_card("EX12-020")
        .unwrap()
        .add_card(make_test_card("TOKEN_KOTENKEN", "Kotenken Token"))
        .hand(0, &["EX12-029", "EX12-043"])
        .hand(1, &["EX12-020"])
        .deck(0, &["EX12-043"; 20])
        .deck(1, &["EX12-020"; 20])
        .security(0, &["EX12-043"; 5])
        .security(1, &["EX12-020"; 5])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.game.turn_player_idx = 1;
    r.game.enter_main_phase();

    let _erlang = r.place_on_field(0, "EX12-034", Some(0));
    let tok = r.place_on_field(0, "TOKEN_KOTENKEN", Some(0));
    r.game.players[0].battle_area[tok.index as usize]
        .top_card_mut()
        .is_token = true;
    let bull = r.place_on_field(1, "EX12-031", Some(0));

    // Fire MarineBullmon [On Play]: place EX12-020 under it, bounce P0's Token.
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(bull));
    r.game.drain_effect_queue();
    // 1. Place EX12-020 (hand index 0) as MarineBullmon's bottom source.
    r.game.decode_action(0, 1);
    // 2. Target P0's Token (opponent slot 1 -> action 101) for return-to-hand.
    let bounce_mask = build_action_mask(&r.game, 1);
    assert_eq!(bounce_mask[101], 1.0, "Token at slot 1 must be targetable");
    r.game.decode_action(101, 1);
    // 3. Erlangmon interrupts the Token's would-leave: accept replacement (59).
    assert_eq!(
        build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    r.game.decode_action(REPLACEMENT_ACCEPT, 0);
    // 4. Choose "Hand" (action 30).
    assert_eq!(build_action_mask(&r.game, 0)[30], 1.0);
    r.game.decode_action(30, 0);
    // 5. Pick EX12-029 Sagomon (hand index 0) -> opens DigiXros material pick.
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);

    // Clone right at Sagomon's DigiXros material selection and resolve identically.
    let mut cloned = r.game.clone();
    assert_eq!(build_action_mask(&cloned, 0)[30], 1.0);
    r.game.decode_action(30, 0);
    cloned.decode_action(30, 0);
    assert_eq!(r.game.verification_digest(), cloned.verification_digest());
    for game in [&r.game, &cloned] {
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
    // Both clones must progress identically to a win with zero tokens in any
    // non-battle zone.
    for decision in 0..2000 {
        if r.game.game_over {
            eprintln!("interrupted token return completed after {decision} continuation decisions");
            break;
        }
        let player = r
            .game
            .pending_selection
            .as_ref()
            .map_or_else(|| r.game.turn_player(), |s| s.selecting_player);
        let mask = build_action_mask(&r.game, player);
        let action = digimon_engine::policies::greedy::greedy_action(&r.game, &mask);
        assert!(mask[action as usize] > 0.0);
        let before = r.game.verification_digest();
        r.game.decode_action(action, player);
        cloned.decode_action(action, player);
        assert_ne!(
            r.game.verification_digest(),
            before,
            "action {action} must progress"
        );
        assert_eq!(r.game.verification_digest(), cloned.verification_digest());
        for p in &r.game.players {
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
        r.game.game_over,
        "fixed game must finish within the decision bound"
    );
}
